#![allow(dead_code)]
// ============================================================
// DEPRECATED — ADead-BIB CodeGen v2
// ============================================================
// This file is DEPRECATED. Use isa_compiler.rs instead.
// The ISA-based compiler provides:
//   - Typed instructions (ADeadOp) instead of raw bytes
//   - Optimization at IR level
//   - Multi-mode support (16/32/64-bit)
//
// Kept for reference only. Will be removed in v4.0.
// ============================================================
//
// ADead-BIB CodeGen v2.0 - Sin Límites
// Generación de código mejorada con:
// - Múltiples funciones
// - Stack dinámico
// - Syscalls directos (opcional)
// - Multi-target (Windows/Linux)
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com

use crate::frontend::ast::*;
use std::collections::HashMap;

/// Target de compilación
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    Windows,
    Linux,
    Raw,
}

/// Información de una función compilada
#[derive(Clone, Debug)]
pub struct CompiledFunction {
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub params: Vec<String>,
    pub locals_size: i32,
}

/// CodeGenerator v2 - Sin límites
pub struct CodeGeneratorV2 {
    // Código generado
    code: Vec<u8>,
    data: Vec<u8>,

    // Strings
    strings: Vec<String>,
    string_offsets: HashMap<String, u64>,

    // Funciones
    functions: HashMap<String, CompiledFunction>,
    function_calls: Vec<(usize, String)>, // (offset, nombre) para resolver después

    // Estado actual
    current_function: Option<String>,
    variables: HashMap<String, i32>,
    stack_offset: i32,
    max_stack: i32,

    // Configuración
    target: Target,
    base_address: u64,
    data_rva: u64,
}

impl CodeGeneratorV2 {
    pub fn new(target: Target) -> Self {
        // v1.4.0: data_rva actualizado a 0x2078 (después de printf+scanf en IAT)
        let (base, data_rva) = match target {
            Target::Windows => (0x0000000140000000, 0x2078),
            Target::Linux => (0x400000, 0x1000),
            Target::Raw => (0x0, 0x1000),
        };

        Self {
            code: Vec::new(),
            data: Vec::new(),
            strings: Vec::new(),
            string_offsets: HashMap::new(),
            functions: HashMap::new(),
            function_calls: Vec::new(),
            current_function: None,
            variables: HashMap::new(),
            stack_offset: 0,
            max_stack: 0,
            target,
            base_address: base,
            data_rva,
        }
    }

    /// Genera código para todo el programa
    pub fn generate(&mut self, program: &Program) -> (Vec<u8>, Vec<u8>) {
        // Fase 1: Recolectar strings
        self.collect_all_strings(program);
        // Incluir strings de nivel superior
        self.collect_strings_from_stmts(&program.statements);

        // Fase 2: Emitir salto a main al inicio (se parchea después)
        let has_main = program.functions.iter().any(|f| f.name == "main");
        let jmp_to_main_offset = if has_main && program.functions.len() > 1 {
            self.emit_bytes(&[0xE9]); // jmp rel32
            let offset = self.code.len();
            self.emit_i32(0); // placeholder
            Some(offset)
        } else {
            None
        };

        // Fase 3: Compilar funciones auxiliares primero (no main)
        for func in &program.functions {
            if func.name != "main" {
                self.compile_function(func);
            }
        }

        // Fase 4: Compilar main o nivel superior
        if !program.statements.is_empty() {
            self.compile_top_level(&program.statements);
        }

        // Fase 5: Compilar main si existe
        let main_offset = self.code.len();
        for func in &program.functions {
            if func.name == "main" {
                self.compile_function(func);
            }
        }

        // Parchear salto a main
        if let Some(jmp_offset) = jmp_to_main_offset {
            let rel = (main_offset as i32) - (jmp_offset as i32 + 4);
            self.code[jmp_offset..jmp_offset + 4].copy_from_slice(&rel.to_le_bytes());
        }

        // Fase 6: Resolver llamadas a funciones
        self.resolve_function_calls();

        // Fase 7: Generar sección de datos
        self.generate_data_section();

        (self.code.clone(), self.data.clone())
    }

    /// Recolecta todos los strings del programa
    fn collect_all_strings(&mut self, program: &Program) {
        // Añadir formatos de printf y \n para println
        self.strings.push("%d".to_string());
        self.strings.push("%s".to_string());
        self.strings.push("%.2f".to_string()); // Para flotantes con 2 decimales
        self.strings.push("\n".to_string());

        for func in &program.functions {
            self.collect_strings_from_stmts(&func.body);
        }

        // Calcular offsets de strings
        let mut offset = 0u64;
        for s in &self.strings {
            self.string_offsets.insert(s.clone(), offset);
            offset += s.len() as u64 + 1;
        }
    }

    /// Compila statements de nivel superior como función de entrada
    fn compile_top_level(&mut self, stmts: &[Stmt]) {
        let func_offset = self.code.len();
        self.current_function = Some("__entry".to_string());
        self.variables.clear();
        self.stack_offset = -8;
        self.max_stack = 0;

        // Prologue
        self.emit_bytes(&[0x55]); // push rbp
        self.emit_bytes(&[0x48, 0x89, 0xE5]); // mov rbp, rsp
        self.emit_bytes(&[0x48, 0x81, 0xEC, 0x00, 0x00, 0x00, 0x00]); // sub rsp, imm32
        let stack_size_offset = self.code.len() - 4;

        for stmt in stmts {
            self.emit_statement(stmt);
        }

        // Epilogue
        self.emit_bytes(&[0x31, 0xC0]); // xor eax, eax
        self.emit_bytes(&[0x48, 0x89, 0xEC]); // mov rsp, rbp
        self.emit_bytes(&[0x5D]); // pop rbp
        self.emit_bytes(&[0xC3]); // ret

        let stack_size = ((-self.stack_offset + 15) & !15) as u32; // 16-byte alignment
        self.code[stack_size_offset..stack_size_offset + 4]
            .copy_from_slice(&stack_size.to_le_bytes());

        self.functions.insert(
            "__entry".to_string(),
            CompiledFunction {
                name: "__entry".to_string(),
                offset: func_offset,
                size: self.code.len() - func_offset,
                params: vec![],
                locals_size: -self.stack_offset,
            },
        );

        self.current_function = None;
    }

