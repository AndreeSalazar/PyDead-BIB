// ============================================================
// Lifetime Analysis — Use-After-Free Detection
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt};
use std::collections::{HashMap, HashSet};

pub fn analyze_lifetimes(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();

    for func in &program.functions {
        let mut analyzer = LifetimeAnalyzer::new(&func.name);
        for stmt in &func.body {
            analyzer.check_stmt(stmt);
        }
        reports.extend(analyzer.reports);
    }

    for stmt in &program.statements {
        let mut analyzer = LifetimeAnalyzer::new("main");
        analyzer.check_stmt(stmt);
        reports.extend(analyzer.reports);
    }

    reports
}

struct LifetimeAnalyzer {
    func_name: String,
    freed_vars: HashSet<String>,
    alias_map: HashMap<String, String>,
    current_line: usize,
    reports: Vec<UBReport>,
}

impl LifetimeAnalyzer {
    fn new(func_name: &str) -> Self {
        Self {
            func_name: func_name.to_string(),
            freed_vars: HashSet::new(),
            alias_map: HashMap::new(),
            current_line: 0,
            reports: Vec::new(),
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::LineMarker(l) => {
                self.current_line = *l;
            }
            Stmt::Free(expr) => {
                if let Expr::Variable(name) = expr {
                    if self.is_freed(name) {
                        self.reports.push(
                            UBReport::new(
                                UBSeverity::Error,
                                UBKind::DoubleFree,
                                format!("Double free of variable '{}' [C99 §7.20.3.2, C++98]", name),
                            )
                            .with_location(self.func_name.clone(), self.current_line)
                            .with_suggestion("Do not use pointer after free()".to_string()),
                        );
                    } else {
                        self.mark_freed(name.clone());
                    }
                }
            }
            Stmt::Assign { name, value } => {
                self.check_expr_use(value);
                // Si se reasigna, ya no está freed
                self.freed_vars.remove(name);
                
                if let Expr::Variable(rhs) = value {
                    self.alias_map.insert(name.clone(), rhs.clone());
                    // Heredar freed state
                    if self.is_freed(rhs) {
                        self.freed_vars.insert(name.clone());
                    }
                } else {
                    self.alias_map.remove(name);
                }
            }
            Stmt::DerefAssign { pointer, value } => {
                self.check_expr_use(pointer);
                self.check_expr_use(value);
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.check_expr_use(condition);
                
                let state_before = self.freed_vars.clone();
                let alias_before = self.alias_map.clone();
                
                for s in then_body {
                    self.check_stmt(s);
                }
                
                let state_then = self.freed_vars.clone();
                
                self.freed_vars = state_before.clone();
                self.alias_map = alias_before.clone();
                
                if let Some(eb) = else_body {
                    for s in eb {
                        self.check_stmt(s);
                    }
                }
                
                // Conservador
                for v in state_then {
                    self.freed_vars.insert(v);
                }
            }
            Stmt::While { condition, body } => {
                self.check_expr_use(condition);
                for s in body {
                    self.check_stmt(s);
                }
            }
            Stmt::Expr(expr) | Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
                self.check_expr_use(expr);
                
                // C frontend genera Stmt::Free directo, o puede haber llamadas a free dentro de Expr
                if let Expr::Call { name, args } = expr {
                    if name == "free" && args.len() == 1 {
                        if let Expr::Variable(ptr_name) = &args[0] {
                            if self.is_freed(ptr_name) {
                                self.reports.push(
                                    UBReport::new(
                                        UBSeverity::Error,
                                        UBKind::DoubleFree,
                                        format!("Double free of variable '{}' [C99 §7.20.3.2, C++98]", ptr_name),
                                    )
                                    .with_location(self.func_name.clone(), self.current_line)
                                    .with_suggestion("Remove duplicate free() call".to_string()),
                                );
                            } else {
                                self.mark_freed(ptr_name.clone());
                            }
                        }
                    } else if name.contains("free") || name.contains("libera") || name == "realloc" {
                        if !args.is_empty() {
                            if let Expr::Variable(ptr_name) = &args[0] {
                                self.mark_freed(ptr_name.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn mark_freed(&mut self, name: String) {
        self.freed_vars.insert(name.clone());
        let mut to_mark = Vec::new();
        for (alias, original) in &self.alias_map {
            if original == &name || alias == &name {
                to_mark.push(alias.clone());
                to_mark.push(original.clone());
            }
        }
        for a in to_mark {
            self.freed_vars.insert(a);
        }
    }

    fn is_freed(&self, name: &str) -> bool {
        if self.freed_vars.contains(name) {
            return true;
        }
        if let Some(original) = self.alias_map.get(name) {
            if self.freed_vars.contains(original) {
                return true;
            }
        }
        false
    }

    fn check_expr_use(&mut self, expr: &Expr) {
        match expr {
            Expr::Variable(name) => {
                if self.is_freed(name) {
                    self.reports.push(
                        UBReport::new(
                            UBSeverity::Error,
                            UBKind::UseAfterFree,
                            format!("Use of freed variable '{}' [C99 §7.20.3, C++98]", name),
                        )
                        .with_location(self.func_name.clone(), self.current_line),
                    );
                }
            }
            Expr::Deref(inner) => {
                self.check_expr_use(inner);
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_expr_use(left);
                self.check_expr_use(right);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifetime_analysis() {
        let program = Program::new();
        let reports = analyze_lifetimes(&program);
        assert_eq!(reports.len(), 0);
    }
}
