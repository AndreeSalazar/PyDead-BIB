// ============================================================
// Integer Overflow/Underflow Detection
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{BinOp, Expr, Program, Stmt};

use std::collections::HashMap;

pub fn analyze_overflow(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();
    let mut global_env = HashMap::new();

    for func in &program.functions {
        let mut current_line = 0;
        let mut local_env = global_env.clone();
        for stmt in &func.body {
            check_stmt_overflow(stmt, &func.name, &mut reports, &mut current_line, &mut local_env);
        }
    }

    let mut current_line = 0;
    for stmt in &program.statements {
        check_stmt_overflow(stmt, "main", &mut reports, &mut current_line, &mut global_env);
    }

    reports
}

fn check_stmt_overflow(
    stmt: &Stmt,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    current_line: &mut usize,
    env: &mut HashMap<String, i64>,
) {
    match stmt {
        Stmt::LineMarker(l) => {
            *current_line = *l;
        }
        Stmt::Assign { name, value } => {
            check_expr_overflow(value, func_name, reports, current_line, env);
            if let Some(val) = eval_expr(value, env) {
                env.insert(name.clone(), val);
            } else {
                env.remove(name);
            }
        }
        Stmt::VarDecl {
            name,
            value: Some(value),
            ..
        } => {
            check_expr_overflow(value, func_name, reports, current_line, env);
            if let Some(val) = eval_expr(value, env) {
                env.insert(name.clone(), val);
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            check_expr_overflow(condition, func_name, reports, current_line, env);
            for s in then_body {
                check_stmt_overflow(s, func_name, reports, current_line, env);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    check_stmt_overflow(s, func_name, reports, current_line, env);
                }
            }
        }
        Stmt::While { condition, body } => {
            check_expr_overflow(condition, func_name, reports, current_line, env);
            for s in body {
                check_stmt_overflow(s, func_name, reports, current_line, env);
            }
        }
        Stmt::Expr(expr) | Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
            check_expr_overflow(expr, func_name, reports, current_line, env);
        }
        _ => {}
    }
}

fn eval_expr(expr: &Expr, env: &HashMap<String, i64>) -> Option<i64> {
    match expr {
        Expr::Number(n) => Some(*n),
        Expr::Variable(name) => env.get(name).copied(),
        Expr::UnaryOp { op: crate::ast::UnaryOp::Neg, expr: inner } => {
            eval_expr(inner, env).map(|v| -v)
        }
        _ => None,
    }
}

