// ============================================================
// Type Confusion & Invalid Cast Detection
// ============================================================
// Detecta casts invalidos y confusion de tipos.
// UBKind::TypeConfusion, UBKind::InvalidCast
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt, Type};
use std::collections::HashMap;

pub fn analyze_type_safety(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();
    let mut global_env = HashMap::new();

    for stmt in &program.statements {
        if let Stmt::VarDecl { var_type, name, .. } = stmt {
            global_env.insert(name.clone(), var_type.clone());
        }
    }

    for func in &program.functions {
        let mut current_line = 0;
        let mut local_env = global_env.clone();
        for param in &func.params {
            local_env.insert(param.name.clone(), param.param_type.clone());
        }
        for stmt in &func.body {
            check_stmt_types(stmt, &func.name, &mut reports, &mut current_line, &mut local_env);
        }
    }

    let mut current_line = 0;
    for stmt in &program.statements {
        check_stmt_types(stmt, "main", &mut reports, &mut current_line, &mut global_env);
    }

    reports
}

fn check_stmt_types(
    stmt: &Stmt,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    current_line: &mut usize,
    env: &mut HashMap<String, Type>,
) {
    match stmt {
        Stmt::LineMarker(l) => {
            *current_line = *l;
        }
        Stmt::Assign { value, .. } => {
            check_expr_types(value, func_name, reports, current_line, env);
        }
        Stmt::VarDecl {
            var_type,
            name,
            value,
            ..
        } => {
            env.insert(name.clone(), var_type.clone());
            if let Some(val) = value {
                check_expr_types(val, func_name, reports, current_line, env);
            }
        }
        Stmt::DerefAssign { pointer, value } => {
            check_expr_types(pointer, func_name, reports, current_line, env);
            check_expr_types(value, func_name, reports, current_line, env);
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            check_expr_types(condition, func_name, reports, current_line, env);
            for s in then_body {
                check_stmt_types(s, func_name, reports, current_line, env);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    check_stmt_types(s, func_name, reports, current_line, env);
                }
            }
        }
        Stmt::While { condition, body } => {
            check_expr_types(condition, func_name, reports, current_line, env);
            for s in body {
                check_stmt_types(s, func_name, reports, current_line, env);
            }
        }
        Stmt::Expr(expr) | Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
            check_expr_types(expr, func_name, reports, current_line, env);
        }
        _ => {}
    }
}

fn check_expr_types(
    expr: &Expr,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    current_line: &mut usize,
    env: &HashMap<String, Type>,
) {
    match expr {
        Expr::Cast {
            target_type,
            expr: inner,
        } => {
            // Detectar casts potencialmente peligrosos
            if is_dangerous_cast(target_type, inner) {
                reports.push(
                    UBReport::new(
                        UBSeverity::Error,
                        UBKind::InvalidCast,
                        format!("Potentially unsafe cast to '{:?}'", target_type),
                    )
                    .with_location(func_name.to_string(), *current_line)
                    .with_suggestion("Verify cast is valid at runtime".to_string()),
                );
            }
            
            // Detección de Strict Aliasing
            if let Type::Pointer(t_target) = target_type {
                let inner_type_opt = match inner.as_ref() {
                    Expr::AddressOf(inner_expr) => match inner_expr.as_ref() {
                        Expr::Variable(name) => env.get(name).cloned(),
                        _ => None,
                    },
                    Expr::Variable(name) => {
                        env.get(name).cloned().and_then(|t| {
                            if let Type::Pointer(inner_ptr) = t {
                                Some(*inner_ptr)
                            } else {
                                None
                            }
                        })
                    }
                    _ => None,
                };
                
                if let Some(t_source) = inner_type_opt {
                    if !is_compatible_for_aliasing(&t_source, t_target) {
                        reports.push(
                            UBReport::new(
                                UBSeverity::Error,
                                UBKind::StrictAliasingViolation,
                                format!("Strict Aliasing Violation: '{:?}' reinterpreted as '{:?}'", t_source, *t_target),
                            )
                            .with_location(func_name.to_string(), *current_line)
                            .with_suggestion("Use memcpy or bitcast for type punning".to_string()),
                        );
                    }
                }
            }
            
            check_expr_types(inner, func_name, reports, current_line, env);
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr_types(left, func_name, reports, current_line, env);
            check_expr_types(right, func_name, reports, current_line, env);
        }
        Expr::Deref(inner) => {
            check_expr_types(inner, func_name, reports, current_line, env);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                check_expr_types(arg, func_name, reports, current_line, env);
            }
        }
        _ => {}
    }
}

fn is_dangerous_cast(target_type: &Type, expr: &Expr) -> bool {
    // Cast de literal numérico a puntero (ej: (int*)42) es UB
    if matches!(target_type, Type::Pointer(_)) {
        if let Expr::Number(n) = expr {
            return *n != 0; // 0 es NULL, seguro
        }
        // Sin más info de tipos, permitimos casts entre variables porque C lo usa para void* y malloc
    }
    false
}

fn is_compatible_for_aliasing(source: &Type, target: &Type) -> bool {
    // Reglas de C/C++: typeid iguales, void*, char*, o unsigned/signed char*
    if source == target {
        return true;
    }
    match target {
        Type::Void | Type::I8 | Type::U8 => return true,
        _ => {}
    }
    match source {
        Type::Void => return true, // un cast de malloc (void*) no viola aliasing
        _ => {}
    }
    
    // Signed/Unsigned de igual tamaño son compatibles
    match (source, target) {
        (Type::I32, Type::U32) | (Type::U32, Type::I32) => true,
        (Type::I64, Type::U64) | (Type::U64, Type::I64) => true,
        (Type::I16, Type::U16) | (Type::U16, Type::I16) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_safety_clean() {
        let program = Program::new();
        let reports = analyze_type_safety(&program);
        assert_eq!(reports.len(), 0);
    }
}
