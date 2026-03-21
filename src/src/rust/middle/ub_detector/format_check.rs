// ============================================================
// Format String Mismatch Detection
// ============================================================
// Detecta printf/scanf con format specifiers que no coinciden
// con los tipos de los argumentos.
// UBKind::FormatStringMismatch
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt, Type};
use std::collections::HashMap;

pub fn analyze_format_strings(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();
    let mut global_env: HashMap<String, Type> = HashMap::new();

    // Collect global variable types
    for stmt in &program.statements {
        if let Stmt::VarDecl { name, var_type, .. } = stmt {
            global_env.insert(name.clone(), var_type.clone());
        }
    }

    for func in &program.functions {
        let mut env = global_env.clone();
        for param in &func.params {
            env.insert(param.name.clone(), param.param_type.clone());
        }
        let mut current_line = 0;
        for stmt in &func.body {
            check_stmt(stmt, &func.name, &mut reports, &mut current_line, &env);
        }
    }

    let mut current_line = 0;
    for stmt in &program.statements {
        check_stmt(stmt, "main", &mut reports, &mut current_line, &global_env);
    }

    reports
}

fn check_stmt(
    stmt: &Stmt,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    current_line: &mut usize,
    env: &HashMap<String, Type>,
) {
    match stmt {
        Stmt::LineMarker(l) => { *current_line = *l; }
        Stmt::VarDecl { value: Some(val), .. } => {
            check_expr_format(val, func_name, reports, current_line, env);
        }
        Stmt::Assign { value, .. } => {
            check_expr_format(value, func_name, reports, current_line, env);
        }
        Stmt::Expr(expr) | Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
            check_expr_format(expr, func_name, reports, current_line, env);
        }
        Stmt::If { condition, then_body, else_body, .. } => {
            check_expr_format(condition, func_name, reports, current_line, env);
            for s in then_body { check_stmt(s, func_name, reports, current_line, env); }
            if let Some(eb) = else_body {
                for s in eb { check_stmt(s, func_name, reports, current_line, env); }
            }
        }
        Stmt::While { condition, body } => {
            check_expr_format(condition, func_name, reports, current_line, env);
            for s in body { check_stmt(s, func_name, reports, current_line, env); }
        }
        Stmt::Return(Some(expr)) => {
            check_expr_format(expr, func_name, reports, current_line, env);
        }
        _ => {}
    }
}

fn check_expr_format(
    expr: &Expr,
    func_name: &str,
    reports: &mut Vec<UBReport>,
    current_line: &mut usize,
    env: &HashMap<String, Type>,
) {
    match expr {
        Expr::Call { name, args } => {
            if is_printf_family(name) && !args.is_empty() {
                if let Expr::String(ref fmt) = args[0] {
                    let specifiers = parse_format_specifiers(fmt);
                    let value_args = &args[1..];

                    // Check argument count
                    if specifiers.len() != value_args.len() {
                        reports.push(
                            UBReport::new(
                                UBSeverity::Error,
                                UBKind::FormatStringMismatch,
                                format!(
                                    "{}(): format string expects {} arguments but {} provided",
                                    name, specifiers.len(), value_args.len()
                                ),
                            )
                            .with_location(func_name.to_string(), *current_line)
                            .with_suggestion("Match format specifiers to argument count".to_string()),
                        );
                    }

                    // Check type mismatches
                    for (i, (spec, arg)) in specifiers.iter().zip(value_args.iter()).enumerate() {
                        if let Some(arg_type) = infer_expr_type(arg, env) {
                            if !is_compatible_format(spec, &arg_type) {
                                reports.push(
                                    UBReport::new(
                                        UBSeverity::Warning,
                                        UBKind::FormatStringMismatch,
                                        format!(
                                            "{}(): argument {} has type '{}' but format specifier is '%{}'",
                                            name, i + 1, arg_type, spec
                                        ),
                                    )
                                    .with_location(func_name.to_string(), *current_line)
                                    .with_suggestion(format!(
                                        "Use '{}' for type '{}'",
                                        suggest_specifier(&arg_type),
                                        arg_type
                                    )),
                                );
                            }
                        }
                    }
                }
            }
            for arg in args { check_expr_format(arg, func_name, reports, current_line, env); }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr_format(left, func_name, reports, current_line, env);
            check_expr_format(right, func_name, reports, current_line, env);
        }
        _ => {}
    }
}

