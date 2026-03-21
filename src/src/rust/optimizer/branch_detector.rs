use crate::frontend::ast::*;

#[derive(Debug, Clone)]
pub enum BranchPattern {
    // if (x > 0) result = x; else result = 0;
    ReLU {
        var: String,
        result: String,
    },

    // if (cond) a else b
    Select {
        cond: Expr,
        true_val: Expr,
        false_val: Expr,
        target: String,
    },

    // if (x < min) x = min; if (x > max) x = max;
    Clamp {
        var: String,
        min: Expr,
        max: Expr,
    },
}

pub struct BranchDetector;

impl BranchDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, stmts: &[Stmt]) -> Vec<BranchPattern> {
        let mut patterns = Vec::new();

        for stmt in stmts {
            // Detectar ReLU: if x > 0 { result = x } else { result = 0 }
            if let Some(pattern) = self.detect_relu(stmt) {
                patterns.push(pattern);
                continue;
            }

            // Detectar Select: if cond { result = a } else { result = b }
            if let Some(pattern) = self.detect_select(stmt) {
                patterns.push(pattern);
                continue;
            }
        }

        patterns
    }

    fn detect_relu(&self, stmt: &Stmt) -> Option<BranchPattern> {
        if let Stmt::If {
            condition,
            then_body,
            else_body,
        } = stmt
        {
            // Check condition: x > 0
            if let Expr::Comparison {
                op: CmpOp::Gt,
                left,
                right,
            } = condition
            {
                if let (Expr::Variable(var_name), Expr::Number(0)) = (left.as_ref(), right.as_ref())
                {
                    // Check then: result = x
                    if then_body.len() == 1 {
                        if let Stmt::Assign {
                            name: target_name,
                            value: Expr::Variable(val_name),
                        } = &then_body[0]
                        {
                            if var_name == val_name {
                                // Check else: result = 0
                                if let Some(else_stmts) = else_body {
                                    if else_stmts.len() == 1 {
                                        if let Stmt::Assign {
                                            name: else_target,
                                            value: Expr::Number(0),
                                        } = &else_stmts[0]
                                        {
                                            if target_name == else_target {
                                                return Some(BranchPattern::ReLU {
                                                    var: var_name.clone(),
                                                    result: target_name.clone(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn detect_select(&self, stmt: &Stmt) -> Option<BranchPattern> {
        if let Stmt::If {
            condition,
            then_body,
            else_body,
        } = stmt
        {
            if then_body.len() == 1 {
                if let Stmt::Assign {
                    name: target_name,
                    value: true_expr,
                } = &then_body[0]
                {
                    if let Some(else_stmts) = else_body {
                        if else_stmts.len() == 1 {
                            if let Stmt::Assign {
                                name: else_target,
                                value: false_expr,
                            } = &else_stmts[0]
                            {
                                if target_name == else_target {
                                    return Some(BranchPattern::Select {
                                        cond: condition.clone(),
                                        true_val: true_expr.clone(),
                                        false_val: false_expr.clone(),
                                        target: target_name.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
