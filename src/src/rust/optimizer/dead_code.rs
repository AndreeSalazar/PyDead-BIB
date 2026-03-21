// ============================================================
// Dead Code Elimination — Elimina codigo no alcanzable
// ============================================================
// SIN explotar UB — diferencia critica con GCC
// ============================================================

use crate::frontend::ast::*;

pub struct DeadCodeEliminator;

impl DeadCodeEliminator {
    pub fn new() -> Self {
        Self
    }

    pub fn eliminate(&self, program: &mut Program) {
        for func in &mut program.functions {
            func.body = self.eliminate_stmts(&func.body);
        }
        program.statements = self.eliminate_stmts(&program.statements);
    }

    fn eliminate_stmts(&self, stmts: &[Stmt]) -> Vec<Stmt> {
        let mut result = Vec::new();
        for stmt in stmts {
            result.push(self.eliminate_stmt(stmt));
            // Statements after return are unreachable
            if matches!(stmt, Stmt::Return(_)) {
                break;
            }
        }
        result
    }

    fn eliminate_stmt(&self, stmt: &Stmt) -> Stmt {
        match stmt {
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                // if (false) { ... } → eliminate then branch
                if Self::is_always_false(condition) {
                    if let Some(eb) = else_body {
                        return eb.first().cloned().unwrap_or(Stmt::Pass);
                    }
                    return Stmt::Pass;
                }
                // if (true) { ... } → keep only then branch
                if Self::is_always_true(condition) {
                    return then_body.first().cloned().unwrap_or(Stmt::Pass);
                }
                Stmt::If {
                    condition: condition.clone(),
                    then_body: self.eliminate_stmts(then_body),
                    else_body: else_body.as_ref().map(|eb| self.eliminate_stmts(eb)),
                }
            }
            Stmt::While { condition, body } => {
                // while (false) { ... } → eliminate
                if Self::is_always_false(condition) {
                    return Stmt::Pass;
                }
                Stmt::While {
                    condition: condition.clone(),
                    body: self.eliminate_stmts(body),
                }
            }
            Stmt::DoWhile { body, condition } => Stmt::DoWhile {
                body: self.eliminate_stmts(body),
                condition: condition.clone(),
            },
            Stmt::For {
                var,
                start,
                end,
                body,
            } => Stmt::For {
                var: var.clone(),
                start: start.clone(),
                end: end.clone(),
                body: self.eliminate_stmts(body),
            },
            _ => stmt.clone(),
        }
    }

    fn is_always_false(expr: &Expr) -> bool {
        matches!(expr, Expr::Number(0) | Expr::Bool(false))
    }

    fn is_always_true(expr: &Expr) -> bool {
        match expr {
            Expr::Number(n) => *n != 0,
            Expr::Bool(true) => true,
            _ => false,
        }
    }
}

impl Default for DeadCodeEliminator {
    fn default() -> Self {
        Self::new()
    }
}