    fn collect_strings_from_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Print(Expr::String(s)) | Stmt::Println(Expr::String(s)) => {
                    // Procesar secuencias de escape (estilo C++)
                    let processed = s
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\r", "\r");
                    if !self.strings.contains(&processed) {
                        self.strings.push(processed);
                    }
                }
                Stmt::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.collect_strings_from_stmts(then_body);
                    if let Some(else_stmts) = else_body {
                        self.collect_strings_from_stmts(else_stmts);
                    }
                }
                Stmt::While { body, .. } => self.collect_strings_from_stmts(body),
                Stmt::For { body, .. } => self.collect_strings_from_stmts(body),
                Stmt::ForEach { body, .. } => self.collect_strings_from_stmts(body),
                _ => {}
            }
        }
    }

    /// Compila una función
    fn compile_function(&mut self, func: &Function) {
        let func_offset = self.code.len();
        self.current_function = Some(func.name.clone());
        self.variables.clear();
        self.stack_offset = -8;
        self.max_stack = 0;

        // Registrar parámetros
        for (i, param) in func.params.iter().enumerate() {
            // Windows x64 calling convention: rcx, rdx, r8, r9, stack
            let param_offset = match i {
                0..=3 => {
                    // Parámetros en registros, guardar en stack
                    let off = self.stack_offset;
                    self.stack_offset -= 8;
                    off
                }
                _ => {
                    // Parámetros en stack (offset positivo desde rbp)
                    16 + ((i - 4) as i32 * 8)
                }
            };
            self.variables.insert(param.name.clone(), param_offset);
        }

        // Prologue placeholder (se parchea después)
        let _prologue_start = self.code.len();
        self.emit_bytes(&[0x55]); // push rbp
        self.emit_bytes(&[0x48, 0x89, 0xE5]); // mov rbp, rsp
        self.emit_bytes(&[0x48, 0x81, 0xEC, 0x00, 0x00, 0x00, 0x00]); // sub rsp, imm32
        let stack_size_offset = self.code.len() - 4;

        // Guardar parámetros de registros en stack
        for (i, param) in func.params.iter().enumerate().take(4) {
            if let Some(&offset) = self.variables.get(&param.name) {
                match i {
                    0 => self.emit_bytes(&[0x48, 0x89, 0x4D]), // mov [rbp+off], rcx
                    1 => self.emit_bytes(&[0x48, 0x89, 0x55]), // mov [rbp+off], rdx
                    2 => self.emit_bytes(&[0x4C, 0x89, 0x45]), // mov [rbp+off], r8
                    3 => self.emit_bytes(&[0x4C, 0x89, 0x4D]), // mov [rbp+off], r9
                    _ => {}
                }
                self.code.push(offset as u8);
            }
        }

        // Body
        for stmt in &func.body {
            self.emit_statement(stmt);
        }

        // Epilogue implícito si no hay return
        self.emit_bytes(&[0x31, 0xC0]); // xor eax, eax
        self.emit_bytes(&[0x48, 0x89, 0xEC]); // mov rsp, rbp
        self.emit_bytes(&[0x5D]); // pop rbp
        self.emit_bytes(&[0xC3]); // ret

        // Parchear tamaño del stack
        let stack_size = ((-self.stack_offset + 15) & !15) as u32; // Alinear a 16
        self.code[stack_size_offset..stack_size_offset + 4]
            .copy_from_slice(&stack_size.to_le_bytes());

        // Registrar función
        let func_size = self.code.len() - func_offset;
        self.functions.insert(
            func.name.clone(),
            CompiledFunction {
                name: func.name.clone(),
                offset: func_offset,
                size: func_size,
                params: func.params.iter().map(|p| p.name.clone()).collect(),
                locals_size: -self.stack_offset,
            },
        );

        self.current_function = None;
    }

    /// Resuelve llamadas a funciones
    fn resolve_function_calls(&mut self) {
        for (call_offset, func_name) in &self.function_calls {
            if let Some(func) = self.functions.get(func_name) {
                let rel_offset = func.offset as i32 - (*call_offset as i32 + 4);
                self.code[*call_offset..*call_offset + 4]
                    .copy_from_slice(&rel_offset.to_le_bytes());
            } else {
                eprintln!(
                    "⚠️  Warning: Function '{}' not found during linking",
                    func_name
                );
            }
        }
    }

    /// Genera sección de datos
    fn generate_data_section(&mut self) {
        for s in &self.strings.clone() {
            self.data.extend_from_slice(s.as_bytes());
            self.data.push(0);
        }
    }

    /// Obtiene dirección de un string
    fn get_string_address(&self, s: &str) -> u64 {
        if let Some(&offset) = self.string_offsets.get(s) {
            self.base_address + self.data_rva + offset
        } else {
            self.base_address + self.data_rva
        }
    }

    // ========================================
    // Emisión de statements
    // ========================================

    fn emit_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Print(expr) => self.emit_print(expr),
            Stmt::Println(expr) => self.emit_println(expr),
            Stmt::PrintNum(expr) => self.emit_print_num(expr),
            Stmt::Assign { name, value } => self.emit_assign(name, value),
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.emit_if(condition, then_body, else_body.as_deref());
            }
            Stmt::While { condition, body } => self.emit_while(condition, body),
            Stmt::For {
                var,
                start,
                end,
                body,
            } => self.emit_for(var, start, end, body),
            Stmt::ForEach {
                var,
                iterable,
                body,
            } => self.emit_foreach(var, iterable, body),
            Stmt::Return(expr) => self.emit_return(expr.as_ref()),
            Stmt::Expr(expr) => {
                self.emit_expression(expr);
            }
            Stmt::Pass => {}
            _ => {}
        }
    }

    fn emit_print(&mut self, expr: &Expr) {
        if let Expr::String(s) = expr {
            // Procesar secuencias de escape como \n, \t, \r (estilo C++)
            let processed = s
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r");

            // NO añadir \n automáticamente - el usuario lo pone manualmente
            // Asegurar que el string procesado esté en la tabla
            if !self.strings.contains(&processed) {
                self.strings.push(processed.clone());
            }

            let string_addr = self.get_string_address(&processed);

            match self.target {
                Target::Linux => {
                    // sys_write(1, buf, len)
                    self.emit_bytes(&[0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00]); // mov rax, 1
                    self.emit_bytes(&[0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00]); // mov rdi, 1
                    self.emit_bytes(&[0x48, 0xBE]); // mov rsi, addr
                    self.emit_u64(string_addr);
                    self.emit_bytes(&[0x48, 0xC7, 0xC2]); // mov rdx, len
                    self.emit_u32(processed.len() as u32);
                    self.emit_bytes(&[0x0F, 0x05]); // syscall
                }
                Target::Windows | Target::Raw => {
                    // Usar printf via IAT (compatible con codegen actual)
                    self.emit_bytes(&[0x48, 0xB9]);
                    self.emit_u64(string_addr);
                    self.emit_call_printf();
                }
            }
        } else {
            // Evaluar la expresión y determinar el tipo
            self.emit_expression(expr); // RAX = valor

            // Detectar si es flotante
            let is_float = matches!(expr, Expr::Float(_));

            // Detectar si es una expresión numérica entera
            // Incluir Call porque las funciones típicamente retornan enteros
            let is_integer = matches!(
                expr,
                Expr::Number(_)
                    | Expr::Variable(_)
                    | Expr::BinaryOp { .. }
                    | Expr::Bool(_)
                    | Expr::Call { .. }
                    | Expr::IntCast(_)
                    | Expr::Len(_)
            );

            match self.target {
                Target::Windows | Target::Raw => {
                    if is_float {
                        // printf("%.2f", xmm1) para flotantes
                        // Windows x64: flotantes van en XMM1 para printf
                        let fmt_addr = self.get_string_address("%.2f");

                        // mov rdx, rax (bits del double)
                        self.emit_bytes(&[0x48, 0x89, 0xC2]);
                        // movq xmm1, rdx - mover bits a XMM1
                        self.emit_bytes(&[0x66, 0x48, 0x0F, 0x6E, 0xCA]);
                        // mov rcx, fmt_addr
                        self.emit_bytes(&[0x48, 0xB9]);
                        self.emit_u64(fmt_addr);
                        self.emit_call_printf();
                    } else if is_integer {
                        // printf("%d", rax) para números enteros
                        let fmt_addr = self.get_string_address("%d");

                        self.emit_bytes(&[0x48, 0x89, 0xC2]); // mov rdx, rax
                        self.emit_bytes(&[0x48, 0xB9]); // mov rcx, fmt_addr
                        self.emit_u64(fmt_addr);
                        self.emit_call_printf();
                    } else {
                        // printf("%s", rax) para strings
                        let fmt_addr = self.get_string_address("%s");

                        self.emit_bytes(&[0x48, 0x89, 0xC2]); // mov rdx, rax
                        self.emit_bytes(&[0x48, 0xB9]); // mov rcx, fmt_addr
                        self.emit_u64(fmt_addr);
                        self.emit_call_printf();
                    }
                }
                Target::Linux => {
                    // TODO: Implement for Linux
                }
            }
        }
    }

    /// println - igual que print pero añade \n automáticamente
    fn emit_println(&mut self, expr: &Expr) {
        // Primero imprimir la expresión
        self.emit_print(expr);

        // Luego imprimir \n
        let newline = "\n".to_string();
        if !self.strings.contains(&newline) {
            self.strings.push(newline.clone());
        }
        let nl_addr = self.get_string_address("\n");

        match self.target {
            Target::Windows | Target::Raw => {
                self.emit_bytes(&[0x48, 0xB9]);
                self.emit_u64(nl_addr);
                self.emit_call_printf();
            }
            Target::Linux => {}
        }
    }

    fn emit_print_num(&mut self, expr: &Expr) {
        self.emit_expression(expr);

        // Usar %d sin \n - el usuario pone \n manualmente
        let fmt_addr = self.get_string_address("%d");

        self.emit_bytes(&[0x48, 0x89, 0xC2]); // mov rdx, rax
        self.emit_bytes(&[0x48, 0xB9]);
        self.emit_u64(fmt_addr);
        self.emit_call_printf();
    }

    fn emit_call_printf(&mut self) {
        // Windows x64 calling convention
        self.emit_bytes(&[0x48, 0x83, 0xEC, 0x20]); // sub rsp, 32

        // call [rip+offset] - IAT printf at 0x2040 (v1.4.0)
        let call_end_rva = 0x1000 + self.code.len() as u64 + 6;
        let iat_printf_rva = 0x2040u64;
        let offset = iat_printf_rva as i64 - call_end_rva as i64;
        self.emit_bytes(&[0xFF, 0x15]);
        self.emit_i32(offset as i32);

        self.emit_bytes(&[0x48, 0x83, 0xC4, 0x20]); // add rsp, 32
    }

    fn emit_call_scanf(&mut self) {
        // Windows x64 calling convention
        self.emit_bytes(&[0x48, 0x83, 0xEC, 0x20]); // sub rsp, 32

        // call [rip+offset] - IAT scanf at 0x2048 (v1.4.0)
        let call_end_rva = 0x1000 + self.code.len() as u64 + 6;
        let iat_scanf_rva = 0x2048u64;
        let offset = iat_scanf_rva as i64 - call_end_rva as i64;
        self.emit_bytes(&[0xFF, 0x15]);
        self.emit_i32(offset as i32);

        self.emit_bytes(&[0x48, 0x83, 0xC4, 0x20]); // add rsp, 32
    }

    fn emit_assign(&mut self, name: &str, value: &Expr) {
        // Optimización: detectar patrones comunes para generar código más eficiente
        // Patrón: x = x + 1 -> inc [rbp + offset]
        // Patrón: x = x - 1 -> dec [rbp + offset]
        if let Some(&offset) = self.variables.get(name) {
            if let Expr::BinaryOp { op, left, right } = value {
                if let Expr::Variable(var_name) = left.as_ref() {
                    if var_name == name {
                        if let Expr::Number(n) = right.as_ref() {
                            if *n == 1 {
                                match op {
                                    BinOp::Add => {
                                        // x = x + 1 -> inc qword [rbp + offset]
                                        self.emit_bytes(&[0x48, 0xFF, 0x85]);
                                        self.emit_i32(offset);
                                        return;
                                    }
                                    BinOp::Sub => {
                                        // x = x - 1 -> dec qword [rbp + offset]
                                        self.emit_bytes(&[0x48, 0xFF, 0x8D]);
                                        self.emit_i32(offset);
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        // Código normal para otros casos
        self.emit_expression(value);

        let offset = if let Some(&off) = self.variables.get(name) {
            off
        } else {
            let off = self.stack_offset;
            self.variables.insert(name.to_string(), off);
            self.stack_offset -= 8;
            off
        };

        self.emit_bytes(&[0x48, 0x89, 0x85]);
        self.emit_i32(offset);
    }

    fn emit_if(&mut self, condition: &Expr, then_body: &[Stmt], else_body: Option<&[Stmt]>) {
        self.emit_condition(condition);
        self.emit_bytes(&[0x48, 0x85, 0xC0]); // test rax, rax

        self.emit_bytes(&[0x0F, 0x84]);
        let je_offset_pos = self.code.len();
        self.emit_i32(0);

        for stmt in then_body {
            self.emit_statement(stmt);
        }

        if let Some(else_stmts) = else_body {
            self.emit_bytes(&[0xE9]);
            let jmp_offset_pos = self.code.len();
            self.emit_i32(0);

            let else_label = self.code.len();
            let je_offset = (else_label - je_offset_pos - 4) as i32;
            self.code[je_offset_pos..je_offset_pos + 4].copy_from_slice(&je_offset.to_le_bytes());

            for stmt in else_stmts {
                self.emit_statement(stmt);
            }

            let end_label = self.code.len();
            let jmp_offset = (end_label - jmp_offset_pos - 4) as i32;
            self.code[jmp_offset_pos..jmp_offset_pos + 4]
                .copy_from_slice(&jmp_offset.to_le_bytes());
        } else {
            let else_label = self.code.len();
            let je_offset = (else_label - je_offset_pos - 4) as i32;
            self.code[je_offset_pos..je_offset_pos + 4].copy_from_slice(&je_offset.to_le_bytes());
        }
    }

    fn emit_while(&mut self, condition: &Expr, body: &[Stmt]) {
        // OPTIMIZACIÓN BRUTAL: Detectar patrón while counter < CONST { counter += 1 }
        // y usar registros para todo el loop
        if let Expr::Comparison {
            op: CmpOp::Lt,
            left,
            right,
        } = condition
        {
            if let (Expr::Variable(var_name), Expr::Number(limit)) = (left.as_ref(), right.as_ref())
            {
                if let Some(&var_offset) = self.variables.get(var_name) {
                    // Verificar si el body es solo counter += 1
                    let is_simple_increment = body.len() == 1
                        && if let Stmt::Assign { name, value } = &body[0] {
                            name == var_name
                                && if let Expr::BinaryOp {
                                    op: BinOp::Add,
                                    left: l,
                                    right: r,
                                } = value
                                {
                                    if let (Expr::Variable(v), Expr::Number(n)) =
                                        (l.as_ref(), r.as_ref())
                                    {
                                        v == var_name && *n == 1
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                        } else {
                            false
                        };

                    if is_simple_increment {
                        // ============================================================
                        // 🔥 LOOP BRUTAL v5.0 - HEX CRUDO AL METAL 🔥
                        // ============================================================
                        // Técnica: Loop con salto condicional hacia atrás
                        //
                        // Estructura (SOLO 8 bytes en el hot path!):
                        //   .loop:
                        //     inc rcx         ; 48 FF C1 (3 bytes)
                        //     cmp rcx, r8     ; 4C 39 C1 (3 bytes)
                        //     jl .loop        ; 7C F8    (2 bytes) - SHORT JUMP!
                        //
                        // Ventajas:
                        // - Solo 8 bytes por iteración (vs 17 bytes antes)
                        // - Short jump (2 bytes) vs near jump (5 bytes)
                        // - Mejor predicción de branch (backward jump)
                        // - Sin jmp incondicional - el jl hace todo
                        // ============================================================
                        // RCX = counter, R8 = limit

                        // mov rcx, [rbp+offset] (cargar counter inicial)
                        self.emit_bytes(&[0x48, 0x8B, 0x8D]);
                        self.emit_i32(var_offset);

                        // mov r8, limit (cargar límite en registro)
                        self.emit_bytes(&[0x49, 0xB8]);
                        self.emit_u64(*limit as u64);

                        // Verificar si counter >= limit (skip loop si ya terminamos)
                        // cmp rcx, r8
                        self.emit_bytes(&[0x4C, 0x39, 0xC1]);
                        // jge skip (short jump, 2 bytes)
                        self.emit_bytes(&[0x7D]);
                        let skip_pos = self.code.len();
                        self.emit_bytes(&[0x00]); // placeholder

                        // ============ HOT LOOP - 8 BYTES TOTAL! ============
                        let _loop_start = self.code.len(); // Para referencia

                        // inc rcx (3 bytes) - incrementar PRIMERO
                        self.emit_bytes(&[0x48, 0xFF, 0xC1]);

                        // cmp rcx, r8 (3 bytes) - comparar
                        self.emit_bytes(&[0x4C, 0x39, 0xC1]);

                        // jl loop_start (2 bytes) - SHORT JUMP hacia atrás!
                        // Offset = -(tamaño del loop) = -8
                        self.emit_bytes(&[0x7C, 0xF8]); // jl -8

                        // ============ FIN DEL LOOP ============

                        // Parchear skip jump
                        let skip_offset = (self.code.len() - skip_pos - 1) as u8;
                        self.code[skip_pos] = skip_offset;

                        // Guardar resultado de vuelta en memoria
                        // mov [rbp+offset], rcx
                        self.emit_bytes(&[0x48, 0x89, 0x8D]);
                        self.emit_i32(var_offset);

                        return;
                    }

                    // Loop optimizado pero con body genérico
                    // Cargar límite en R8 FUERA del loop
                    self.emit_bytes(&[0x49, 0xB8]); // mov r8, limit
                    self.emit_u64(*limit as u64);

                    let loop_start = self.code.len();

                    // cmp [rbp+offset], r8
                    self.emit_bytes(&[0x4C, 0x39, 0x85]);
                    self.emit_i32(var_offset);

                    // jge loop_end
                    self.emit_bytes(&[0x0F, 0x8D]);
                    let jge_offset_pos = self.code.len();
                    self.emit_i32(0);

                    // Body del loop
                    for stmt in body {
                        self.emit_statement(stmt);
                    }

                    // jmp loop_start
                    self.emit_bytes(&[0xE9]);
                    let jmp_back = (loop_start as i64 - self.code.len() as i64 - 4) as i32;
                    self.emit_i32(jmp_back);

                    // Parchear salto de salida
                    let loop_end = self.code.len();
                    let jge_offset = (loop_end - jge_offset_pos - 4) as i32;
                    self.code[jge_offset_pos..jge_offset_pos + 4]
                        .copy_from_slice(&jge_offset.to_le_bytes());

                    return;
                }
            }
        }

        // Loop genérico para otros casos
        let loop_start = self.code.len();

        self.emit_condition(condition);
        self.emit_bytes(&[0x48, 0x85, 0xC0]);

        self.emit_bytes(&[0x0F, 0x84]);
        let je_offset_pos = self.code.len();
        self.emit_i32(0);

        for stmt in body {
            self.emit_statement(stmt);
        }

        self.emit_bytes(&[0xE9]);
        let jmp_back = (loop_start as i64 - self.code.len() as i64 - 4) as i32;
        self.emit_i32(jmp_back);

        let end_label = self.code.len();
        let je_offset = (end_label - je_offset_pos - 4) as i32;
        self.code[je_offset_pos..je_offset_pos + 4].copy_from_slice(&je_offset.to_le_bytes());
    }

    /// Detecta patrón: while var < CONST { var += 1 }
    fn detect_simple_counter_loop(&self, condition: &Expr, body: &[Stmt]) -> Option<(String, i64)> {
        // Verificar que la condición sea: var < NUMBER
        if let Expr::Comparison {
            op: CmpOp::Lt,
            left,
            right,
        } = condition
        {
            if let Expr::Variable(var_name) = left.as_ref() {
                if let Expr::Number(limit) = right.as_ref() {
                    // Verificar que el body sea solo: var += 1 o var = var + 1
                    if body.len() == 1 {
                        if let Stmt::Assign { name, value } = &body[0] {
                            if name == var_name {
                                // Detectar: var = var + 1 o var += 1 (ambos generan BinaryOp)
                                if let Expr::BinaryOp {
                                    op: BinOp::Add,
                                    left: l,
                                    right: r,
                                } = value
                                {
                                    if let Expr::Variable(v) = l.as_ref() {
                                        if v == var_name {
                                            // Verificar que sea +1
                                            if let Expr::Number(n) = r.as_ref() {
                                                if *n == 1 {
                                                    return Some((var_name.clone(), *limit));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Emite loop de contador ultra-optimizado usando registros
    fn emit_optimized_counter_loop(&mut self, counter_var: &str, limit: i64, _body: &[Stmt]) {
        // Cargar valor inicial del contador en RCX
        if let Some(&offset) = self.variables.get(counter_var) {
            // mov rcx, [rbp + offset]
            self.emit_bytes(&[0x48, 0x8B, 0x8D]);
            self.emit_i32(offset);
        } else {
            // xor ecx, ecx (counter = 0, más eficiente)
            self.emit_bytes(&[0x31, 0xC9]);
        }

        // Cargar límite en R8 (fuera del loop - loop invariant)
        // mov r8, limit
        self.emit_bytes(&[0x49, 0xB8]);
        self.emit_u64(limit as u64);

        // Loop optimizado:
        // loop_start:
        //   cmp rcx, r8
        //   jge loop_end
        //   inc rcx
        //   jmp loop_start
        // loop_end:

        let loop_start = self.code.len();

        // cmp rcx, r8
        self.emit_bytes(&[0x4C, 0x39, 0xC1]);

        // jge loop_end (saltar si rcx >= r8)
        self.emit_bytes(&[0x0F, 0x8D]);
        let jge_offset_pos = self.code.len();
        self.emit_i32(0);

        // inc rcx (1 sola instrucción!)
        self.emit_bytes(&[0x48, 0xFF, 0xC1]);

        // jmp loop_start
        self.emit_bytes(&[0xE9]);
        let jmp_back = (loop_start as i64 - self.code.len() as i64 - 4) as i32;
        self.emit_i32(jmp_back);

        // Parchear salto de salida
        let loop_end = self.code.len();
        let jge_offset = (loop_end - jge_offset_pos - 4) as i32;
        self.code[jge_offset_pos..jge_offset_pos + 4].copy_from_slice(&jge_offset.to_le_bytes());

        // Guardar resultado de vuelta en memoria
        if let Some(&offset) = self.variables.get(counter_var) {
            // mov [rbp + offset], rcx
            self.emit_bytes(&[0x48, 0x89, 0x8D]);
            self.emit_i32(offset);
        } else {
            // Nueva variable
            let offset = self.stack_offset;
            self.variables.insert(counter_var.to_string(), offset);
            self.stack_offset -= 8;
            self.emit_bytes(&[0x48, 0x89, 0x8D]);
            self.emit_i32(offset);
        }
    }

    fn emit_for(&mut self, var: &str, start: &Expr, end: &Expr, body: &[Stmt]) {
        // OPTIMIZACIÓN: Si el body está vacío, usar loop ultra-rápido en registros
        if body.is_empty() {
            // Loop vacío optimizado: solo contar
            if let (Expr::Number(start_val), Expr::Number(end_val)) = (start, end) {
                self.emit_optimized_empty_for(*start_val, *end_val, var);
                return;
            }
        }

        // Loop normal con registros para contador
        // RCX = contador, R8 = límite

        // Evaluar start en RAX, mover a RCX
        self.emit_expression(start);
        self.emit_bytes(&[0x48, 0x89, 0xC1]); // mov rcx, rax

        // Evaluar end en RAX, mover a R8
        self.emit_expression(end);
        self.emit_bytes(&[0x49, 0x89, 0xC0]); // mov r8, rax

        // Registrar variable (para acceso dentro del body)
        let var_offset = self.stack_offset;
        self.variables.insert(var.to_string(), var_offset);
        self.stack_offset -= 8;

        let loop_start = self.code.len();

        // cmp rcx, r8
        self.emit_bytes(&[0x4C, 0x39, 0xC1]);

        // jge end
        self.emit_bytes(&[0x0F, 0x8D]);
        let jge_offset_pos = self.code.len();
        self.emit_i32(0);

        // Guardar RCX en variable para uso en body
        self.emit_bytes(&[0x48, 0x89, 0x8D]); // mov [rbp + offset], rcx
        self.emit_i32(var_offset);

        // Guardar RCX y R8 antes del body (pueden ser modificados)
        self.emit_bytes(&[0x51]); // push rcx
        self.emit_bytes(&[0x41, 0x50]); // push r8

        for stmt in body {
            self.emit_statement(stmt);
        }

        // Restaurar R8 y RCX
        self.emit_bytes(&[0x41, 0x58]); // pop r8
        self.emit_bytes(&[0x59]); // pop rcx

        // inc rcx
        self.emit_bytes(&[0x48, 0xFF, 0xC1]);

        // jmp loop_start
        self.emit_bytes(&[0xE9]);
        let jmp_back = (loop_start as i64 - self.code.len() as i64 - 4) as i32;
        self.emit_i32(jmp_back);

        let end_label = self.code.len();
        let jge_offset = (end_label - jge_offset_pos - 4) as i32;
        self.code[jge_offset_pos..jge_offset_pos + 4].copy_from_slice(&jge_offset.to_le_bytes());

        // Guardar valor final en variable
        self.emit_bytes(&[0x48, 0x89, 0x8D]); // mov [rbp + offset], rcx
        self.emit_i32(var_offset);
    }

    /// Loop for vacío ultra-optimizado
    fn emit_optimized_empty_for(&mut self, start: i64, end: i64, var: &str) {
        // Simplemente asignar el valor final
        // mov rax, end
        self.emit_bytes(&[0x48, 0xB8]);
        self.emit_u64(end as u64);

        let var_offset = self.stack_offset;
        self.variables.insert(var.to_string(), var_offset);
        self.stack_offset -= 8;
        self.emit_bytes(&[0x48, 0x89, 0x85]);
        self.emit_i32(var_offset);

        let _ = start; // Evitar warning
    }

    fn emit_foreach(&mut self, var: &str, iterable: &Expr, body: &[Stmt]) {
        // for x in arr { } - iterar sobre un array
        // 1. Evaluar el iterable (obtener dirección base del array)
        self.emit_expression(iterable);
        let arr_base_offset = self.stack_offset;
        self.stack_offset -= 8;
        self.emit_bytes(&[0x48, 0x89, 0x85]); // mov [rbp + offset], rax
        self.emit_i32(arr_base_offset);

        // 2. Leer longitud del array (está en [rax])
        self.emit_bytes(&[0x48, 0x8B, 0x00]); // mov rax, [rax]
        let len_offset = self.stack_offset;
        self.stack_offset -= 8;
        self.emit_bytes(&[0x48, 0x89, 0x85]); // mov [rbp + offset], rax
        self.emit_i32(len_offset);

        // 3. Inicializar índice a 0
        self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
        let idx_offset = self.stack_offset;
        self.stack_offset -= 8;
        self.emit_bytes(&[0x48, 0x89, 0x85]); // mov [rbp + offset], rax
        self.emit_i32(idx_offset);

        // 4. Variable del loop
        let var_offset = self.stack_offset;
        self.variables.insert(var.to_string(), var_offset);
        self.stack_offset -= 8;

        let loop_start = self.code.len();

        // 5. Comparar índice con longitud
        self.emit_bytes(&[0x48, 0x8B, 0x85]); // mov rax, [rbp + idx_offset]
        self.emit_i32(idx_offset);
        self.emit_bytes(&[0x48, 0x3B, 0x85]); // cmp rax, [rbp + len_offset]
        self.emit_i32(len_offset);

        // jge end (salir si idx >= len)
        self.emit_bytes(&[0x0F, 0x8D]);
        let jge_offset_pos = self.code.len();
        self.emit_i32(0);

        // 6. Cargar elemento actual: arr[idx]
        // Calcular offset: (idx + 1) * 8
        self.emit_bytes(&[0x48, 0x8B, 0x85]); // mov rax, [rbp + idx_offset]
        self.emit_i32(idx_offset);
        self.emit_bytes(&[0x48, 0xFF, 0xC0]); // inc rax
        self.emit_bytes(&[0x48, 0xC1, 0xE0, 0x03]); // shl rax, 3

        // rbx = arr_base
        self.emit_bytes(&[0x48, 0x8B, 0x9D]); // mov rbx, [rbp + arr_base_offset]
        self.emit_i32(arr_base_offset);

        // rbx = rbx - rax (porque stack crece hacia abajo)
        self.emit_bytes(&[0x48, 0x29, 0xC3]); // sub rbx, rax

        // rax = [rbx] (valor del elemento)
        self.emit_bytes(&[0x48, 0x8B, 0x03]); // mov rax, [rbx]

        // Guardar en variable del loop
        self.emit_bytes(&[0x48, 0x89, 0x85]); // mov [rbp + var_offset], rax
        self.emit_i32(var_offset);

        // 7. Ejecutar cuerpo del loop
        for stmt in body {
            self.emit_statement(stmt);
        }

        // 8. Incrementar índice
        self.emit_bytes(&[0x48, 0xFF, 0x85]); // inc [rbp + idx_offset]
        self.emit_i32(idx_offset);

        // 9. Saltar al inicio del loop
        self.emit_bytes(&[0xE9]);
        let jmp_back = (loop_start as i64 - self.code.len() as i64 - 4) as i32;
        self.emit_i32(jmp_back);

        // 10. Parchear salto de salida
        let end_label = self.code.len();
        let jge_offset = (end_label - jge_offset_pos - 4) as i32;
        self.code[jge_offset_pos..jge_offset_pos + 4].copy_from_slice(&jge_offset.to_le_bytes());
    }

    fn emit_return(&mut self, expr: Option<&Expr>) {
        if let Some(e) = expr {
            self.emit_expression(e);
        } else {
            self.emit_bytes(&[0x31, 0xC0]);
        }
        self.emit_bytes(&[0x48, 0x89, 0xEC]);
        self.emit_bytes(&[0x5D]);
        self.emit_bytes(&[0xC3]);
    }

    fn emit_condition(&mut self, expr: &Expr) {
        match expr {
            Expr::Comparison { op, left, right } => {
                self.emit_expression(left);
                self.emit_bytes(&[0x50]);
                self.emit_expression(right);
                self.emit_bytes(&[0x48, 0x89, 0xC3]);
                self.emit_bytes(&[0x58]);
                self.emit_bytes(&[0x48, 0x39, 0xD8]);

                match op {
                    CmpOp::Eq => self.emit_bytes(&[0x0F, 0x94, 0xC0]),
                    CmpOp::Ne => self.emit_bytes(&[0x0F, 0x95, 0xC0]),
                    CmpOp::Lt => self.emit_bytes(&[0x0F, 0x9C, 0xC0]),
                    CmpOp::Le => self.emit_bytes(&[0x0F, 0x9E, 0xC0]),
                    CmpOp::Gt => self.emit_bytes(&[0x0F, 0x9F, 0xC0]),
                    CmpOp::Ge => self.emit_bytes(&[0x0F, 0x9D, 0xC0]),
                }

                self.emit_bytes(&[0x48, 0x0F, 0xB6, 0xC0]);
            }
            Expr::Bool(b) => {
                let val = if *b { 1u32 } else { 0u32 };
                self.emit_bytes(&[0xB8]);
                self.emit_u32(val);
            }
            _ => self.emit_expression(expr),
        }
    }

    fn emit_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Number(n) => {
                self.emit_bytes(&[0x48, 0xB8]);
                self.emit_u64(*n as u64);
            }
            Expr::Float(f) => {
                // Cargar flotante como bits en RAX para pasar a printf
                // printf en Windows x64 espera double en XMM1 para %f
                let bits = f.to_bits();
                self.emit_bytes(&[0x48, 0xB8]); // mov rax, imm64
                self.emit_u64(bits);
            }
            Expr::Bool(b) => {
                let val = if *b { 1u64 } else { 0u64 };
                self.emit_bytes(&[0x48, 0xB8]);
                self.emit_u64(val);
            }
            Expr::Variable(name) => {
                if let Some(&offset) = self.variables.get(name) {
                    self.emit_bytes(&[0x48, 0x8B, 0x85]);
                    self.emit_i32(offset);
                } else {
                    self.emit_bytes(&[0x31, 0xC0]);
                }
            }
            Expr::BinaryOp { op, left, right } => {
                self.emit_expression(left);
                self.emit_bytes(&[0x50]);
                self.emit_expression(right);
                self.emit_bytes(&[0x48, 0x89, 0xC3]);
                self.emit_bytes(&[0x58]);

                match op {
                    BinOp::Add => self.emit_bytes(&[0x48, 0x01, 0xD8]),
                    BinOp::Sub => self.emit_bytes(&[0x48, 0x29, 0xD8]),
                    BinOp::Mul => self.emit_bytes(&[0x48, 0x0F, 0xAF, 0xC3]),
                    BinOp::Div => {
                        self.emit_bytes(&[0x48, 0x99]);
                        self.emit_bytes(&[0x48, 0xF7, 0xFB]);
                    }
                    BinOp::Mod => {
                        self.emit_bytes(&[0x48, 0x99]);
                        self.emit_bytes(&[0x48, 0xF7, 0xFB]);
                        self.emit_bytes(&[0x48, 0x89, 0xD0]);
                    }
                    BinOp::And => self.emit_bytes(&[0x48, 0x21, 0xD8]),
                    BinOp::Or => self.emit_bytes(&[0x48, 0x09, 0xD8]),
                }
            }
            Expr::UnaryOp { op, expr: inner } => {
                self.emit_expression(inner);
                match op {
                    UnaryOp::Neg => self.emit_bytes(&[0x48, 0xF7, 0xD8]),
                    UnaryOp::Not => {
                        self.emit_bytes(&[0x48, 0x85, 0xC0]);
                        self.emit_bytes(&[0x0F, 0x94, 0xC0]);
                        self.emit_bytes(&[0x48, 0x0F, 0xB6, 0xC0]);
                    }
                }
            }
            Expr::Call { name, args } => {
                self.emit_call(name, args);
            }
            Expr::Input => {
                self.emit_input();
            }
            Expr::Comparison { .. } => self.emit_condition(expr),
            // Built-in functions v1.3.0
            Expr::Array(elements) => {
                // Almacenar elementos del array en variables locales consecutivas
                // Primero guardamos la longitud, luego cada elemento
                let len = elements.len() as i64;

                // Reservar espacio para longitud + elementos
                let array_base = self.stack_offset;

                // Guardar longitud en primera posición
                self.emit_bytes(&[0x48, 0xB8]); // mov rax, len
                self.emit_u64(len as u64);
                self.emit_bytes(&[0x48, 0x89, 0x85]); // mov [rbp + offset], rax
                self.emit_i32(array_base);
                self.stack_offset -= 8;

                // Guardar cada elemento
                for elem in elements.iter() {
                    self.emit_expression(elem);
                    self.emit_bytes(&[0x48, 0x89, 0x85]); // mov [rbp + offset], rax
                    self.emit_i32(self.stack_offset);
                    self.stack_offset -= 8;
                }

                // RAX = dirección base del array (donde está la longitud)
                self.emit_bytes(&[0x48, 0x8D, 0x85]); // lea rax, [rbp + offset]
                self.emit_i32(array_base);
            }
            Expr::Index { object, index } => {
                // Para indexación, primero evaluamos el objeto (dirección base)
                // luego el índice, y calculamos la dirección del elemento

                // Evaluar objeto primero (obtener dirección base)
                self.emit_expression(object);
                self.emit_bytes(&[0x48, 0x89, 0xC3]); // mov rbx, rax (guardar base)

                // Evaluar índice
                self.emit_expression(index);

                // Calcular offset: (índice + 1) * 8 (saltamos la longitud)
                self.emit_bytes(&[0x48, 0xFF, 0xC0]); // inc rax
                self.emit_bytes(&[0x48, 0xC1, 0xE0, 0x03]); // shl rax, 3 (multiplicar por 8)

                // Restar del base (porque stack crece hacia abajo)
                self.emit_bytes(&[0x48, 0x29, 0xC3]); // sub rbx, rax

                // Cargar valor: rax = [rbx]
                self.emit_bytes(&[0x48, 0x8B, 0x03]); // mov rax, [rbx]
            }
            Expr::Len(inner) => {
                // Para len(), evaluamos la expresión y leemos la longitud
                // que está en la primera posición del array
                self.emit_expression(inner);
                // rax ya tiene la dirección base, la longitud está en [rax]
                self.emit_bytes(&[0x48, 0x8B, 0x00]); // mov rax, [rax]
            }
            Expr::IntCast(inner) => {
                self.emit_expression(inner);
                // El valor ya está en RAX como entero
            }
            Expr::FloatCast(inner) => {
                self.emit_expression(inner);
                // cvtsi2sd xmm0, rax - convertir entero a double
                self.emit_bytes(&[0xF2, 0x48, 0x0F, 0x2A, 0xC0]);
                // movq rax, xmm0 - mover bits de vuelta a rax
                self.emit_bytes(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]);
            }
            Expr::StrCast(_inner) => {
                // Conversión a string - placeholder
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
            Expr::BoolCast(inner) => {
                self.emit_expression(inner);
                // test rax, rax; setne al; movzx rax, al
                self.emit_bytes(&[0x48, 0x85, 0xC0]); // test rax, rax
                self.emit_bytes(&[0x0F, 0x95, 0xC0]); // setne al
                self.emit_bytes(&[0x48, 0x0F, 0xB6, 0xC0]); // movzx rax, al
            }
            Expr::Push { array: _, value: _ } => {
                // Push a array - placeholder
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
            Expr::Pop(_) => {
                // Pop de array - placeholder
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
            Expr::StringConcat { left, right } => {
                // Concatenación de strings - placeholder
                self.emit_expression(left);
                self.emit_expression(right);
            }
            Expr::Slice { .. }
            | Expr::New { .. }
            | Expr::MethodCall { .. }
            | Expr::FieldAccess { .. }
            | Expr::This
            | Expr::Super
            | Expr::Lambda { .. }
            | Expr::Ternary { .. }
            | Expr::String(_)
            | Expr::Null => {
                // No soportado en codegen_v2 por ahora
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }

            // ========== PUNTEROS Y MEMORIA (v3.2) ==========
            Expr::Deref(ptr) => {
                self.emit_expression(ptr);
                // mov rax, [rax] - dereference pointer
                self.emit_bytes(&[0x48, 0x8B, 0x00]);
            }
            Expr::AddressOf(expr) => {
                if let Expr::Variable(name) = expr.as_ref() {
                    if let Some(&offset) = self.variables.get(name) {
                        // lea rax, [rbp + offset]
                        self.emit_bytes(&[0x48, 0x8D, 0x85]);
                        self.emit_i32(offset);
                    } else {
                        self.emit_bytes(&[0x48, 0x31, 0xC0]);
                    }
                } else {
                    self.emit_expression(expr);
                }
            }
            Expr::ArrowAccess { pointer, field: _ } => {
                self.emit_expression(pointer);
                self.emit_bytes(&[0x48, 0x8B, 0x00]); // mov rax, [rax]
            }
            Expr::SizeOf(_) => {
                // sizeof(int) = 8 en x86-64
                self.emit_bytes(&[0x48, 0xB8]);
                self.emit_u64(8);
            }
            Expr::Malloc(size) => {
                self.emit_expression(size);
                // Placeholder - llamar a HeapAlloc
                self.emit_bytes(&[0x48, 0x31, 0xC0]);
            }
            Expr::Realloc { ptr, new_size } => {
                self.emit_expression(ptr);
                self.emit_expression(new_size);
                self.emit_bytes(&[0x48, 0x31, 0xC0]);
            }
            Expr::Cast {
                target_type: _,
                expr,
            } => {
                self.emit_expression(expr);
            }
            Expr::Nullptr => {
                self.emit_bytes(&[0x48, 0x31, 0xC0]);
            }
            Expr::PreIncrement(expr) | Expr::PostIncrement(expr) => {
                self.emit_expression(expr);
                self.emit_bytes(&[0x48, 0xFF, 0xC0]); // inc rax
            }
            Expr::PreDecrement(expr) | Expr::PostDecrement(expr) => {
                self.emit_expression(expr);
                self.emit_bytes(&[0x48, 0xFF, 0xC8]); // dec rax
            }
            Expr::BitwiseOp { op, left, right } => {
                self.emit_expression(left);
                self.emit_bytes(&[0x50]); // push rax
                self.emit_expression(right);
                self.emit_bytes(&[0x48, 0x89, 0xC1]); // mov rcx, rax
                self.emit_bytes(&[0x58]); // pop rax
                match op {
                    crate::frontend::ast::BitwiseOp::And => self.emit_bytes(&[0x48, 0x21, 0xC8]),
                    crate::frontend::ast::BitwiseOp::Or => self.emit_bytes(&[0x48, 0x09, 0xC8]),
                    crate::frontend::ast::BitwiseOp::Xor => self.emit_bytes(&[0x48, 0x31, 0xC8]),
                    crate::frontend::ast::BitwiseOp::LeftShift => {
                        self.emit_bytes(&[0x48, 0xD3, 0xE0])
                    }
                    crate::frontend::ast::BitwiseOp::RightShift => {
                        self.emit_bytes(&[0x48, 0xD3, 0xE8])
                    }
                }
            }
            Expr::BitwiseNot(expr) => {
                self.emit_expression(expr);
                self.emit_bytes(&[0x48, 0xF7, 0xD0]); // not rax
            }
            // ========== OS-LEVEL EXPRESSIONS (v3.1-OS) ==========
            Expr::RegRead { reg_name: _ } => {
                // reg(rax) — value is already in rax conceptually
                // In real code, this would map to the correct register
                // For now, nop (value stays in rax)
                self.emit_bytes(&[0x90]); // nop
            }
            Expr::MemRead { addr } => {
                // read_mem(addr) — mov rax, [rax]
                self.emit_expression(addr);
                self.emit_bytes(&[0x48, 0x8B, 0x00]); // mov rax, [rax]
            }
            Expr::PortIn { port } => {
                // port_in(port) — in al, dx
                self.emit_expression(port);
                self.emit_bytes(&[0x48, 0x89, 0xC2]); // mov rdx, rax (port number)
                self.emit_bytes(&[0xEC]); // in al, dx
                self.emit_bytes(&[0x48, 0x0F, 0xB6, 0xC0]); // movzx rax, al
            }
            Expr::CpuidExpr => {
                // cpuid — result in eax (leaf 0 by default)
                self.emit_bytes(&[0x31, 0xC0]); // xor eax, eax
                self.emit_bytes(&[0x0F, 0xA2]); // cpuid
                                                // eax already contains result
            }
            Expr::LabelAddr { .. } => {
                // Label address — not supported in legacy codegen_v2
                // Use ISA compiler for bootloader code
                eprintln!("⚠️  label_addr() not supported in codegen_v2, use ISA compiler");
                self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
            }
        }
    }

    fn emit_call(&mut self, name: &str, args: &[Expr]) {
        // Evaluar argumentos en registros (Windows x64)
        for (i, arg) in args.iter().enumerate().take(4) {
            self.emit_expression(arg);
            match i {
                0 => self.emit_bytes(&[0x48, 0x89, 0xC1]), // mov rcx, rax
                1 => self.emit_bytes(&[0x48, 0x89, 0xC2]), // mov rdx, rax
                2 => self.emit_bytes(&[0x49, 0x89, 0xC0]), // mov r8, rax
                3 => self.emit_bytes(&[0x49, 0x89, 0xC1]), // mov r9, rax
                _ => {}
            }
        }

        // Shadow space
        self.emit_bytes(&[0x48, 0x83, 0xEC, 0x20]);

        // call rel32 (placeholder)
        self.emit_bytes(&[0xE8]);
        let call_offset = self.code.len();
        self.function_calls.push((call_offset, name.to_string()));
        self.emit_i32(0);

        // Restaurar stack
        self.emit_bytes(&[0x48, 0x83, 0xC4, 0x20]);
    }

    fn emit_input(&mut self) {
        // input() - v1.4.0: Lee un entero de stdin usando scanf("%d", &var)
        // Windows x64 calling convention:
        // rcx = primer arg (formato "%d")
        // rdx = segundo arg (puntero a variable)

        // Reservar espacio en stack para el resultado (8 bytes)
        let input_var_offset = self.stack_offset;
        self.stack_offset -= 8;

        // Inicializar variable a 0
        self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax
        self.emit_bytes(&[0x48, 0x89, 0x85]); // mov [rbp + offset], rax
        self.emit_i32(input_var_offset);

        // rcx = formato "%d"
        let fmt_addr = self.get_string_address("%d");
        self.emit_bytes(&[0x48, 0xB9]); // mov rcx, imm64
        self.emit_u64(fmt_addr);

        // rdx = &variable (lea rdx, [rbp + offset])
        self.emit_bytes(&[0x48, 0x8D, 0x95]); // lea rdx, [rbp + disp32]
        self.emit_i32(input_var_offset);

        // Llamar scanf
        self.emit_call_scanf();

        // Cargar resultado en rax
        self.emit_bytes(&[0x48, 0x8B, 0x85]); // mov rax, [rbp + offset]
        self.emit_i32(input_var_offset);
    }

    // ========================================
    // Helpers
    // ========================================

    fn emit_bytes(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    fn emit_u32(&mut self, value: u32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    fn emit_u64(&mut self, value: u64) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    fn emit_i32(&mut self, value: i32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_v2_creation() {
        let cg = CodeGeneratorV2::new(Target::Windows);
        assert_eq!(cg.base_address, 0x0000000140000000);
    }

    #[test]
    fn test_codegen_v2_linux() {
        let cg = CodeGeneratorV2::new(Target::Linux);
        assert_eq!(cg.target, Target::Linux);
        assert_eq!(cg.base_address, 0x400000);
    }
}