fn is_printf_family(name: &str) -> bool {
    matches!(name, "printf" | "fprintf" | "sprintf" | "snprintf" | "scanf" | "fscanf" | "sscanf")
}

fn parse_format_specifiers(fmt: &str) -> Vec<String> {
    let mut specs = Vec::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            i += 1;
            if i >= chars.len() { break; }
            if chars[i] == '%' { i += 1; continue; } // %% escape

            let mut spec = String::new();
            // Skip flags: -, +, space, 0, #
            while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '0' | '#') {
                i += 1;
            }
            // Skip width
            while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
            // Skip precision
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
            }
            // Length modifier
            while i < chars.len() && matches!(chars[i], 'h' | 'l' | 'L' | 'z' | 'j' | 't') {
                spec.push(chars[i]);
                i += 1;
            }
            // Conversion specifier
            if i < chars.len() {
                spec.push(chars[i]);
                specs.push(spec);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    specs
}

fn infer_expr_type(expr: &Expr, env: &HashMap<String, Type>) -> Option<Type> {
    match expr {
        Expr::Number(_) => Some(Type::I32),
        Expr::Float(_) => Some(Type::F64),
        Expr::String(_) => Some(Type::Pointer(Box::new(Type::I8))),
        Expr::Variable(name) => env.get(name).cloned(),
        Expr::Bool(_) => Some(Type::Bool),
        Expr::Cast { target_type, .. } => Some(target_type.clone()),
        _ => None,
    }
}

fn is_compatible_format(spec: &str, ty: &Type) -> bool {
    let conv = spec.chars().last().unwrap_or(' ');
    match conv {
        'd' | 'i' => matches!(ty, Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::Bool),
        'u' | 'x' | 'X' | 'o' => matches!(ty, Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::I8 | Type::I16 | Type::I32 | Type::I64),
        'f' | 'e' | 'E' | 'g' | 'G' => matches!(ty, Type::F32 | Type::F64),
        's' => matches!(ty, Type::Pointer(_) | Type::Str),
        'c' => matches!(ty, Type::I8 | Type::U8 | Type::I32),
        'p' => matches!(ty, Type::Pointer(_)),
        'n' => matches!(ty, Type::Pointer(_)),
        _ => true, // Unknown specifier — don't warn
    }
}

fn suggest_specifier(ty: &Type) -> String {
    match ty {
        Type::I8 | Type::I16 | Type::I32 => "%d".to_string(),
        Type::I64 => "%lld".to_string(),
        Type::U8 | Type::U16 | Type::U32 => "%u".to_string(),
        Type::U64 => "%llu".to_string(),
        Type::F32 | Type::F64 => "%f".to_string(),
        Type::Pointer(_) => "%p".to_string(),
        Type::Bool => "%d".to_string(),
        Type::Str => "%s".to_string(),
        _ => "%?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_specifier_parsing() {
        let specs = parse_format_specifiers("Hello %s, you are %d years old (%f%%)");
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0], "s");
        assert_eq!(specs[1], "d");
        assert_eq!(specs[2], "f");
    }

    #[test]
    fn test_format_empty() {
        let specs = parse_format_specifiers("Hello world");
        assert_eq!(specs.len(), 0);
    }

    #[test]
    fn test_format_long_specifiers() {
        let specs = parse_format_specifiers("%lld %zu %hhu");
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0], "lld");
        assert_eq!(specs[1], "zu");
        assert_eq!(specs[2], "hhu");
    }

    #[test]
    fn test_clean_program() {
        let program = Program::new();
        let reports = analyze_format_strings(&program);
        assert_eq!(reports.len(), 0);
    }
}