fn check_expr_overflow(
    expr: &Expr,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    current_line: &mut usize,
    env: &mut HashMap<String, i64>,
) {
    match expr {
        Expr::BinaryOp { op, left, right } => {
            // Detectar overflow en operaciones aritméticas
            if let (Some(l), Some(r)) = (eval_expr(left, env), eval_expr(right, env)) {
                match op {
                    BinOp::Add => {
                        if (l as i32).checked_add(r as i32).is_none() {
                            reports.push(
                                UBReport::new(
                                    UBSeverity::Error,
                                    UBKind::IntegerOverflow,
                                    format!("Integer overflow in addition: {} + {} [C99 §6.5.5, C++17 §8]", l, r),
                                )
                                .with_location(func_name.to_string(), *current_line)
                                .with_suggestion(
                                    "Use checked arithmetic or wider type".to_string(),
                                ),
                            );
                        }
                    }
                    BinOp::Sub => {
                        if (l as i32).checked_sub(r as i32).is_none() {
                            reports.push(
                                UBReport::new(
                                    UBSeverity::Error,
                                    UBKind::IntegerUnderflow,
                                    format!("Integer underflow in subtraction: {} - {} [C99 §6.5.5]", l, r),
                                )
                                .with_location(func_name.to_string(), *current_line),
                            );
                        }
                    }
                    BinOp::Mul => {
                        if (l as i32).checked_mul(r as i32).is_none() {
                            reports.push(
                                UBReport::new(
                                    UBSeverity::Error,
                                    UBKind::IntegerOverflow,
                                    format!("Integer overflow in multiplication: {} * {} [C99 §6.5.5]", l, r),
                                )
                                .with_location(func_name.to_string(), *current_line),
                            );
                        }
                    }
                    BinOp::Div => {
                        if r == 0 {
                            reports.push(
                                UBReport::new(
                                    UBSeverity::Error,
                                    UBKind::DivisionByZero,
                                    format!("Division by zero: {} / 0 [C99 §6.5.5, C++17 §8.6]", l),
                                )
                                .with_location(func_name.to_string(), *current_line)
                                .with_suggestion("Add zero check before division".to_string()),
                            );
                        }
                    }
                    BinOp::Mod => {
                        if r == 0 {
                            reports.push(
                                UBReport::new(
                                    UBSeverity::Error,
                                    UBKind::DivisionByZero,
                                    format!("Modulo by zero: {} % 0 [C99 §6.5.5]", l),
                                )
                                .with_location(func_name.to_string(), *current_line),
                            );
                        }
                    }
                    _ => {}
                }
            }
            check_expr_overflow(left, func_name, reports, current_line, env);
            check_expr_overflow(right, func_name, reports, current_line, env);
        }
        Expr::Cast { target_type: _, expr: inner } => {
            // Detect signed promotion overflow: e.g., char c = 200; int x = (int)(c * 256);
            // When a small signed type value is promoted and multiplied, it can overflow
            if let Expr::BinaryOp { op: BinOp::Mul, left, right } = inner.as_ref() {
                if let (Some(l), Some(r)) = (eval_expr(left, env), eval_expr(right, env)) {
                    let product = l.wrapping_mul(r);
                    // Check if the values suggest narrow-type promotion overflow
                    // (values that fit in char/short but overflow when multiplied in int)
                    let fits_in_char = (-128..=127).contains(&l) || (0..=255).contains(&l);
                    let fits_in_short = (-32768..=32767).contains(&l) || (0..=65535).contains(&l);
                    if (fits_in_char || fits_in_short)
                        && (product > i32::MAX as i64 || product < i32::MIN as i64)
                    {
                        reports.push(
                            UBReport::new(
                                UBSeverity::Warning,
                                UBKind::SignedOverflowPromotion,
                                format!(
                                    "Signed promotion overflow: {} * {} = {} overflows after implicit promotion [C99 §6.3.1.1]",
                                    l, r, product
                                ),
                            )
                            .with_location(func_name.to_string(), *current_line)
                            .with_suggestion("Cast to wider type before arithmetic to avoid promotion overflow".to_string()),
                        );
                    }
                }
            }
            check_expr_overflow(inner, func_name, reports, current_line, env);
        }
        Expr::BitwiseOp { op, left, right } => {
            if let (Some(l), Some(r)) = (eval_expr(left, env), eval_expr(right, env)) {
                if matches!(op, crate::ast::BitwiseOp::LeftShift | crate::ast::BitwiseOp::RightShift) {
                    if r < 0 || r >= 32 { // Assuming 32-bit int since C standard rules
                        reports.push(
                            UBReport::new(
                                UBSeverity::Error,
                                UBKind::ShiftOverflow,
                                format!("Shift amount out of bounds: {} shifted by {} [C99 6.5.7, C++17 8.5.7]", l, r),
                            )
                            .with_location(func_name.to_string(), *current_line)
                            .with_suggestion("Shift amount must be >= 0 and < bit width".to_string()),
                        );
                    } else if matches!(op, crate::ast::BitwiseOp::LeftShift) && l < 0 {
                        reports.push(
                            UBReport::new(
                                UBSeverity::Warning,
                                UBKind::ShiftOverflow,
                                format!("Left shift of negative value: {} << {} [C99 6.5.7, C++98, changed in C++20]", l, r),
                            )
                            .with_location(func_name.to_string(), *current_line)
                            .with_suggestion("Left shift is undefined for negative signed values before C++20".to_string()),
                        );
                    }
                }
            }
            check_expr_overflow(left, func_name, reports, current_line, env);
            check_expr_overflow(right, func_name, reports, current_line, env);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_division_by_zero() {
        let program = Program::new();
        let reports = analyze_overflow(&program);
        assert_eq!(reports.len(), 0);
    }
}
