// ============================================================
// Use-After-Free & Dangling Pointer Detection
// ============================================================
// Extends lifetime.rs with dangling pointer detection.
// Tracks scope boundaries to detect dangling stack pointers.
// UBKind::UseAfterFree, UBKind::DanglingPointer
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};
use crate::ast::{Expr, Program, Stmt};
use std::collections::HashMap;

pub fn analyze_use_after_free(program: &Program) -> Vec<UBReport> {
    let mut reports = Vec::new();

    for func in &program.functions {
        let mut analyzer = UseAfterAnalyzer::new(&func.name);
        for stmt in &func.body {
            analyzer.check_stmt(stmt);
        }
        reports.extend(analyzer.reports);
    }

    for stmt in &program.statements {
        let mut analyzer = UseAfterAnalyzer::new("main");
        analyzer.check_stmt(stmt);
        reports.extend(analyzer.reports);
    }

    reports
}

/// Estado de un puntero rastreado
#[derive(Debug, Clone, PartialEq)]
enum PtrState {
    Valid,
    Freed,
    PointsToStack,
}

struct UseAfterAnalyzer {
    func_name: String,
    /// Estado de punteros: nombre -> estado
    ptr_states: HashMap<String, PtrState>,
    alias_map: HashMap<String, String>,
    /// Nivel de scope actual (para detectar dangling)
    scope_depth: usize,
    /// Variables declaradas en cada scope
    scope_vars: Vec<Vec<String>>,
    current_line: usize,
    reports: Vec<UBReport>,
}

