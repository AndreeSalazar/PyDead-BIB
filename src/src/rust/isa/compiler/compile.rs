// ============================================================
// ISA Compiler — Main compile() and string collection
// ============================================================

use super::core::{CompiledFunction, IsaCompiler};
use crate::frontend::ast::*;
use crate::isa::encoder::Encoder;
use crate::isa::ADeadOp;

impl IsaCompiler {
    /// Compila un programa completo
    pub fn compile(&mut self, program: &Program) -> (Vec<u8>, Vec<u8>, Vec<usize>, Vec<usize>) {
        // Fase 1: Recolectar strings
        self.collect_all_strings(program);
        self.collect_strings_from_stmts(&program.statements);

        // Fase 2: Registrar labels de funciones
        // Support multiple entry point names: main, kernel_main, _start, _main
        let entry_candidates = ["main", "kernel_main", "_start", "_main"];
        let entry_name = entry_candidates.iter()
            .find(|&&name| program.functions.iter().any(|f| f.name == name))
            .map(|&s| s.to_string());
        let has_entry = entry_name.is_some();
        let entry_name_str = entry_name.unwrap_or_else(|| "main".to_string());

        for func in &program.functions {
            let label = self.ir.new_label();
            self.functions.insert(
                func.name.clone(),
                CompiledFunction {
                    name: func.name.clone(),
                    label,
                    params: func.params.iter().map(|p| p.name.clone()).collect(),
                },
            );
        }

        // Fase 3: Si hay entry point, saltar a él
        let entry_label = self.functions.get(&entry_name_str).map(|f| f.label);
        let needs_jmp = has_entry && (program.functions.len() > 1 || !program.statements.is_empty());
        if needs_jmp {
            if let Some(lbl) = entry_label {
                self.ir.emit(ADeadOp::Jmp { target: lbl });
            }
        }

        // Fase 3.5: Dead code elimination — only compile functions reachable from entry
        let reachable = Self::collect_reachable_functions(program, &entry_name_str);

        // Fase 4: Compilar funciones auxiliares (solo las alcanzables)
        for func in &program.functions {
            if func.name != entry_name_str && reachable.contains(&func.name) {
                self.compile_function(func);
            }
        }

        // Fase 5: Compilar top-level statements
        if !has_entry && !program.statements.is_empty() {
            self.compile_top_level(&program.statements);
        }

        // Fase 6: Compilar entry point (ÚLTIMO — para que todas las funciones ya tengan labels)
        for func in &program.functions {
            if func.name == entry_name_str {
                self.compile_function(func);
            }
        }

        // Fase 7: Encode ADeadIR → bytes
        let mut encoder = Encoder::new();
        let result = encoder.encode_all(self.ir.ops());

        // Fase 8: Resolver llamadas por nombre
        let code = result.code;
        for (offset, name) in &result.unresolved_calls {
            if let Some(func) = self.functions.get(name) {
                let _ = (offset, func);
            }
        }

        // Fase 9: Generar sección de datos
        let data = self.generate_data_section();

        (
            code,
            data,
            result.iat_call_offsets,
            result.string_imm64_offsets,
        )
    }

    // ========================================
    // Dead Code Elimination — Reachability Analysis
    // ========================================

    fn collect_reachable_functions(program: &Program, entry_name: &str) -> std::collections::HashSet<String> {
        let mut reachable = std::collections::HashSet::new();
        let mut worklist: Vec<String> = vec![entry_name.to_string()];
        let func_map: std::collections::HashMap<&str, &Function> = program.functions.iter()
            .map(|f| (f.name.as_str(), f)).collect();
        while let Some(name) = worklist.pop() {
            if reachable.contains(&name) { continue; }
            reachable.insert(name.clone());
            if let Some(func) = func_map.get(name.as_str()) {
                let mut called = Vec::new();
                Self::dce_collect_calls_stmts(&func.body, &mut called);
                for callee in called {
                    if !reachable.contains(&callee) && func_map.contains_key(callee.as_str()) {
                        worklist.push(callee);
                    }
                }
            }
        }
        let mut top_calls = Vec::new();
        Self::dce_collect_calls_stmts(&program.statements, &mut top_calls);
        for callee in top_calls {
            if !reachable.contains(&callee) && func_map.contains_key(callee.as_str()) {
                reachable.insert(callee);
            }
        }
        reachable
    }

