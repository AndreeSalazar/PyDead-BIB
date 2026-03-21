#![allow(dead_code)]
// ============================================================
// DEPRECATED — ADead-BIB Legacy CodeGen v1
// ============================================================
// This file is DEPRECATED. Use isa_compiler.rs instead.
// Pipeline: AST → IsaCompiler → ADeadIR → Encoder → bytes
//
// Kept for reference only. Will be removed in v4.0.
// ============================================================

use crate::frontend::ast::{BinOp, CmpOp, Expr, Stmt, UnaryOp};
use std::collections::HashMap;

pub struct CodeGenerator {
    code: Vec<u8>,
    data: Vec<u8>,
    strings: HashMap<String, usize>, // String -> Offset in data
    string_addresses: Vec<u64>,
    base_address: u64,
    variables: HashMap<String, usize>, // Name -> Stack Offset
    stack_offset: usize,
}

impl CodeGenerator {
    pub fn new(base_address: u64) -> Self {
        Self {
            code: Vec::new(),
            data: Vec::new(),
            strings: HashMap::new(),
            string_addresses: Vec::new(),
            base_address,
            variables: HashMap::new(),
            stack_offset: 0,
        }
    }

    pub fn generate(&mut self, program: &crate::frontend::ast::Program) -> (Vec<u8>, Vec<u8>) {
        // Generar header y setup inicial
        self.emit_prologue();

        for stmt in &program.statements {
            self.emit_statement(stmt);
        }

        self.emit_epilogue();

        (self.code.clone(), self.data.clone())
    }

