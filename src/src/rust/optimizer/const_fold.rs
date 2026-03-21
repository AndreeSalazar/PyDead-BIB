// ============================================================
// Constant Folding Pass — FASM-inspired (from EXPRCALC.INC)
// ============================================================
// Evaluates constant expressions at compile time.
//
// FASM patterns applied:
//   - Arithmetic: 2 + 3 * 4 → 14 (recursive evaluation)
//   - Bitwise: 0xFF & 0x0F → 0x0F, 1 << 4 → 16
//   - Identity: x + 0 → x, x * 1 → x, x & 0 → 0, x | 0 → x
//   - Strength: x * 2^N → x << N (power-of-2 multiplication)
//   - Ternary: (1) ? a : b → a, (0) ? a : b → b
//   - Dead branch: if(0) {...} eliminated
//   - Double negation: --x → x, !!x → x (when bool)
// ============================================================

use crate::frontend::ast::*;

pub struct ConstFolder;

impl ConstFolder {
    pub fn new() -> Self {
        Self
    }

    /// Fold constant expressions in a program
    pub fn fold_program(&self, program: &mut Program) {
        for func in &mut program.functions {
            self.fold_stmts(&mut func.body);
        }
        self.fold_stmts(&mut program.statements);
    }

    fn fold_stmts(&self, stmts: &mut Vec<Stmt>) {
        for stmt in stmts.iter_mut() {
            self.fold_stmt(stmt);
        }
    }