    fn dce_collect_calls_stmts(stmts: &[Stmt], calls: &mut Vec<String>) {
        for stmt in stmts {
            Self::dce_collect_calls_stmt(stmt, calls);
        }
    }

    fn dce_collect_calls_stmt(stmt: &Stmt, calls: &mut Vec<String>) {
        match stmt {
            Stmt::Expr(e) => Self::dce_collect_calls_expr(e, calls),
            Stmt::VarDecl { value: Some(e), .. } => Self::dce_collect_calls_expr(e, calls),
            Stmt::Assign { value, .. } => Self::dce_collect_calls_expr(value, calls),
            Stmt::Return(Some(e)) => Self::dce_collect_calls_expr(e, calls),
            Stmt::If { condition, then_body, else_body } => {
                Self::dce_collect_calls_expr(condition, calls);
                Self::dce_collect_calls_stmts(then_body, calls);
                if let Some(eb) = else_body { Self::dce_collect_calls_stmts(eb, calls); }
            }
            Stmt::While { condition, body } => {
                Self::dce_collect_calls_expr(condition, calls);
                Self::dce_collect_calls_stmts(body, calls);
            }
            Stmt::For { start, end, body, .. } => {
                Self::dce_collect_calls_expr(start, calls);
                Self::dce_collect_calls_expr(end, calls);
                Self::dce_collect_calls_stmts(body, calls);
            }
            Stmt::DoWhile { body, condition } => {
                Self::dce_collect_calls_stmts(body, calls);
                Self::dce_collect_calls_expr(condition, calls);
            }
            Stmt::CompoundAssign { value, .. } => Self::dce_collect_calls_expr(value, calls),
            Stmt::Print(e) | Stmt::Println(e) | Stmt::PrintNum(e) | Stmt::Free(e) => {
                Self::dce_collect_calls_expr(e, calls);
            }
            _ => {}
        }
    }

    fn dce_collect_calls_expr(expr: &Expr, calls: &mut Vec<String>) {
        match expr {
            Expr::Call { name, args } => {
                calls.push(name.clone());
                for a in args { Self::dce_collect_calls_expr(a, calls); }
            }
            Expr::BinaryOp { left, right, .. } | Expr::Comparison { left, right, .. } => {
                Self::dce_collect_calls_expr(left, calls);
                Self::dce_collect_calls_expr(right, calls);
            }
            Expr::UnaryOp { expr, .. } | Expr::Deref(expr) | Expr::AddressOf(expr)
            | Expr::Cast { expr, .. } | Expr::Malloc(expr) => {
                Self::dce_collect_calls_expr(expr, calls);
            }
            Expr::Index { object, index } => {
                Self::dce_collect_calls_expr(object, calls);
                Self::dce_collect_calls_expr(index, calls);
            }
            Expr::FieldAccess { object, .. } => Self::dce_collect_calls_expr(object, calls),
            Expr::Ternary { condition, then_expr, else_expr } => {
                Self::dce_collect_calls_expr(condition, calls);
                Self::dce_collect_calls_expr(then_expr, calls);
                Self::dce_collect_calls_expr(else_expr, calls);
            }
            Expr::Array(elems) => {
                for e in elems { Self::dce_collect_calls_expr(e, calls); }
            }
            _ => {}
        }
    }