impl UseAfterAnalyzer {
    fn new(func_name: &str) -> Self {
        Self {
            func_name: func_name.to_string(),
            ptr_states: HashMap::new(),
            alias_map: HashMap::new(),
            scope_depth: 0,
            scope_vars: vec![Vec::new()],
            current_line: 0,
            reports: Vec::new(),
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::LineMarker(l) => {
                self.current_line = *l;
            }
            Stmt::Free(expr) => {
                if let Expr::Variable(name) = expr {
                    self.mark_freed(name.clone());
                }
            }
            Stmt::VarDecl { name, .. } => {
                if let Some(vars) = self.scope_vars.last_mut() {
                    vars.push(name.clone());
                }
            }
            Stmt::Assign { name, value } => {
                self.check_expr_use(value);
                // Si se asigna la direccion de una variable local
                if is_address_of_local(value) {
                    self.ptr_states
                        .insert(name.clone(), PtrState::PointsToStack);
                    self.alias_map.remove(name);
                } else if let Expr::Variable(rhs) = value {
                    self.ptr_states.remove(name);
                    self.alias_map.insert(name.clone(), rhs.clone());
                    // Heredar estado si el rhs ya está freed
                    if self.is_freed(rhs) {
                        self.ptr_states.insert(name.clone(), PtrState::Freed);
                    }
                } else {
                    // Si se reasigna a algo más, ya no apunta a memoria free'd
                    self.ptr_states.remove(name);
                    self.alias_map.remove(name);
                }
            }
            Stmt::Expr(expr) | Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
                self.check_expr_use(expr);
                
                // Marcar como freed DESPUÉS de evaluar, sino marca el argumento de free() como Use-After-Free
                if let Expr::Call { name, args } = expr {
                    if name == "free" && args.len() == 1 {
                        if !args.is_empty() {
                            if let Expr::Variable(ptr_name) = &args[0] {
                                self.mark_freed(ptr_name.clone());
                            }
                        }
                    } else {
                        // Si pasamos un puntero a una función (como libera(ptr))
                        // conservadoramente podríamos asumir que puede liberarlo (cross-function)
                        // Para este engine: marcamos parámetros que son punteros si la función se llama "libera" o "free" o similar.
                        // Como es genérico, revisemos si es un puntero siendo pasado como arg único a algo que suena a free/realloc
                        if name.contains("free") || name.contains("libera") || name == "realloc" {
                            if !args.is_empty() {
                                if let Expr::Variable(ptr_name) = &args[0] {
                                    self.mark_freed(ptr_name.clone());
                                }
                            }
                        }
                    }
                }
            }
            Stmt::DerefAssign { pointer, value } => {
                self.check_expr_use(pointer);
                self.check_expr_use(value);
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.check_expr_use(condition);
                
                let state_before = self.ptr_states.clone();
                let alias_before = self.alias_map.clone();
                
                self.enter_scope();
                for s in then_body {
                    self.check_stmt(s);
                }
                self.leave_scope();
                
                let state_then = self.ptr_states.clone();
                
                self.ptr_states = state_before.clone();
                self.alias_map = alias_before.clone();
                
                if let Some(eb) = else_body {
                    self.enter_scope();
                    for s in eb {
                        self.check_stmt(s);
                    }
                    self.leave_scope();
                }
                
                // Conservador: si un puntero se libera en cualquier rama, consideramos que está liberado
                for (k, v) in state_then {
                    if v == PtrState::Freed {
                        self.ptr_states.insert(k, PtrState::Freed);
                    }
                }
            }
            Stmt::While { condition, body } => {
                self.check_expr_use(condition);
                self.enter_scope();
                for s in body {
                    self.check_stmt(s);
                }
                self.leave_scope();
            }
            Stmt::Return(Some(expr)) => {
                // Detect returning address of local variable
                if let Expr::AddressOf(inner) = expr {
                    if let Expr::Variable(var_name) = inner.as_ref() {
                        // Check if the variable is local (declared in any scope of this function)
                        let is_local = self.scope_vars.iter().any(|vars| vars.contains(var_name));
                        if is_local {
                            self.reports.push(
                                UBReport::new(
                                    UBSeverity::Error,
                                    UBKind::ReturnLocalAddress,
                                    format!(
                                        "Returning address of local variable '{}' — pointer will dangle after function returns [C99 §6.2.4]",
                                        var_name
                                    ),
                                )
                                .with_location(self.func_name.clone(), self.current_line)
                                .with_suggestion("Return by value or allocate on the heap with malloc".to_string()),
                            );
                        }
                    }
                }
                self.check_expr_use(expr);
            }
            _ => {}
        }
    }

    fn mark_freed(&mut self, name: String) {
        self.ptr_states.insert(name.clone(), PtrState::Freed);
        
        // Marcar todos los aliases
        let mut to_mark = Vec::new();
        for (alias, original) in &self.alias_map {
            if original == &name || alias == &name {
                to_mark.push(alias.clone());
                to_mark.push(original.clone());
            }
        }
        for a in to_mark {
            self.ptr_states.insert(a, PtrState::Freed);
        }
    }

    fn is_freed(&self, name: &str) -> bool {
        if self.ptr_states.get(name) == Some(&PtrState::Freed) {
            return true;
        }
        // Check alias
        if let Some(original) = self.alias_map.get(name) {
            if self.ptr_states.get(original) == Some(&PtrState::Freed) {
                return true;
            }
        }
        false
    }

    fn enter_scope(&mut self) {
        self.scope_depth += 1;
        self.scope_vars.push(Vec::new());
    }

    fn leave_scope(&mut self) {
        // Al salir del scope, las variables locales mueren
        if let Some(leaving_vars) = self.scope_vars.pop() {
            for var in &leaving_vars {
                // Cualquier puntero que apunte a estas variables es dangling
                let dangling_ptrs: Vec<String> = self
                    .ptr_states
                    .iter()
                    .filter(|(_, state)| **state == PtrState::PointsToStack)
                    .map(|(name, _)| name.clone())
                    .collect();

                for ptr_name in dangling_ptrs {
                    self.reports.push(
                        UBReport::new(
                            UBSeverity::Warning,
                            UBKind::DanglingPointer,
                            format!(
                                "Pointer '{}' may dangle after '{}' leaves scope [C99 §6.2.4, C++17 §6.7.3]",
                                ptr_name, var
                            ),
                        )
                        .with_location(self.func_name.clone(), self.current_line)
                        .with_suggestion(
                            "Do not take address of stack variable that outlives scope".to_string(),
                        ),
                    );
                }
            }
        }
        self.scope_depth = self.scope_depth.saturating_sub(1);
    }

    fn check_expr_use(&mut self, expr: &Expr) {
        match expr {
            Expr::Variable(name) => {
                if self.is_freed(name) {
                    self.reports.push(
                        UBReport::new(
                            UBSeverity::Error,
                            UBKind::UseAfterFree,
                            format!("Use of freed pointer '{}' [C99 §7.20.3, C++98]", name),
                        )
                        .with_location(self.func_name.clone(), self.current_line)
                        .with_suggestion("Do not use pointer after free()".to_string()),
                    );
                }
            }
            Expr::Deref(inner) => {
                self.check_expr_use(inner);
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_expr_use(left);
                self.check_expr_use(right);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.check_expr_use(arg);
                }
            }
            _ => {}
        }
    }
}

fn is_address_of_local(expr: &Expr) -> bool {
    matches!(expr, Expr::AddressOf(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_after_free_detection() {
        let program = Program::new();
        let reports = analyze_use_after_free(&program);
        assert_eq!(reports.len(), 0);
    }
}