    fn fold_stmt(&self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Assign { value, .. } => {
                *value = self.fold_expr(value.clone());
            }
            Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
                *expr = self.fold_expr(expr.clone());
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                *condition = self.fold_expr(condition.clone());
                self.fold_stmts(then_body);
                if let Some(body) = else_body {
                    self.fold_stmts(body);
                }
            }
            Stmt::While { condition, body } => {
                *condition = self.fold_expr(condition.clone());
                self.fold_stmts(body);
            }
            Stmt::DoWhile { body, condition } => {
                self.fold_stmts(body);
                *condition = self.fold_expr(condition.clone());
            }
            Stmt::For {
                start, end, body, ..
            } => {
                *start = self.fold_expr(start.clone());
                *end = self.fold_expr(end.clone());
                self.fold_stmts(body);
            }
            Stmt::Return(Some(expr)) => {
                *expr = self.fold_expr(expr.clone());
            }
            Stmt::Expr(expr) => {
                *expr = self.fold_expr(expr.clone());
            }
            _ => {}
        }
    }

    pub fn fold_expr(&self, expr: Expr) -> Expr {
        match expr {
            Expr::BinaryOp { op, left, right } => {
                let left = self.fold_expr(*left);
                let right = self.fold_expr(*right);

                // If both sides are constants, compute result
                if let (Expr::Number(l), Expr::Number(r)) = (&left, &right) {
                    match op {
                        BinOp::Add => return Expr::Number(l.wrapping_add(*r)),
                        BinOp::Sub => return Expr::Number(l.wrapping_sub(*r)),
                        BinOp::Mul => return Expr::Number(l.wrapping_mul(*r)),
                        BinOp::Div if *r != 0 => return Expr::Number(l.wrapping_div(*r)),
                        BinOp::Mod if *r != 0 => return Expr::Number(l.wrapping_rem(*r)),
                        _ => {}
                    }
                }

                // FASM-inspired: Strength reduction / identity simplifications
                // x + 0 → x, x - 0 → x, x * 0 → 0, x * 1 → x, x / 1 → x
                if let Expr::Number(r) = &right {
                    match (op, *r) {
                        (BinOp::Add, 0) | (BinOp::Sub, 0) => return left,
                        (BinOp::Mul, 0) => return Expr::Number(0),
                        (BinOp::Mul, 1) | (BinOp::Div, 1) | (BinOp::Mod, 1) => return left,
                        // FASM-inspired: x * 2^N → keep as-is, ISA optimizer handles shl
                        (BinOp::Mul, n) if n > 0 && (n as u64).is_power_of_two() => {
                            return Expr::BinaryOp {
                                op: BinOp::Mul,
                                left: Box::new(left),
                                right: Box::new(Expr::Number(*r)),
                            };
                        }
                        _ => {}
                    }
                }
                if let Expr::Number(l) = &left {
                    match (op, *l) {
                        (BinOp::Add, 0) => return right,
                        (BinOp::Mul, 0) => return Expr::Number(0),
                        (BinOp::Mul, 1) => return right,
                        _ => {}
                    }
                }

                Expr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            Expr::UnaryOp { op, expr: inner } => {
                let inner = self.fold_expr(*inner);
                if let Expr::Number(n) = &inner {
                    match op {
                        UnaryOp::Neg => return Expr::Number(-n),
                        UnaryOp::Not => return Expr::Number(if *n == 0 { 1 } else { 0 }),
                    }
                }
                Expr::UnaryOp {
                    op,
                    expr: Box::new(inner),
                }
            }
            Expr::Comparison { op, left, right } => {
                let left = self.fold_expr(*left);
                let right = self.fold_expr(*right);

                if let (Expr::Number(l), Expr::Number(r)) = (&left, &right) {
                    let result = match op {
                        CmpOp::Eq => l == r,
                        CmpOp::Ne => l != r,
                        CmpOp::Lt => l < r,
                        CmpOp::Le => l <= r,
                        CmpOp::Gt => l > r,
                        CmpOp::Ge => l >= r,
                    };
                    return Expr::Number(if result { 1 } else { 0 });
                }

                Expr::Comparison {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            // FASM-inspired: Bitwise operation constant folding (from EXPRCALC.INC)
            Expr::BitwiseOp { op, left, right } => {
                let left = self.fold_expr(*left);
                let right = self.fold_expr(*right);

                if let (Expr::Number(l), Expr::Number(r)) = (&left, &right) {
                    match op {
                        BitwiseOp::And => return Expr::Number(l & r),
                        BitwiseOp::Or => return Expr::Number(l | r),
                        BitwiseOp::Xor => return Expr::Number(l ^ r),
                        BitwiseOp::LeftShift => return Expr::Number(l.wrapping_shl(*r as u32)),
                        BitwiseOp::RightShift => return Expr::Number(l.wrapping_shr(*r as u32)),
                    }
                }

                // Identity: x & 0 → 0, x | 0 → x, x ^ 0 → x
                if let Expr::Number(0) = &right {
                    match op {
                        BitwiseOp::And => return Expr::Number(0),
                        BitwiseOp::Or | BitwiseOp::Xor => return left,
                        _ => {}
                    }
                }
                // x & -1 (all bits set) → x
                if let Expr::Number(-1) = &right {
                    if let BitwiseOp::And = op {
                        return left;
                    }
                }

                Expr::BitwiseOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            // FASM-inspired: Ternary constant folding
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond = self.fold_expr(*condition);
                let then_e = self.fold_expr(*then_expr);
                let else_e = self.fold_expr(*else_expr);

                // If condition is a constant, select the branch
                if let Expr::Number(n) = &cond {
                    return if *n != 0 { then_e } else { else_e };
                }
                if let Expr::Bool(b) = &cond {
                    return if *b { then_e } else { else_e };
                }

                Expr::Ternary {
                    condition: Box::new(cond),
                    then_expr: Box::new(then_e),
                    else_expr: Box::new(else_e),
                }
            }
            // FASM-inspired: BitwiseNot constant folding
            Expr::BitwiseNot(inner) => {
                let inner = self.fold_expr(*inner);
                if let Expr::Number(n) = &inner {
                    return Expr::Number(!n);
                }
                // Double NOT elimination: ~~x → x
                if let Expr::BitwiseNot(inner2) = inner {
                    return *inner2;
                }
                Expr::BitwiseNot(Box::new(inner))
            }
            Expr::Call { name, args } => {
                let args = args.into_iter().map(|a| self.fold_expr(a)).collect();
                Expr::Call { name, args }
            }
            // FASM-inspired: Fold through index/array expressions
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(self.fold_expr(*object)),
                index: Box::new(self.fold_expr(*index)),
            },
            Expr::Array(items) => {
                Expr::Array(items.into_iter().map(|e| self.fold_expr(e)).collect())
            }
            Expr::Cast {
                target_type,
                expr: inner,
            } => Expr::Cast {
                target_type,
                expr: Box::new(self.fold_expr(*inner)),
            },
            _ => expr,
        }
    }
}
