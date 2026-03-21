// ============================================================
// Uninitialized Variable Detection
// ============================================================
// Detecta variables usadas antes de ser inicializadas.
// UBKind::UninitializedVariable
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt, Type};
use std::collections::HashSet;

pub fn analyze_uninitialized(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();

    for func in &program.functions {
        let mut analyzer = UninitAnalyzer::new(&func.name);
        for stmt in &func.body {
            analyzer.check_stmt(stmt);
        }
        reports.extend(analyzer.reports);
    }

    let mut analyzer = UninitAnalyzer::new("main");
    for stmt in &program.statements {
        analyzer.check_stmt(stmt);
    }
    reports.extend(analyzer.reports);

    reports
}

struct UninitAnalyzer {
    func_name: String,
    /// Variables que han sido inicializadas
    initialized: HashSet<String>,
    /// Variables declaradas sin valor inicial
    declared_uninit: HashSet<String>,
    current_line: usize,
    reports: Vec<UBReport>,
}

impl UninitAnalyzer {
    fn new(func_name: &str) -> Self {
        Self {
            func_name: func_name.to_string(),
            initialized: HashSet::new(),
            declared_uninit: HashSet::new(),
            current_line: 0,
            reports: Vec::new(),
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::LineMarker(l) => {
                self.current_line = *l;
            }
            // Variable declarada con valor → inicializada
            Stmt::VarDecl {
                name,
                value: Some(val),
                ..
            } => {
                // Temporarily mark as uninit to catch `int b = b;` self-assignment
                self.declared_uninit.insert(name.clone());
                self.check_expr_use(val);
                self.declared_uninit.remove(name);
                
                self.initialized.insert(name.clone());
            }
            // Variable declarada SIN valor → no inicializada
            Stmt::VarDecl {
                name, value: None, var_type, ..
            } => {
                // Arrays and Structs are implicitly given their base addresses
                if matches!(var_type, Type::Array(_, _) | Type::Struct(_)) {
                    self.initialized.insert(name.clone());
                } else {
                    self.declared_uninit.insert(name.clone());
                }
            }
            // Asignacion → marca como inicializada
            Stmt::Assign { name, value } => {
                self.check_expr_use(value);
                self.initialized.insert(name.clone());
                self.declared_uninit.remove(name);
            }
            Stmt::IndexAssign { object, index, value } => {
                self.check_expr_use(index);
                self.check_expr_use(value);
                if let Expr::Variable(name) = object {
                    self.initialized.insert(name.clone());
                    self.declared_uninit.remove(name);
                } else {
                    self.check_expr_use(object);
                }
            }
            Stmt::FieldAssign { object, field: _, value } => {
                self.check_expr_use(value);
                if let Expr::Variable(name) = object {
                    self.initialized.insert(name.clone());
                    self.declared_uninit.remove(name);
                } else {
                    self.check_expr_use(object);
                }
            }
            Stmt::DerefAssign { pointer, value } => {
                self.check_expr_use(pointer);
                self.check_expr_use(value);
            }
            Stmt::ArrowAssign { pointer, value, .. } => {
                self.check_expr_use(pointer);
                self.check_expr_use(value);
            }
            // Return con valor → verificar uso
            Stmt::Return(Some(expr)) => {
                self.check_expr_use(expr);
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.check_expr_use(condition);
                
                let init_before = self.initialized.clone();
                let uninit_before = self.declared_uninit.clone();
                
                for s in then_body {
                    self.check_stmt(s);
                }
                let init_then = self.initialized.clone();
                
                self.initialized = init_before.clone();
                self.declared_uninit = uninit_before.clone();
                
                if let Some(eb) = else_body {
                    for s in eb {
                        self.check_stmt(s);
                    }
                }
                let init_else = self.initialized.clone();
                
                // Merge conservatively: only initialized if initialized in both branches
                let mut new_init = HashSet::new();
                for var in init_then {
                    if init_else.contains(&var) {
                        new_init.insert(var);
                    }
                }
                self.initialized = new_init.clone();
                
                // Keep declared_uninit accurate
                self.declared_uninit = uninit_before;
                for var in &self.initialized {
                    self.declared_uninit.remove(var);
                }
            }
            Stmt::While { condition, body } => {
                self.check_expr_use(condition);
                let init_before = self.initialized.clone();
                let uninit_before = self.declared_uninit.clone();
                for s in body {
                    self.check_stmt(s);
                }
                // Conservatively assume 0 iterations might run
                self.initialized = init_before;
                self.declared_uninit = uninit_before;
            }
            Stmt::Expr(expr) | Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
                self.check_expr_use(expr);
            }
            _ => {}
        }
    }

    fn check_expr_use(&mut self, expr: &Expr) {
        match expr {
            Expr::Variable(name) => {
                if self.declared_uninit.contains(name) && !self.initialized.contains(name) {
                    self.reports.push(
                        UBReport::new(
                            UBSeverity::Error,
                            UBKind::UninitializedVariable,
                            format!("Variable '{}' used before initialization", name),
                        )
                        .with_location(self.func_name.clone(), self.current_line)
                        .with_suggestion(format!("Initialize '{}' before use", name)),
                    );
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_expr_use(left);
                self.check_expr_use(right);
            }
            Expr::Deref(inner) => {
                self.check_expr_use(inner);
            }
            Expr::AddressOf(inner) => {
                if let Expr::Variable(name) = &**inner {
                    self.initialized.insert(name.clone());
                    self.declared_uninit.remove(name);
                } else {
                    self.check_expr_use(inner);
                }
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.check_expr_use(arg);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uninit_detection() {
        let program = Program::new();
        let reports = analyze_uninitialized(&program);
        assert_eq!(reports.len(), 0);
    }
}