    pub(crate) fn collect_all_strings(&mut self, program: &Program) {
        self.strings.push("%d".to_string());
        self.strings.push("%s".to_string());
        self.strings.push("%.2f".to_string());
        self.strings.push("\n".to_string());

        for func in &program.functions {
            self.collect_strings_from_stmts(&func.body);
        }

        let mut offset = 0u64;
        for s in &self.strings {
            self.string_offsets.insert(s.clone(), offset);
            offset += s.len() as u64 + 1;
        }
    }

    pub(crate) fn collect_strings_from_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::String(s) => {
                let processed = s
                    .replace("\\n", "\n")
                    .replace("\\t", "\t")
                    .replace("\\r", "\r");
                if !self.strings.contains(&processed) {
                    self.strings.push(processed);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.collect_strings_from_expr(left);
                self.collect_strings_from_expr(right);
            }
            Expr::UnaryOp { expr: inner, .. } => {
                self.collect_strings_from_expr(inner);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.collect_strings_from_expr(arg);
                }
            }
            Expr::Comparison { left, right, .. } => {
                self.collect_strings_from_expr(left);
                self.collect_strings_from_expr(right);
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.collect_strings_from_expr(condition);
                self.collect_strings_from_expr(then_expr);
                self.collect_strings_from_expr(else_expr);
            }
            Expr::MethodCall { object, args, .. } => {
                self.collect_strings_from_expr(object);
                for arg in args {
                    self.collect_strings_from_expr(arg);
                }
            }
            Expr::Index { object, index } => {
                self.collect_strings_from_expr(object);
                self.collect_strings_from_expr(index);
            }
            Expr::FieldAccess { object, .. } => {
                self.collect_strings_from_expr(object);
            }
            Expr::Array(elems) => {
                for e in elems {
                    self.collect_strings_from_expr(e);
                }
            }
            Expr::New { args, .. } => {
                for arg in args {
                    self.collect_strings_from_expr(arg);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn collect_strings_from_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Print(expr) | Stmt::Println(expr) | Stmt::PrintNum(expr) => {
                    self.collect_strings_from_expr(expr);
                }
                Stmt::Assign { value, .. } => {
                    self.collect_strings_from_expr(value);
                }
                Stmt::VarDecl { value, .. } => {
                    if let Some(val) = value {
                        self.collect_strings_from_expr(val);
                    }
                }
                Stmt::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    self.collect_strings_from_expr(condition);
                    self.collect_strings_from_stmts(then_body);
                    if let Some(else_stmts) = else_body {
                        self.collect_strings_from_stmts(else_stmts);
                    }
                }
                Stmt::While { condition, body } => {
                    self.collect_strings_from_expr(condition);
                    self.collect_strings_from_stmts(body);
                }
                Stmt::DoWhile { body, condition } => {
                    self.collect_strings_from_stmts(body);
                    self.collect_strings_from_expr(condition);
                }
                Stmt::For {
                    start, end, body, ..
                } => {
                    self.collect_strings_from_expr(start);
                    self.collect_strings_from_expr(end);
                    self.collect_strings_from_stmts(body);
                }
                Stmt::ForEach { iterable, body, .. } => {
                    self.collect_strings_from_expr(iterable);
                    self.collect_strings_from_stmts(body);
                }
                Stmt::Return(Some(expr)) => {
                    self.collect_strings_from_expr(expr);
                }
                Stmt::Expr(expr) => {
                    self.collect_strings_from_expr(expr);
                }
                Stmt::CompoundAssign { value, .. } => {
                    self.collect_strings_from_expr(value);
                }
                Stmt::IndexAssign {
                    object,
                    index,
                    value,
                } => {
                    self.collect_strings_from_expr(object);
                    self.collect_strings_from_expr(index);
                    self.collect_strings_from_expr(value);
                }
                Stmt::FieldAssign { object, value, .. } => {
                    self.collect_strings_from_expr(object);
                    self.collect_strings_from_expr(value);
                }
                _ => {}
            }
        }
    }

    pub(crate) fn generate_data_section(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for s in &self.strings {
            data.extend_from_slice(s.as_bytes());
            data.push(0);
        }
        data
    }
}
