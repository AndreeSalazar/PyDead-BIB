// ============================================================
// Constant Propagation — Variables constantes inlineadas
// ============================================================

use crate::frontend::ast::*;
use std::collections::HashMap;

pub struct ConstPropagator;

impl ConstPropagator {
    pub fn new() -> Self {
        Self
    }

    pub fn propagate(&self, program: &mut Program) {
        for func in &mut program.functions {
            let mut constants: HashMap<String, i64> = HashMap::new();
            func.body = self.propagate_stmts(&func.body, &mut constants);
        }
        let mut constants: HashMap<String, i64> = HashMap::new();
        program.statements = self.propagate_stmts(&program.statements, &mut constants);
    }

    fn propagate_stmts(&self, stmts: &[Stmt], constants: &mut HashMap<String, i64>) -> Vec<Stmt> {
        stmts
            .iter()
            .map(|s| self.propagate_stmt(s, constants))
            .collect()
    }

    fn propagate_stmt(&self, stmt: &Stmt, constants: &mut HashMap<String, i64>) -> Stmt {
        match stmt {
            // int x = 5; → track x=5
            Stmt::VarDecl {
                var_type: _,
                name,
                value: Some(Expr::Number(n)),
            } => {
                constants.insert(name.clone(), *n);
                stmt.clone()
            }
            // x = 5; → track x=5 (if simple assignment)
            Stmt::Assign { name, value } => {
                if let Expr::Number(n) = value {
                    constants.insert(name.clone(), *n);
                } else {
                    constants.remove(name);
                }
                Stmt::Assign {
                    name: name.clone(),
                    value: self.propagate_expr(value, constants),
                }
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => Stmt::If {
                condition: self.propagate_expr(condition, constants),
                then_body: self.propagate_stmts(then_body, &mut constants.clone()),
                else_body: else_body
                    .as_ref()
                    .map(|eb| self.propagate_stmts(eb, &mut constants.clone())),
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: self.propagate_expr(condition, constants),
                body: self.propagate_stmts(body, &mut constants.clone()),
            },
            Stmt::Print(expr) => Stmt::Print(self.propagate_expr(expr, constants)),
            Stmt::Println(expr) => Stmt::Println(self.propagate_expr(expr, constants)),
            Stmt::PrintNum(expr) => Stmt::PrintNum(self.propagate_expr(expr, constants)),
            Stmt::Return(Some(expr)) => Stmt::Return(Some(self.propagate_expr(expr, constants))),
            Stmt::Expr(expr) => Stmt::Expr(self.propagate_expr(expr, constants)),
            _ => stmt.clone(),
        }
    }

    fn propagate_expr(&self, expr: &Expr, constants: &HashMap<String, i64>) -> Expr {
        match expr {
            Expr::Variable(name) => {
                if let Some(&value) = constants.get(name) {
                    Expr::Number(value)
                } else {
                    expr.clone()
                }
            }
            Expr::BinaryOp { op, left, right } => Expr::BinaryOp {
                op: *op,
                left: Box::new(self.propagate_expr(left, constants)),
                right: Box::new(self.propagate_expr(right, constants)),
            },
            Expr::Comparison { op, left, right } => Expr::Comparison {
                op: *op,
                left: Box::new(self.propagate_expr(left, constants)),
                right: Box::new(self.propagate_expr(right, constants)),
            },
            Expr::Call { name, args } => Expr::Call {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| self.propagate_expr(a, constants))
                    .collect(),
            },
            _ => expr.clone(),
        }
    }
}

impl Default for ConstPropagator {
    fn default() -> Self {
        Self::new()
    }
}
