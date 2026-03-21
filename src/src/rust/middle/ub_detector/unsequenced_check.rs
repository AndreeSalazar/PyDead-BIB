// ============================================================
// Unsequenced Modifications Detection
// ============================================================
// Detecta UB por orden de evaluación no especificado, e.g.:
// i = i++ + 1;
// arr[i] = i++;
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt};
use std::collections::HashMap;

pub fn analyze_unsequenced(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();

    for func in &program.functions {
        let mut current_line = 0;
        for stmt in &func.body {
            check_stmt(stmt, &func.name, &mut reports, &mut current_line);
        }
    }

    let mut current_line = 0;
    for stmt in &program.statements {
        check_stmt(stmt, "main", &mut reports, &mut current_line);
    }

    reports
}

fn check_stmt(
    stmt: &Stmt,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    current_line: &mut usize,
) {
    match stmt {
        Stmt::LineMarker(l) => {
            *current_line = *l;
        }
        Stmt::Assign { name, value } => {
            let mut ws = HashMap::new();
            let mut rs = HashMap::new();
            check_expr(value, &mut ws, &mut rs);
            
            for (var, w) in &ws {
                if *w > 1 || rs.get(var).unwrap_or(&0) > &0 {
                    report_ub(var, func_name, *current_line, reports);
                } else if var == name {
                    report_ub(var, func_name, *current_line, reports);
                }
            }
        }
        Stmt::IndexAssign { object, index, value } => {
            let mut ws = HashMap::new();
            let mut rs = HashMap::new();
            check_expr(object, &mut ws, &mut rs);
            check_expr(index, &mut ws, &mut rs);
            check_expr(value, &mut ws, &mut rs);
            
            for (var, w) in &ws {
                if *w > 1 || rs.get(var).unwrap_or(&0) > &0 {
                    report_ub(var, func_name, *current_line, reports);
                }
            }
        }
        Stmt::FieldAssign { object, value, .. } | Stmt::ArrowAssign { pointer: object, value, .. } => {
            let mut ws = HashMap::new();
            let mut rs = HashMap::new();
            check_expr(object, &mut ws, &mut rs);
            check_expr(value, &mut ws, &mut rs);
            
            for (var, w) in &ws {
                if *w > 1 || rs.get(var).unwrap_or(&0) > &0 {
                    report_ub(var, func_name, *current_line, reports);
                }
            }
        }
        Stmt::DerefAssign { pointer, value } => {
            let mut ws = HashMap::new();
            let mut rs = HashMap::new();
            check_expr(pointer, &mut ws, &mut rs);
            check_expr(value, &mut ws, &mut rs);
            for (var, w) in &ws {
                if *w > 1 || rs.get(var).unwrap_or(&0) > &0 {
                    report_ub(var, func_name, *current_line, reports);
                }
            }
        }
        Stmt::Expr(expr) | Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
            let mut ws = HashMap::new();
            let mut rs = HashMap::new();
            check_expr(expr, &mut ws, &mut rs);
            for (var, w) in &ws {
                if *w > 1 || rs.get(var).unwrap_or(&0) > &0 {
                    report_ub(var, func_name, *current_line, reports);
                }
            }
        }
        Stmt::If { condition, then_body, else_body } => {
            let mut ws = HashMap::new();
            let mut rs = HashMap::new();
            check_expr(condition, &mut ws, &mut rs);
            for (var, w) in &ws {
                if *w > 1 || rs.get(var).unwrap_or(&0) > &0 {
                    report_ub(var, func_name, *current_line, reports);
                }
            }
            for s in then_body {
                check_stmt(s, func_name, reports, current_line);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    check_stmt(s, func_name, reports, current_line);
                }
            }
        }
        Stmt::While { condition, body } => {
            let mut ws = HashMap::new();
            let mut rs = HashMap::new();
            check_expr(condition, &mut ws, &mut rs);
            for (var, w) in &ws {
                if *w > 1 || rs.get(var).unwrap_or(&0) > &0 {
                    report_ub(var, func_name, *current_line, reports);
                }
            }
            for s in body {
                check_stmt(s, func_name, reports, current_line);
            }
        }
        _ => {}
    }
}

fn report_ub(var: &str, func_name: &str, line: usize, reports: &mut Vec<UBReport>) {
    reports.push(
        UBReport::new(
            UBSeverity::Error,
            UBKind::UnsequencedModification,
            format!("Unsequenced modifications / unsequenced read and write of variable '{}'", var),
        )
        .with_location(func_name.to_string(), line)
        .with_suggestion("Separate modifications to different statements with sequence points".to_string())
    );
}

fn check_expr(expr: &Expr, ws: &mut HashMap<String, usize>, rs: &mut HashMap<String, usize>) {
    match expr {
        Expr::PreIncrement(inner) | Expr::PostIncrement(inner) 
        | Expr::PreDecrement(inner) | Expr::PostDecrement(inner) => {
            if let Expr::Variable(name) = inner.as_ref() {
                *ws.entry(name.clone()).or_insert(0) += 1;
            } else {
                check_expr(inner, ws, rs);
                // Also anything it returns is read implicitly if it's not a var, but we only track vars
            }
        }
        Expr::Variable(name) => {
            *rs.entry(name.clone()).or_insert(0) += 1;
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, ws, rs);
            check_expr(right, ws, rs);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, ws, rs);
        }
        Expr::Index { object, index } => {
            check_expr(object, ws, rs);
            check_expr(index, ws, rs);
        }
        Expr::Call { args, .. } => {
            // Evaluacion de argumentos es secuenciada ANTES de la llamada,
            // pero el orden entre ellos NO es secuenciado.
            // Si llamamos f(i++, i) es UB.
            for arg in args {
                check_expr(arg, ws, rs);
            }
        }
        Expr::Cast { expr: inner, .. } => {
            check_expr(inner, ws, rs);
        }
        Expr::Ternary { condition, then_expr, else_expr } => {
            // El operador ternario TIENE un punto de secuencia después de condition.
            // Así que modificaciones en condition no entran en conflicto if se leen en then/else.
            // Para simplificar agresivamente en este linter:
            let mut cond_ws = HashMap::new();
            let mut cond_rs = HashMap::new();
            check_expr(condition, &mut cond_ws, &mut cond_rs);
            // Merge con main
            for (k, v) in cond_ws { *ws.entry(k).or_insert(0) += v; }
            for (k, v) in cond_rs { *rs.entry(k).or_insert(0) += v; }
            
            // then & else are mutually exclusive
            check_expr(then_expr, ws, rs);
            check_expr(else_expr, ws, rs);
        }
        Expr::Deref(inner) | Expr::AddressOf(inner) => {
            check_expr(inner, ws, rs);
        }
        Expr::FieldAccess { object, .. } | Expr::ArrowAccess { pointer: object, .. } => {
            check_expr(object, ws, rs);
        }
        Expr::SizeOf(inner) => {
            if let crate::ast::SizeOfArg::Expr(e) = inner.as_ref() {
                // SizeOf no evalua sus operandos en C (excepto VLA, que no sportamos full)
                // Pero lo analizamos por completitud.
                check_expr(e, ws, rs);
            }
        }
        _ => {}
    }
}
