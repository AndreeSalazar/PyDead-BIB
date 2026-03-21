// ============================================================
// Null Pointer Dereference Detection
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt};
use std::collections::HashMap;

pub fn analyze_null_safety(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();

    // Analizar funciones
    for func in &program.functions {
        let mut null_vars = HashMap::new();
        let mut current_line = 0;
        for stmt in &func.body {
            check_stmt_null(
                stmt,
                &func.name,
                &mut reports,
                &mut null_vars,
                &mut current_line,
            );
        }
    }

    // Analizar top-level statements
    let mut null_vars = HashMap::new();
    let mut current_line = 0;
    for stmt in &program.statements {
        check_stmt_null(
            stmt,
            "main",
            &mut reports,
            &mut null_vars,
            &mut current_line,
        );
    }

    reports
}

fn check_stmt_null(
    stmt: &Stmt,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    null_vars: &mut HashMap<String, bool>,
    current_line: &mut usize,
) {
    match stmt {
        Stmt::LineMarker(l) => {
            *current_line = *l;
        }
        Stmt::VarDecl { name, value, .. } => {
            if let Some(val) = value {
                let is_null = is_potentially_null(val, null_vars);
                null_vars.insert(name.clone(), is_null);
            }
        }
        Stmt::Assign { name, value } => {
            let is_null = is_potentially_null(value, null_vars);
            null_vars.insert(name.clone(), is_null);
        }
        Stmt::DerefAssign { pointer, .. } => {
            if is_potentially_null(pointer, null_vars) {
                let severity = if is_definitely_null(pointer, null_vars) {
                    UBSeverity::Error
                } else {
                    UBSeverity::Warning
                };
                reports.push(
                    UBReport::new(
                        severity,
                        UBKind::NullPointerDereference,
                        format!("Dereferencing potentially null pointer"),
                    )
                    .with_location(func_name.to_string(), *current_line)
                    .with_suggestion("Add null check before dereference".to_string()),
                );
            }
        }
        Stmt::ArrowAssign { pointer, .. } => {
            if is_potentially_null(pointer, null_vars) {
                let severity = if is_definitely_null(pointer, null_vars) {
                    UBSeverity::Error
                } else {
                    UBSeverity::Warning
                };
                reports.push(
                    UBReport::new(
                        severity,
                        UBKind::NullPointerDereference,
                        format!("Arrow access on potentially null pointer"),
                    )
                    .with_location(func_name.to_string(), *current_line),
                );
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            check_expr_null(condition, func_name, reports, null_vars, current_line);
            
            let not_null_var_then = get_null_checked_var(condition, true);
            let not_null_var_else = get_null_checked_var(condition, false);
            
            // then branch
            let mut then_vars = null_vars.clone();
            if let Some(ref var) = not_null_var_then {
                then_vars.insert(var.clone(), false);
            }
            for s in then_body {
                check_stmt_null(s, func_name, reports, &mut then_vars, current_line);
            }
            
            // else branch
            let mut else_vars = null_vars.clone();
            if let Some(ref var) = not_null_var_else {
                else_vars.insert(var.clone(), false);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    check_stmt_null(s, func_name, reports, &mut else_vars, current_line);
                }
            }
            
            // Merge states conservatively: if a var is null in either branch, it's null afterwards.
            for (k, v) in then_vars {
                if v || *else_vars.get(&k).unwrap_or(&false) {
                    null_vars.insert(k, true);
                } else {
                    null_vars.insert(k, false);
                }
            }
        }
        Stmt::While { condition, body } => {
            check_expr_null(condition, func_name, reports, null_vars, current_line);
            
            let not_null_var_then = get_null_checked_var(condition, true);
            
            let mut body_vars = null_vars.clone();
            if let Some(ref var) = not_null_var_then {
                body_vars.insert(var.clone(), false);
            }
            for s in body {
                check_stmt_null(s, func_name, reports, &mut body_vars, current_line);
            }
            
            // Merge back conservatively
            for (k, v) in body_vars {
                if v {
                    null_vars.insert(k, true);
                }
            }
        }
        _ => {}
    }
}

fn get_null_checked_var(expr: &Expr, is_true_branch: bool) -> Option<String> {
    if let Expr::Comparison { op, left, right } = expr {
        let is_left_null = is_null_literal(left);
        let is_right_null = is_null_literal(right);
        
        if is_left_null || is_right_null {
            let var_expr = if is_left_null { right } else { left };
            if let Expr::Variable(name) = &**var_expr {
                let checks_not_null = match op {
                    crate::ast::CmpOp::Ne => is_true_branch,
                    crate::ast::CmpOp::Eq => !is_true_branch,
                    _ => false,
                };
                if checks_not_null {
                    return Some(name.clone());
                }
            }
        }
    }
    None
}

fn is_null_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Nullptr | Expr::Number(0) | Expr::Null)
}

fn check_expr_null(
    expr: &Expr,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    null_vars: &HashMap<String, bool>,
    current_line: &mut usize,
) {
    match expr {
        Expr::Deref(inner) => {
            if is_potentially_null(inner, null_vars) {
                let severity = if is_definitely_null(inner, null_vars) {
                    UBSeverity::Error
                } else {
                    UBSeverity::Warning
                };
                reports.push(
                    UBReport::new(
                        severity,
                        UBKind::NullPointerDereference,
                        format!("Dereferencing potentially null expression"),
                    )
                    .with_location(func_name.to_string(), *current_line),
                );
            }
        }
        Expr::ArrowAccess { pointer, .. } => {
            if is_potentially_null(pointer, null_vars) {
                let severity = if is_definitely_null(pointer, null_vars) {
                    UBSeverity::Error
                } else {
                    UBSeverity::Warning
                };
                reports.push(
                    UBReport::new(
                        severity,
                        UBKind::NullPointerDereference,
                        format!("Arrow access on potentially null pointer"),
                    )
                    .with_location(func_name.to_string(), *current_line),
                );
            }
        }
        _ => {}
    }
}

fn is_definitely_null(expr: &Expr, null_vars: &HashMap<String, bool>) -> bool {
    match expr {
        Expr::Nullptr | Expr::Number(0) | Expr::Null => true,
        Expr::Variable(name) => {
            // A variable is definitely null if it was assigned a null literal
            // and never had a null check (tracked as true in null_vars)
            *null_vars.get(name).unwrap_or(&false)
        }
        _ => false,
    }
}

fn is_potentially_null(expr: &Expr, null_vars: &HashMap<String, bool>) -> bool {
    match expr {
        Expr::Nullptr | Expr::Number(0) | Expr::Null => true,
        Expr::Variable(name) => *null_vars.get(name).unwrap_or(&false),
        Expr::Cast { expr: inner, .. } => is_potentially_null(inner, null_vars),
        Expr::Call { name, .. } if name == "malloc" || name == "calloc" || name == "realloc" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_detection() {
        let program = Program::new();
        let reports = analyze_null_safety(&program);
        assert_eq!(reports.len(), 0);
    }
}