    fn emit_prologue(&mut self) {
        // push rbp
        // mov rbp, rsp
        self.code.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5]);
    }

    fn emit_epilogue(&mut self) {
        // pop rbp
        // ret
        self.code.extend_from_slice(&[0x5D, 0xC3]);
    }

    fn emit_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Print(expr) => self.emit_print(expr),
            Stmt::PrintNum(expr) => self.emit_print(expr), // Treat as print for now
            Stmt::Assign { name, value } => {
                self.emit_expression(value);
                // mov [rbp - offset], rax
                let offset = self.get_variable_offset(name);
                self.code.push(0x48);
                self.code.push(0x89);
                self.code.push(0x85);
                self.emit_u32(offset as u32); // Use u32 for offset
            }
            Stmt::If {
                condition: _condition,
                then_body: _then_body,
                else_body: _else_body,
            } => {
                // TODO: Implementar control flow
                eprintln!("⚠️  If statement not implemented in legacy codegen");
            }
            Stmt::Expr(expr) => {
                self.emit_expression(expr);
            }
            _ => {
                eprintln!(
                    "⚠️  Statement type not implemented in legacy codegen: {:?}",
                    stmt
                );
            }
        }
    }

    fn emit_print(&mut self, expr: &Expr) {
        match expr {
            Expr::String(s) => {
                eprintln!("⚠️  Print implementación pendiente - usando ret por ahora");
                let _string_idx = self.add_string(s.clone());
            }
            _ => {
                eprintln!("⚠️  Print solo soporta strings por ahora");
            }
        }
    }

    fn emit_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Number(n) => {
                // mov rax, value
                self.code.push(0x48); // REX.W
                self.code.push(0xC7); // MOV
                self.code.push(0xC0); // ModR/M: rax
                self.emit_u32(*n as u32);
            }
            Expr::Float(f) => {
                let bits = f.to_bits();
                self.code.push(0x48);
                self.code.push(0xB8);
                self.emit_u64(bits);
            }
            Expr::String(s) => {
                let string_idx = self.add_string(s.clone());
                let string_addr = self.string_addresses[string_idx];
                self.code.push(0x48);
                self.code.push(0xB8);
                self.emit_u64(string_addr);
            }
            Expr::Bool(b) => {
                self.code.push(0x48);
                self.code.push(0xC7);
                self.code.push(0xC0);
                self.emit_u32(if *b { 1 } else { 0 });
            }
            Expr::Null => {
                self.emit_bytes(&[0x48, 0x31, 0xC0]);
            }
            Expr::Variable(name) => {
                self.emit_variable_load(name);
            }
            Expr::BinaryOp { op, left, right } => {
                self.emit_binary_op(op, left, right);
            }
            Expr::UnaryOp { op, expr } => {
                self.emit_unary_op(op, expr);
            }
            Expr::Comparison { op, left, right } => {
                self.emit_comparison(op, left, right);
            }
            Expr::Call { name, args } => {
                self.emit_function_call(name, args);
            }
            Expr::Array(elements) => {
                self.emit_array_creation(elements);
            }
            Expr::Index { object, index } => {
                self.emit_index_access(object, index);
            }
            Expr::Slice { object, start, end } => {
                self.emit_slice(object, start, end);
            }
            Expr::New { class_name, args } => {
                self.emit_object_creation(class_name, args);
            }
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                self.emit_method_call(object, method, args);
            }
            Expr::FieldAccess { object, field } => {
                self.emit_field_access(object, field);
            }
            Expr::This => {
                self.emit_bytes(&[0x48, 0x89, 0xF8]);
            }
            Expr::Super => {
                self.emit_bytes(&[0x48, 0x8B, 0x7F, 0x08]);
            }
            Expr::Input => {
                // Input no soportado en codegen legacy
                self.emit_bytes(&[0x31, 0xC0]); // xor eax, eax
            }
            Expr::Lambda { params, body } => {
                self.emit_lambda(params, body);
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.emit_ternary(condition, then_expr, else_expr);
            }
            // Built-in functions v1.3.0
            Expr::Len(expr) => {
                self.emit_expression(expr);
                // Por ahora, retorna 0 - implementación pendiente
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
            Expr::Push { array: _, value: _ } => {
                // Push no soportado en codegen legacy
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
            Expr::Pop(_) => {
                // Pop no soportado en codegen legacy
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
            Expr::IntCast(expr) => {
                self.emit_expression(expr);
                // Truncar a entero (ya está en rax)
            }
            Expr::FloatCast(expr) => {
                self.emit_expression(expr);
                // cvtsi2sd xmm0, rax
                self.emit_bytes(&[0xF2, 0x48, 0x0F, 0x2A, 0xC0]);
            }
            Expr::StrCast(_expr) => {
                // Conversión a string no soportada en codegen legacy
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
            Expr::BoolCast(expr) => {
                self.emit_expression(expr);
                // test rax, rax; setne al; movzx rax, al
                self.emit_bytes(&[0x48, 0x85, 0xC0]); // test rax, rax
                self.emit_bytes(&[0x0F, 0x95, 0xC0]); // setne al
                self.emit_bytes(&[0x48, 0x0F, 0xB6, 0xC0]); // movzx rax, al
            }
            Expr::StringConcat { left, right } => {
                // Concatenación no soportada en codegen legacy
                self.emit_expression(left);
                self.emit_expression(right);
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }

            // ========== PUNTEROS Y MEMORIA (v3.2) ==========
            Expr::Deref(ptr) => {
                self.emit_expression(ptr);
                // mov rax, [rax] - dereference pointer
                self.emit_bytes(&[0x48, 0x8B, 0x00]);
            }
            Expr::AddressOf(expr) => {
                // Para variables, cargar dirección en lugar de valor
                if let Expr::Variable(name) = expr.as_ref() {
                    self.emit_variable_address(name);
                } else {
                    self.emit_expression(expr);
                }
            }
            Expr::ArrowAccess { pointer, field: _ } => {
                self.emit_expression(pointer);
                // mov rax, [rax] - dereference then access field
                self.emit_bytes(&[0x48, 0x8B, 0x00]);
            }
            Expr::SizeOf(_) => {
                // Por defecto, sizeof(int) = 8 bytes en x86-64
                self.emit_bytes(&[0x48, 0xC7, 0xC0, 0x08, 0x00, 0x00, 0x00]);
            }
            Expr::Malloc(size) => {
                self.emit_expression(size);
                // Llamar a malloc (pendiente implementación real)
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // placeholder
            }
            Expr::Realloc { ptr, new_size } => {
                self.emit_expression(ptr);
                self.emit_expression(new_size);
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // placeholder
            }
            Expr::Cast {
                target_type: _,
                expr,
            } => {
                self.emit_expression(expr);
                // Cast es no-op en nivel de bytes
            }
            Expr::Nullptr => {
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
            Expr::PreIncrement(expr) | Expr::PostIncrement(expr) => {
                self.emit_expression(expr);
                // inc rax
                self.emit_bytes(&[0x48, 0xFF, 0xC0]);
            }
            Expr::PreDecrement(expr) | Expr::PostDecrement(expr) => {
                self.emit_expression(expr);
                // dec rax
                self.emit_bytes(&[0x48, 0xFF, 0xC8]);
            }
            Expr::BitwiseOp { op, left, right } => {
                self.emit_expression(left);
                self.emit_bytes(&[0x50]); // push rax
                self.emit_expression(right);
                self.emit_bytes(&[0x48, 0x89, 0xC1]); // mov rcx, rax
                self.emit_bytes(&[0x58]); // pop rax
                match op {
                    crate::frontend::ast::BitwiseOp::And => {
                        self.emit_bytes(&[0x48, 0x21, 0xC8]); // and rax, rcx
                    }
                    crate::frontend::ast::BitwiseOp::Or => {
                        self.emit_bytes(&[0x48, 0x09, 0xC8]); // or rax, rcx
                    }
                    crate::frontend::ast::BitwiseOp::Xor => {
                        self.emit_bytes(&[0x48, 0x31, 0xC8]); // xor rax, rcx
                    }
                    crate::frontend::ast::BitwiseOp::LeftShift => {
                        self.emit_bytes(&[0x48, 0xD3, 0xE0]); // shl rax, cl
                    }
                    crate::frontend::ast::BitwiseOp::RightShift => {
                        self.emit_bytes(&[0x48, 0xD3, 0xE8]); // shr rax, cl
                    }
                }
            }
            Expr::BitwiseNot(expr) => {
                self.emit_expression(expr);
                self.emit_bytes(&[0x48, 0xF7, 0xD0]); // not rax
            }
            // OS-Level expressions (v3.1-OS) — stubs in legacy codegen
            Expr::RegRead { .. }
            | Expr::MemRead { .. }
            | Expr::PortIn { .. }
            | Expr::CpuidExpr
            | Expr::LabelAddr { .. } => {
                eprintln!("⚠️  OS-level expression not supported in legacy codegen");
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
        }
    }

    fn emit_variable_address(&mut self, name: &str) {
        // Cargar dirección de variable en rax
        // Por ahora, placeholder
        let _ = name;
        self.emit_bytes(&[0x48, 0x8D, 0x05, 0x00, 0x00, 0x00, 0x00]); // lea rax, [rip+0]
    }

    // Helpers
    fn add_string(&mut self, s: String) -> usize {
        if let Some(&idx) = self.strings.get(&s) {
            return idx;
        }
        let idx = self.string_addresses.len();
        self.strings.insert(s.clone(), idx);

        // Calcular dirección basada en data actual
        let offset = self.data.len() as u64;
        self.string_addresses
            .push(self.base_address + 0x2000 + offset); // 0x2000 es offset arbitrario de data section

        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0); // Null terminator

        idx
    }

    fn get_variable_offset(&mut self, name: &str) -> usize {
        if let Some(&offset) = self.variables.get(name) {
            return offset;
        }
        self.stack_offset += 8;
        self.variables.insert(name.to_string(), self.stack_offset);
        self.stack_offset
    }

    fn emit_u32(&mut self, value: u32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    fn emit_u64(&mut self, value: u64) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    fn emit_bytes(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    // --- Stubs for missing methods ---

    fn emit_variable_load(&mut self, name: &str) {
        // Implementación básica o stub
        eprintln!("⚠️  emit_variable_load not fully implemented in legacy codegen");
        let offset = self.get_variable_offset(name);
        // mov rax, [rbp - offset]
        self.code.push(0x48);
        self.code.push(0x8B);
        self.code.push(0x85);
        self.emit_u32(-(offset as i32) as u32);
    }

    fn emit_binary_op(&mut self, _op: &BinOp, _left: &Expr, _right: &Expr) {
        eprintln!("⚠️  emit_binary_op stub");
    }

    fn emit_unary_op(&mut self, _op: &UnaryOp, _expr: &Expr) {
        eprintln!("⚠️  emit_unary_op stub");
    }

    fn emit_comparison(&mut self, _op: &CmpOp, _left: &Expr, _right: &Expr) {
        eprintln!("⚠️  emit_comparison stub");
    }

    fn emit_function_call(&mut self, _name: &str, _args: &[Expr]) {
        eprintln!("⚠️  emit_function_call stub");
    }

    fn emit_array_creation(&mut self, _elements: &[Expr]) {
        eprintln!("⚠️  emit_array_creation stub");
    }

    fn emit_index_access(&mut self, _object: &Expr, _index: &Expr) {
        eprintln!("⚠️  emit_index_access stub");
    }

    fn emit_slice(&mut self, _object: &Expr, _start: &Option<Box<Expr>>, _end: &Option<Box<Expr>>) {
        eprintln!("⚠️  emit_slice stub");
    }

    fn emit_object_creation(&mut self, _class_name: &str, _args: &[Expr]) {
        eprintln!("⚠️  emit_object_creation stub");
    }

    fn emit_method_call(&mut self, _object: &Expr, _method: &str, _args: &[Expr]) {
        eprintln!("⚠️  emit_method_call stub");
    }

    fn emit_field_access(&mut self, _object: &Expr, _field: &str) {
        eprintln!("⚠️  emit_field_access stub");
    }

    fn emit_lambda(&mut self, _params: &[String], _body: &Box<Expr>) {
        eprintln!("⚠️  emit_lambda stub");
    }

    fn emit_ternary(&mut self, _condition: &Expr, _then_expr: &Expr, _else_expr: &Expr) {
        eprintln!("⚠️  emit_ternary stub");
    }
}
