use super::branch_detector::BranchPattern;
use crate::frontend::ast::*;

pub struct BranchlessTransformer;

impl BranchlessTransformer {
    pub fn new() -> Self {
        Self
    }

    pub fn transform(&self, pattern: BranchPattern) -> Vec<Stmt> {
        match pattern {
            BranchPattern::ReLU { var, result } => {
                // result = max(0, var)
                // Esto se puede optimizar luego a instrucciones SIMD max
                vec![Stmt::Assign {
                    name: result,
                    value: Expr::Call {
                        name: "max".to_string(),
                        args: vec![Expr::Number(0), Expr::Variable(var)],
                    },
                }]
            }
            BranchPattern::Select {
                cond,
                true_val,
                false_val,
                target,
            } => {
                // result = if cond then true_val else false_val
                // Se mapea a expresión Ternary que el CodeGen puede convertir a CMOV
                vec![Stmt::Assign {
                    name: target,
                    value: Expr::Ternary {
                        condition: Box::new(cond),
                        then_expr: Box::new(true_val),
                        else_expr: Box::new(false_val),
                    },
                }]
            }
            _ => vec![],
        }
    }
}
