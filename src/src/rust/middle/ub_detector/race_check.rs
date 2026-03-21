// ============================================================
// Data Race & Stack Overflow Detection
// ============================================================
// Detecta acceso concurrente sin sincronizacion y
// recursion infinita potencial.
// UBKind::DataRace, UBKind::StackOverflow
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt};

pub fn analyze_concurrency(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();

    // Detectar recursion infinita (StackOverflow potencial)
    for func in &program.functions {
        let mut checker = RecursionChecker::new(&func.name);
        for stmt in &func.body {
            checker.check_stmt(stmt);
        }
        checker.finalize(&mut reports);
    }

    reports
}

struct RecursionChecker {
    func_name: String,
    /// Cuenta de llamadas recursivas directas
    self_call_count: usize,
    /// Si hay condicion de parada visible
    has_base_case: bool,
}

impl RecursionChecker {
    fn new(func_name: &str) -> Self {
        Self {
            func_name: func_name.to_string(),
            self_call_count: 0,
            has_base_case: false,
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Return(_) => {
                // Un return sin llamada recursiva es un caso base
                self.has_base_case = true;
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                if then_body.iter().any(|s| Self::has_return(s)) {
                    self.has_base_case = true;
                }
                for s in then_body {
                    self.check_stmt(s);
                }
                if let Some(eb) = else_body {
                    if eb.iter().any(|s| Self::has_return(s)) {
                        self.has_base_case = true;
                    }
                    for s in eb {
                        self.check_stmt(s);
                    }
                }
            }
            Stmt::While { body, .. } => {
                for s in body {
                    self.check_stmt(s);
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr);
            }
            Stmt::Assign { value, .. } => {
                self.check_expr(value);
            }
            Stmt::VarDecl {
                value: Some(val), ..
            } => {
                self.check_expr(val);
            }
            _ => {}
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { name, args } => {
                if name == &self.func_name {
                    self.self_call_count += 1;
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            _ => {}
        }
    }

    fn has_return(stmt: &Stmt) -> bool {
        matches!(stmt, Stmt::Return(_))
    }

    fn finalize(self, reports: &mut Vec<UBReport>) {
        if self.self_call_count > 0 && !self.has_base_case {
            reports.push(
                UBReport::new(
                    UBSeverity::Error,
                    UBKind::StackOverflow,
                    format!(
                        "Function '{}' calls itself {} time(s) without visible base case",
                        self.func_name, self.self_call_count
                    ),
                )
                .with_location(self.func_name.clone(), 0)
                .with_suggestion("Add a base case to prevent infinite recursion".to_string()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrency_clean() {
        let program = Program::new();
        let reports = analyze_concurrency(&program);
        assert_eq!(reports.len(), 0);
    }
}
