// ============================================================
// Array Bounds Check Detection
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt, Type};
use std::collections::HashMap;

pub fn analyze_bounds(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();

    for func in &program.functions {
        let mut arrays = HashMap::new();
        let mut current_line = 0;
        for stmt in &func.body {
            check_stmt_bounds(
                stmt,
                &func.name,
                &mut reports,
                &mut arrays,
                &mut current_line,
            );
        }
    }

    let mut arrays = HashMap::new();
    let mut current_line = 0;
    for stmt in &program.statements {
        check_stmt_bounds(stmt, "main", &mut reports, &mut arrays, &mut current_line);
    }

    reports
}

fn check_stmt_bounds(
    stmt: &Stmt,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    arrays: &mut HashMap<String, i64>,
    current_line: &mut usize,
) {
    match stmt {
        Stmt::LineMarker(l) => {
            *current_line = *l;
        }
        Stmt::VarDecl { name, var_type, .. } => {
            if let Type::Array(_, Some(size)) = var_type {
                arrays.insert(name.clone(), *size as i64);
            }
        }
        Stmt::IndexAssign { object, index, .. } => {
            if let Some(size) = get_array_size(object, arrays) {
                if let Some(idx) = get_constant_index(index) {
                    if idx < 0 || idx >= size {
                        reports.push(
                            UBReport::new(
                                UBSeverity::Error,
                                UBKind::ArrayOutOfBounds,
                                format!("Array index {} out of bounds [0..{})", idx, size),
                            )
                            .with_location(func_name.to_string(), *current_line)
                            .with_suggestion(format!("Index must be in range [0..{})", size)),
                        );
                    }
                }
            }
        }
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            for s in then_body {
                check_stmt_bounds(s, func_name, reports, arrays, current_line);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    check_stmt_bounds(s, func_name, reports, arrays, current_line);
                }
            }
        }
        Stmt::While { body, .. } => {
            for s in body {
                check_stmt_bounds(s, func_name, reports, arrays, current_line);
            }
        }
        Stmt::Assign { value, .. } => {
            check_expr_bounds(value, func_name, reports, arrays, current_line);
        }
        Stmt::VarDecl { value: Some(val), .. } => {
            check_expr_bounds(val, func_name, reports, arrays, current_line);
        }
        Stmt::Expr(expr) | Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
            check_expr_bounds(expr, func_name, reports, arrays, current_line);
        }
        Stmt::Return(Some(expr)) => {
            check_expr_bounds(expr, func_name, reports, arrays, current_line);
        }
        _ => {}
    }
}

fn check_expr_bounds(
    expr: &Expr,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    arrays: &mut HashMap<String, i64>,
    current_line: &mut usize,
) {
    match expr {
        Expr::Index { object, index } => {
            if let Some(size) = get_array_size(object, arrays) {
                if let Some(idx) = get_constant_index(index) {
                    if idx < 0 || idx >= size {
                        reports.push(
                            UBReport::new(
                                UBSeverity::Error,
                                UBKind::ArrayOutOfBounds,
                                format!("Array index {} out of bounds [0..{}) [C99 §6.5.6, C++17 §8.3.1]", idx, size),
                            )
                            .with_location(func_name.to_string(), *current_line)
                            .with_suggestion(format!("Index must be in range [0..{})", size)),
                        );
                    }
                }
            }
            check_expr_bounds(object, func_name, reports, arrays, current_line);
            check_expr_bounds(index, func_name, reports, arrays, current_line);
        }
        Expr::BinaryOp { op, left, right } => {
            // Detect negative pointer arithmetic: ptr + (-n) or ptr - n
            if matches!(op, crate::ast::BinOp::Add | crate::ast::BinOp::Sub) {
                if let Some(idx) = get_constant_index(right) {
                    if (matches!(op, crate::ast::BinOp::Add) && idx < 0)
                        || (matches!(op, crate::ast::BinOp::Sub) && idx > 0)
                    {
                        reports.push(
                            UBReport::new(
                                UBSeverity::Warning,
                                UBKind::ArrayOutOfBounds,
                                format!(
                                    "Negative pointer arithmetic detected (offset {}), may access out-of-bounds memory",
                                    if matches!(op, crate::ast::BinOp::Sub) { -idx } else { idx }
                                ),
                            )
                            .with_location(func_name.to_string(), *current_line)
                            .with_suggestion("Ensure pointer arithmetic does not go before buffer start".to_string()),
                        );
                    }
                }
            }
            check_expr_bounds(left, func_name, reports, arrays, current_line);
            check_expr_bounds(right, func_name, reports, arrays, current_line);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr_bounds(inner, func_name, reports, arrays, current_line);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                check_expr_bounds(arg, func_name, reports, arrays, current_line);
            }
        }
        _ => {}
    }
}

fn get_array_size(expr: &Expr, arrays: &HashMap<String, i64>) -> Option<i64> {
    if let Expr::Variable(name) = expr {
        return arrays.get(name).copied();
    }
    None
}

fn get_constant_index(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Number(n) => Some(*n),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_detection() {
        let program = Program::new();
        let reports = analyze_bounds(&program);
        assert_eq!(reports.len(), 0);
    }
}
