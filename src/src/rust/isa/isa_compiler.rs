// ============================================================
// ADead-BIB ISA Compiler — AST → ADeadIR → Bytes
// ============================================================
// Reemplaza codegen_v2.rs. En vez de emitir bytes directamente,
// genera instrucciones ADeadOp tipadas que luego se codifican.
//
// Pipeline: AST → ADeadIR (Vec<ADeadOp>) → Encoder → bytes
//
// Sin ASM. Sin NASM. Sin LLVM. Solo ISA puro.
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use super::encoder::Encoder;
use super::reg_alloc::TempAllocator;
use super::{ADeadIR, ADeadOp, CallTarget, Condition, Label, Operand, Reg};
use crate::backend::cpu::iat_registry;
use crate::frontend::ast::*;
use std::collections::HashMap;

/// Target de compilación
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    Windows,
    Linux,
    Raw,
}

/// CPU Mode — Escalado natural de ADead-BIB: 16-bit → 32-bit → 64-bit
/// Default: Long64 (64-bit). ADead-BIB escala naturalmente desde la base.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CpuMode {
    /// 16-bit real mode — boot sectors, BIOS calls
    Real16,
    /// 32-bit protected mode — legacy drivers, transitions
    Protected32,
    /// 64-bit long mode — kernel, applications (DEFAULT)
    Long64,
}

impl CpuMode {
    /// Operand size in bits for this mode
    pub fn operand_bits(&self) -> u8 {
        match self {
            CpuMode::Real16 => 16,
            CpuMode::Protected32 => 32,
            CpuMode::Long64 => 64,
        }
    }

    /// Address size in bits for this mode
    pub fn address_bits(&self) -> u8 {
        match self {
            CpuMode::Real16 => 16,
            CpuMode::Protected32 => 32,
            CpuMode::Long64 => 64,
        }
    }

    /// Whether this mode needs REX prefix for 64-bit operands
    pub fn needs_rex(&self) -> bool {
        matches!(self, CpuMode::Long64)
    }

    /// Stack pointer register name for this mode
    pub fn stack_reg(&self) -> &'static str {
        match self {
            CpuMode::Real16 => "SP",
            CpuMode::Protected32 => "ESP",
            CpuMode::Long64 => "RSP",
        }
    }
}

impl Default for CpuMode {
    fn default() -> Self {
        CpuMode::Long64 // ADead-BIB defaults to 64-bit
    }
}

/// Función compilada (metadatos)
#[derive(Clone, Debug)]
pub struct CompiledFunction {
    pub name: String,
    pub label: Label,
    pub params: Vec<String>,
}

/// Class/Struct layout info - inspired by GCC/LLVM Itanium ABI
#[derive(Debug, Clone)]
pub struct ClassLayout {
    pub name: String,
    pub fields: Vec<(String, i32)>,         // (field_name, offset)
    pub field_types: Vec<(String, String)>, // (field_name, type_name) for nested struct detection
    pub size: i32,                          // Stack layout size (8-byte aligned slots)
    pub real_size: i32,                     // True C99 sizeof (for sizeof operator)
}

/// ISA Compiler — Compilador que genera ADeadIR en vez de bytes directos.
pub struct IsaCompiler {
    ir: ADeadIR,

    // Strings
    strings: Vec<String>,
    string_offsets: HashMap<String, u64>,

    // Funciones
    functions: HashMap<String, CompiledFunction>,

    // Class layouts - GCC/LLVM style field offset tracking
    class_layouts: HashMap<String, ClassLayout>,

    // Estado actual
    current_function: Option<String>,
    variables: HashMap<String, i32>,
    variable_types: HashMap<String, Type>,
    array_vars: std::collections::HashSet<String>,
    param_vars: std::collections::HashSet<String>, // Function parameters (use 8-byte stride)
    struct_params: std::collections::HashSet<String>, // Struct parameters (passed by pointer)
    ref_vars: std::collections::HashSet<String>,   // Reference parameters (auto-deref)
    stack_offset: i32,

    // Configuración
    target: Target,
    base_address: u64,
    data_rva: u64,

    // CPU Mode — 16/32/64-bit scaling (default: 64-bit)
    cpu_mode: CpuMode,

    // Named labels (v3.3-Boot) — maps label names to Label IDs
    named_labels: HashMap<String, Label>,

    // Register allocator for temporaries — eliminates push/pop in expressions
    temp_alloc: TempAllocator,

    // Track prologue sub rsp index for patching dynamic stack frame
    prologue_sub_index: Option<usize>,

    // Loop label stack for break/continue — each entry is (break_label, continue_label)
    loop_stack: Vec<(Label, Label)>,

    // Global/static variables — stored at absolute addresses in data section
    // Maps variable name → byte offset within global data area
    global_vars: HashMap<String, u32>,
    global_data: Vec<u8>, // raw bytes for global variable initial values
    global_offset: u32,   // next free offset in global data area
}

impl IsaCompiler {
    pub fn new(target: Target) -> Self {
        let (base, data_rva) = match target {
            Target::Windows => {
                // Compute data_rva dynamically from IAT registry
                // ISA assumes idata_rva=0x2000, PE builder patches to real value
                let assumed_idata_rva: u64 = 0x2000;
                let idata_result = iat_registry::build_idata(assumed_idata_rva as u32, &[]);
                let drva = assumed_idata_rva + idata_result.program_strings_offset as u64;
                (0x0000000140000000u64, drva)
            }
            Target::Linux => (0x400000, 0x1000),
            Target::Raw => (0x0, 0x1000),
        };

        // Initialize default class layouts based on GCC/LLVM ABI research
        let mut class_layouts = HashMap::new();

        // Counter class layout
        class_layouts.insert(
            "Counter".to_string(),
            ClassLayout {
                name: "Counter".to_string(),
                fields: vec![("value".to_string(), 0), ("max_value".to_string(), 8)],
                field_types: vec![],
                size: 16,
                real_size: 16,
            },
        );

        // Point2D class layout
        class_layouts.insert(
            "Point2D".to_string(),
            ClassLayout {
                name: "Point2D".to_string(),
                fields: vec![("x".to_string(), 0), ("y".to_string(), 8)],
                field_types: vec![],
                size: 16,
                real_size: 16,
            },
        );

        // Shape class layout (base class for inheritance)
        class_layouts.insert(
            "Shape".to_string(),
            ClassLayout {
                name: "Shape".to_string(),
                fields: vec![("id".to_string(), 0)],
                field_types: vec![],
                size: 8,
                real_size: 8,
            },
        );

        // Circle class layout (inherits Shape)
        class_layouts.insert(
            "Circle".to_string(),
            ClassLayout {
                name: "Circle".to_string(),
                fields: vec![("id".to_string(), 0), ("radius".to_string(), 8)],
                field_types: vec![],
                size: 16,
                real_size: 16,
            },
        );

        // Rectangle class layout (inherits Shape)
        class_layouts.insert(
            "Rectangle".to_string(),
            ClassLayout {
                name: "Rectangle".to_string(),
                fields: vec![
                    ("id".to_string(), 0),
                    ("w".to_string(), 8),
                    ("h".to_string(), 16),
                ],
                field_types: vec![],
                size: 24,
                real_size: 24,
            },
        );

        // Rect class layout
        class_layouts.insert(
            "Rect".to_string(),
            ClassLayout {
                name: "Rect".to_string(),
                fields: vec![
                    ("origin".to_string(), 0),
                    ("width".to_string(), 16),
                    ("height".to_string(), 24),
                ],
                field_types: vec![],
                size: 32,
                real_size: 32,
            },
        );

        // Stack class layout
        class_layouts.insert(
            "Stack".to_string(),
            ClassLayout {
                name: "Stack".to_string(),
                fields: vec![("data".to_string(), 0), ("top".to_string(), 8)],
                field_types: vec![],
                size: 16,
                real_size: 16,
            },
        );

        // Queue class layout
        class_layouts.insert(
            "Queue".to_string(),
            ClassLayout {
                name: "Queue".to_string(),
                fields: vec![
                    ("data".to_string(), 0),
                    ("front".to_string(), 8),
                    ("rear".to_string(), 16),
                    ("count".to_string(), 24),
                ],
                field_types: vec![],
                size: 32,
                real_size: 32,
            },
        );

        // LinkedList/Node class layout
        class_layouts.insert(
            "LinkedList".to_string(),
            ClassLayout {
                name: "LinkedList".to_string(),
                fields: vec![("head".to_string(), 0)],
                field_types: vec![],
                size: 8,
                real_size: 8,
            },
        );

        class_layouts.insert(
            "Node".to_string(),
            ClassLayout {
                name: "Node".to_string(),
                fields: vec![("value".to_string(), 0), ("next".to_string(), 8)],
                field_types: vec![],
                size: 16,
                real_size: 16,
            },
        );

        Self {
            ir: ADeadIR::new(),
            strings: Vec::new(),
            string_offsets: HashMap::new(),
            functions: HashMap::new(),
            class_layouts,
            current_function: None,
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            array_vars: std::collections::HashSet::new(),
            param_vars: std::collections::HashSet::new(),
            struct_params: std::collections::HashSet::new(),
            ref_vars: std::collections::HashSet::new(),
            stack_offset: 0,
            target,
            base_address: base,
            data_rva,
            cpu_mode: CpuMode::Long64, // Default: 64-bit
            named_labels: HashMap::new(),
            temp_alloc: TempAllocator::new(),
            prologue_sub_index: None,
            loop_stack: Vec::new(),
            global_vars: HashMap::new(),
            global_data: Vec::new(),
            global_offset: 0,
        }
    }

    /// Create compiler with specific CPU mode (16/32/64-bit scaling)
    pub fn with_cpu_mode(target: Target, mode: CpuMode) -> Self {
        let mut compiler = Self::new(target);
        compiler.cpu_mode = mode;
        compiler
    }

    /// Get field offset from class layout (MSVC/GCC/LLVM ABI style)
    /// Returns the byte offset of a field within a class/struct
    fn get_field_offset(&self, field_name: &str) -> i32 {
        // First, try to find the field in any registered class layout (dynamic from program)
        for layout in self.class_layouts.values() {
            for (name, offset) in &layout.fields {
                if name == field_name {
                    return *offset;
                }
            }
        }
        // Fallback to hardcoded common offsets (MSVC x64 ABI: 8-byte aligned)
        match field_name {
            "value" => 0,
            "max_value" => 8,
            "x" => 0,
            "y" => 8,
            "z" => 16,
            "w" => 8,
            "width" => 16, // After origin (Point2D = 16 bytes)
            "h" => 16,
            "height" => 24,
            "data" => 0,
            "top" => 8,
            "front" => 8,
            "rear" => 16,
            "count" => 24,
            "head" => 0,
            "next" => 8,
            "id" => 0,
            "radius" => 8,
            "origin" => 0,
            _ => 0,
        }
    }

    /// Get field offset for a specific class/struct (MSVC x64 ABI)
    /// First tries class_layouts, then falls back to generic field offset
    fn get_class_field_offset(&self, class_name: &str, field_name: &str) -> i32 {
        // Try exact class layout first
        if let Some(layout) = self.class_layouts.get(class_name) {
            for (name, offset) in &layout.fields {
                if name == field_name {
                    return *offset;
                }
            }
        }
        // Fallback to generic field offset
        self.get_field_offset(field_name)
    }

    /// Compute sizeof for a Type, consulting class_layouts for structs
    /// and computing real sizes for arrays.
    fn sizeof_type(&self, ty: &Type) -> i64 {
        match ty {
            Type::Struct(name) | Type::Class(name) | Type::Named(name) => {
                // Look up struct/class in registered layouts
                if let Some(layout) = self.class_layouts.get(name.as_str()) {
                    // Use real_size for C99 sizeof compliance (true field sizes with alignment).
                    // malloc() is separately scaled ×2 to cover our 8-byte field layout.
                    layout.real_size as i64
                } else {
                    // Try c99_sizeof for named primitive types
                    let s = crate::isa::c_isa::c99_sizeof_for_expr(ty);
                    if s > 0 {
                        s
                    } else {
                        8
                    }
                }
            }
            Type::Array(inner, Some(count)) => {
                // Array: element_size * count
                let elem_size = self.sizeof_type(inner);
                elem_size * (*count as i64)
            }
            Type::Array(_, None) => 8, // Flexible array → pointer
            Type::Pointer(_) => 8,
            _ => crate::isa::c_isa::c99_sizeof_for_expr(ty),
        }
    }

    /// Create compiler for 16-bit real mode (boot sectors)
    pub fn new_real16() -> Self {
        Self::with_cpu_mode(Target::Raw, CpuMode::Real16)
    }

    /// Create compiler for 32-bit protected mode
    pub fn new_protected32() -> Self {
        Self::with_cpu_mode(Target::Raw, CpuMode::Protected32)
    }

    /// Create compiler for 64-bit long mode (default)
    pub fn new_long64(target: Target) -> Self {
        Self::with_cpu_mode(target, CpuMode::Long64)
    }

    /// Get the element stride (in bytes) for pointer indexing based on variable type.
    /// For `char *` / `I8` / `U8` → 1, `short *` → 2, `int *` → 4, `long *` / default → 8
    fn element_stride(&self, var_name: &str) -> u8 {
        if let Some(vt) = self.variable_types.get(var_name) {
            match vt {
                Type::Pointer(inner) | Type::Array(inner, _) => match inner.as_ref() {
                    // Only byte-sized types use stride 1 (string literals, char arrays)
                    // Everything else uses 8-byte stride because our stack stores
                    // all values as 64-bit qwords.
                    Type::I8 | Type::U8 | Type::Bool => 1,
                    _ => 8,
                },
                _ => 8,
            }
        } else {
            8 // default: 64-bit slots
        }
    }

    /// Check if a variable is a heap pointer (not a local array on stack)
    fn is_heap_pointer(&self, var_name: &str) -> bool {
        if self.array_vars.contains(var_name) {
            return false; // local array
        }
        if let Some(vt) = self.variable_types.get(var_name) {
            matches!(vt, Type::Pointer(_))
        } else {
            false
        }
    }

    /// Emit index scaling: RAX = RAX * stride.
    /// Uses SHL for power-of-2 strides, no-op for stride 1.
    fn emit_index_scale(&mut self, stride: u8) {
        match stride {
            1 => {} // no scaling needed for byte access
            2 => {
                self.ir.emit(ADeadOp::Shl {
                    dst: Reg::RAX,
                    amount: 1,
                });
            }
            4 => {
                self.ir.emit(ADeadOp::Shl {
                    dst: Reg::RAX,
                    amount: 2,
                });
            }
            8 => {
                self.ir.emit(ADeadOp::Shl {
                    dst: Reg::RAX,
                    amount: 3,
                });
            }
            _ => {
                self.ir.emit(ADeadOp::Shl {
                    dst: Reg::RAX,
                    amount: 3,
                });
            }
        }
    }

    /// Emit load from memory with appropriate width.
    /// For byte access (stride=1), load 64-bit and AND with 0xFF to isolate the byte.
    fn emit_load_with_stride(&mut self, base_reg: Reg, stride: u8) {
        // Load full qword from [base_reg]
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RAX),
            src: Operand::Mem {
                base: base_reg,
                disp: 0,
            },
        });
        if stride == 1 {
            // Mask to single byte: movzx rax, al
            // Emitted as: and rax, 0xFF using mov rbx, 0xFF + and rax, rbx
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBX),
                src: Operand::Imm32(0xFF),
            });
            self.ir.emit(ADeadOp::And {
                dst: Reg::RAX,
                src: Reg::RBX,
            });
        }
    }

    /// Set CPU mode at runtime (for mode transitions)
    pub fn set_cpu_mode(&mut self, mode: CpuMode) {
        self.cpu_mode = mode;
    }

    /// Get current CPU mode
    pub fn cpu_mode(&self) -> CpuMode {
        self.cpu_mode
    }

    /// Access class_layouts for external ISA compilers (c_isa, cpp_isa)
    pub fn class_layouts(&self) -> &HashMap<String, ClassLayout> {
        &self.class_layouts
    }

    /// Insert or override a class layout (used by c_isa/cpp_isa for specialized policies)
    pub fn insert_class_layout(&mut self, name: String, layout: ClassLayout) {
        self.class_layouts.insert(name, layout);
    }

    /// Compila un programa completo y retorna (code_bytes, data_bytes, iat_offsets, string_offsets).
    pub fn compile(&mut self, program: &Program) -> (Vec<u8>, Vec<u8>, Vec<usize>, Vec<usize>) {
        // Fase 0: Registrar layouts de structs/clases del programa (MSVC ABI style)
        for st in &program.structs {
            let mut fields = Vec::new();
            let mut offset = 0i32;
            for field in &st.fields {
                fields.push((field.name.clone(), offset));
                // Compute field size: structs use layout size, arrays use count*8, primitives use 8
                let field_size = match &field.field_type {
                    Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                        self.class_layouts.get(n).map(|l| l.size).unwrap_or(8)
                    }
                    Type::Array(_, Some(count)) => (*count as i32) * 8,
                    Type::Array(_, None) => 8, // unsized array = pointer
                    _ => 8,                    // Primitives: 8-byte qword slots
                };
                offset += field_size;
            }
            let real_size_sum: i32 = st
                .fields
                .iter()
                .map(|f| {
                    let s = crate::isa::c_isa::c99_sizeof(&f.field_type);
                    if s > 0 {
                        s
                    } else {
                        8
                    }
                })
                .sum();
            let real_size = real_size_sum;
            // Collect field type names for nested struct resolution
            let field_types: Vec<(String, String)> = st
                .fields
                .iter()
                .map(|f| {
                    let type_name = match &f.field_type {
                        Type::Struct(n) | Type::Named(n) | Type::Class(n) => n.clone(),
                        _ => String::new(),
                    };
                    (f.name.clone(), type_name)
                })
                .collect();
            self.class_layouts.insert(
                st.name.clone(),
                ClassLayout {
                    name: st.name.clone(),
                    fields,
                    field_types,
                    size: offset,
                    real_size,
                },
            );
        }

        // Fase 0.5: Pre-register global variables (top-level VarDecl statements)
        // These are stored in the data section, not on any function's stack
        for stmt in &program.statements {
            if let Stmt::VarDecl { name, value, .. } = stmt {
                let init = if let Some(Expr::Number(n)) = value {
                    *n
                } else {
                    0
                };
                self.alloc_global(name, init);
            }
        }
        // Also scan functions for static locals
        for func in &program.functions {
            for stmt in &func.body {
                self.scan_static_locals(stmt, &func.name);
            }
        }

        // Fase 1: Recolectar strings
        self.collect_all_strings(program);
        self.collect_strings_from_stmts(&program.statements);

        // Fase 2: Registrar labels de funciones
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

        // Fase 3: Determinar entry point
        // Para binarios flat (bare metal), buscar _start o kernel_main primero
        let entry_name = if self.target == Target::Raw {
            if program.functions.iter().any(|f| f.name == "_start") {
                "_start"
            } else if program.functions.iter().any(|f| f.name == "kernel_main") {
                "kernel_main"
            } else {
                "main"
            }
        } else {
            "main"
        };

        let has_entry = program.functions.iter().any(|f| f.name == entry_name);
        let entry_label = self.functions.get(entry_name).map(|f| f.label);
        let needs_jmp =
            has_entry && (program.functions.len() > 1 || !program.statements.is_empty());
        if needs_jmp {
            if let Some(lbl) = entry_label {
                self.ir.emit(ADeadOp::Jmp { target: lbl });
            }
        }

        // Fase 3.5: Dead code elimination
        // For Target::Raw (flat/OS kernels), compile ALL functions — callbacks,
        // ISRs, shell commands etc. are not directly reachable from entry but needed.
        // DCE only applies to PE/ELF where unused header inlines waste space.
        let use_dce = self.target != Target::Raw;
        let reachable = if use_dce {
            Self::collect_reachable_functions(program, entry_name)
        } else {
            // Mark ALL functions as reachable for flat binaries
            program.functions.iter().map(|f| f.name.clone()).collect()
        };

        // Fase 4: Compilar funciones auxiliares (solo las alcanzables desde entry)
        for func in &program.functions {
            if func.name != entry_name && reachable.contains(&func.name) {
                self.compile_function(func);
            }
        }

        // Fase 5: Compilar top-level statements (only when no entry — script mode)
        if !has_entry && !program.statements.is_empty() {
            self.compile_top_level(&program.statements);
        }

        // Fase 6: Compilar entry point (main, _start, o kernel_main)
        for func in &program.functions {
            if func.name == entry_name {
                self.compile_function(func);
            }
        }

        // Fase 7: Encode ADeadIR → bytes
        let mut encoder = Encoder::new();
        let result = encoder.encode_all(self.ir.ops());

        // Fase 8: Resolver llamadas a funciones por nombre
        let code = result.code;
        for (offset, name) in &result.unresolved_calls {
            if let Some(func) = self.functions.get(name) {
                // Necesitamos saber la posición real del label en el código
                // El encoder ya resolvió los labels internos, pero las llamadas
                // por nombre quedan pendientes. Re-encode para obtener posiciones.
                // Por ahora, las llamadas internas usan CallTarget::Relative(label)
                // y solo Name() se usa para funciones externas no resueltas.
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

    /// Collect all function names transitively reachable from the entry point.
    /// This prevents unused header inline functions from inflating the code section.
    fn collect_reachable_functions(program: &Program, entry_name: &str) -> std::collections::HashSet<String> {
        let mut reachable = std::collections::HashSet::new();
        let mut worklist: Vec<String> = vec![entry_name.to_string()];

        // Build a map: function_name -> &Function for quick lookup
        let func_map: std::collections::HashMap<&str, &Function> = program.functions.iter()
            .map(|f| (f.name.as_str(), f))
            .collect();

        while let Some(name) = worklist.pop() {
            if reachable.contains(&name) {
                continue;
            }
            reachable.insert(name.clone());
            // Find the function body and collect all Call names
            if let Some(func) = func_map.get(name.as_str()) {
                let mut called = Vec::new();
                Self::collect_calls_from_stmts(&func.body, &mut called);
                for callee in called {
                    if !reachable.contains(&callee) && func_map.contains_key(callee.as_str()) {
                        worklist.push(callee);
                    }
                }
            }
        }
        // Also include functions called from top-level statements
        let mut top_calls = Vec::new();
        Self::collect_calls_from_stmts(&program.statements, &mut top_calls);
        for callee in top_calls {
            if !reachable.contains(&callee) && func_map.contains_key(callee.as_str()) {
                worklist.push(callee.clone());
                reachable.insert(callee);
            }
        }
        reachable
    }

    fn collect_calls_from_stmts(stmts: &[Stmt], calls: &mut Vec<String>) {
        for stmt in stmts {
            Self::collect_calls_from_stmt(stmt, calls);
        }
    }

    fn collect_calls_from_stmt(stmt: &Stmt, calls: &mut Vec<String>) {
        match stmt {
            Stmt::Expr(e) => Self::collect_calls_from_expr_dce(e, calls),
            Stmt::VarDecl { value: Some(e), .. } => Self::collect_calls_from_expr_dce(e, calls),
            Stmt::Assign { value, .. } => Self::collect_calls_from_expr_dce(value, calls),
            Stmt::Return(Some(e)) => Self::collect_calls_from_expr_dce(e, calls),
            Stmt::If { condition, then_body, else_body } => {
                Self::collect_calls_from_expr_dce(condition, calls);
                Self::collect_calls_from_stmts(then_body, calls);
                if let Some(eb) = else_body { Self::collect_calls_from_stmts(eb, calls); }
            }
            Stmt::While { condition, body } => {
                Self::collect_calls_from_expr_dce(condition, calls);
                Self::collect_calls_from_stmts(body, calls);
            }
            Stmt::For { start, end, body, .. } => {
                Self::collect_calls_from_expr_dce(start, calls);
                Self::collect_calls_from_expr_dce(end, calls);
                Self::collect_calls_from_stmts(body, calls);
            }
            Stmt::DoWhile { body, condition } => {
                Self::collect_calls_from_stmts(body, calls);
                Self::collect_calls_from_expr_dce(condition, calls);
            }
            Stmt::Switch { expr, cases, default } => {
                Self::collect_calls_from_expr_dce(expr, calls);
                for case in cases {
                    Self::collect_calls_from_stmts(&case.body, calls);
                }
                if let Some(d) = default { Self::collect_calls_from_stmts(d, calls); }
            }
            Stmt::CompoundAssign { value, .. } => Self::collect_calls_from_expr_dce(value, calls),
            Stmt::DerefAssign { pointer, value } => {
                Self::collect_calls_from_expr_dce(pointer, calls);
                Self::collect_calls_from_expr_dce(value, calls);
            }
            Stmt::ArrowAssign { pointer, value, .. } => {
                Self::collect_calls_from_expr_dce(pointer, calls);
                Self::collect_calls_from_expr_dce(value, calls);
            }
            Stmt::IndexAssign { object, index, value } => {
                Self::collect_calls_from_expr_dce(object, calls);
                Self::collect_calls_from_expr_dce(index, calls);
                Self::collect_calls_from_expr_dce(value, calls);
            }
            Stmt::FieldAssign { object, value, .. } => {
                Self::collect_calls_from_expr_dce(object, calls);
                Self::collect_calls_from_expr_dce(value, calls);
            }
            Stmt::Print(e) | Stmt::Println(e) | Stmt::PrintNum(e) | Stmt::Free(e) => {
                Self::collect_calls_from_expr_dce(e, calls);
            }
            _ => {}
        }
    }

    fn collect_calls_from_expr_dce(expr: &Expr, calls: &mut Vec<String>) {
        match expr {
            Expr::Call { name, args } => {
                calls.push(name.clone());
                for a in args { Self::collect_calls_from_expr_dce(a, calls); }
            }
            Expr::MethodCall { object, args, .. } => {
                Self::collect_calls_from_expr_dce(object, calls);
                for a in args { Self::collect_calls_from_expr_dce(a, calls); }
            }
            Expr::BinaryOp { left, right, .. } | Expr::Comparison { left, right, .. }
            | Expr::BitwiseOp { left, right, .. } | Expr::StringConcat { left, right } => {
                Self::collect_calls_from_expr_dce(left, calls);
                Self::collect_calls_from_expr_dce(right, calls);
            }
            Expr::UnaryOp { expr, .. } | Expr::Deref(expr) | Expr::AddressOf(expr)
            | Expr::Cast { expr, .. } | Expr::IntCast(expr) | Expr::FloatCast(expr)
            | Expr::PreIncrement(expr) | Expr::PreDecrement(expr)
            | Expr::PostIncrement(expr) | Expr::PostDecrement(expr)
            | Expr::BitwiseNot(expr) | Expr::Malloc(expr) => {
                Self::collect_calls_from_expr_dce(expr, calls);
            }
            Expr::Index { object, index } => {
                Self::collect_calls_from_expr_dce(object, calls);
                Self::collect_calls_from_expr_dce(index, calls);
            }
            Expr::FieldAccess { object, .. } => Self::collect_calls_from_expr_dce(object, calls),
            Expr::ArrowAccess { pointer, .. } => Self::collect_calls_from_expr_dce(pointer, calls),
            Expr::Ternary { condition, then_expr, else_expr } => {
                Self::collect_calls_from_expr_dce(condition, calls);
                Self::collect_calls_from_expr_dce(then_expr, calls);
                Self::collect_calls_from_expr_dce(else_expr, calls);
            }
            Expr::Array(elems) => {
                for e in elems { Self::collect_calls_from_expr_dce(e, calls); }
            }
            Expr::Realloc { ptr, new_size } => {
                Self::collect_calls_from_expr_dce(ptr, calls);
                Self::collect_calls_from_expr_dce(new_size, calls);
            }
            Expr::New { args, .. } => {
                for a in args { Self::collect_calls_from_expr_dce(a, calls); }
            }
            _ => {} // Leaf nodes: Number, Float, String, Variable, etc.
        }
    }

    // ========================================
    // Recolección de strings
    // ========================================

    fn collect_all_strings(&mut self, program: &Program) {
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

    fn collect_strings_from_expr(&mut self, expr: &Expr) {
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

    fn collect_strings_from_stmts(&mut self, stmts: &[Stmt]) {
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
                Stmt::DerefAssign { pointer, value } => {
                    self.collect_strings_from_expr(pointer);
                    self.collect_strings_from_expr(value);
                }
                _ => {}
            }
        }
    }

    fn generate_data_section(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for s in &self.strings {
            data.extend_from_slice(s.as_bytes());
            data.push(0);
        }
        // Append global variable data after strings
        if !self.global_data.is_empty() {
            // Align to 8 bytes
            while data.len() % 8 != 0 {
                data.push(0);
            }
            // The global_vars offsets are relative to this point
            data.extend_from_slice(&self.global_data);
        }
        data
    }

    /// Calculate the absolute address of a global variable
    fn get_global_address(&self, name: &str) -> Option<u64> {
        if let Some(&offset) = self.global_vars.get(name) {
            // Global data starts after strings (aligned to 8)
            let strings_size: u64 = self.strings.iter().map(|s| s.len() as u64 + 1).sum();
            let aligned_strings = (strings_size + 7) & !7;
            Some(self.base_address + self.data_rva + aligned_strings + offset as u64)
        } else {
            None
        }
    }

    /// Allocate a global variable slot and return its offset
    fn alloc_global(&mut self, name: &str, init_value: i64) -> u32 {
        let offset = self.global_offset;
        self.global_vars.insert(name.to_string(), offset);
        self.global_data
            .extend_from_slice(&init_value.to_le_bytes());
        self.global_offset += 8;
        offset
    }

    /// Scan function body for `static` local variable declarations.
    /// Since the C parser converts `static int x = N;` to a regular VarDecl,
    /// we detect static locals by checking if the VarDecl name matches a
    /// convention. For now, static locals are handled via the C frontend
    /// converting them to global-like storage with mangled names.
    fn scan_static_locals(&mut self, _stmt: &Stmt, _func_name: &str) {
        // Static locals require AST support (StaticVar variant).
        // Currently the C parser strips `static` qualifier.
        // TODO: Add StaticVar to AST when needed.
    }

    /// Emit code to load a global variable's value into RAX
    fn emit_load_global(&mut self, name: &str) {
        if let Some(addr) = self.get_global_address(name) {
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Imm64(addr),
            });
            // Load value at [RAX] into RAX
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Mem {
                    base: Reg::RAX,
                    disp: 0,
                },
            });
        }
    }

    /// Emit code to store RAX into a global variable
    fn emit_store_global(&mut self, name: &str) {
        if let Some(addr) = self.get_global_address(name) {
            self.ir.emit(ADeadOp::Push {
                src: Operand::Reg(Reg::RAX),
            });
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBX),
                src: Operand::Imm64(addr),
            });
            self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Mem {
                    base: Reg::RBX,
                    disp: 0,
                },
                src: Operand::Reg(Reg::RAX),
            });
        }
    }

    fn get_string_address(&self, s: &str) -> u64 {
        if let Some(&offset) = self.string_offsets.get(s) {
            self.base_address + self.data_rva + offset
        } else {
            self.base_address + self.data_rva
        }
    }

    // ========================================
    // Compilación de funciones
    // ========================================

    fn compile_function(&mut self, func: &Function) {
        self.current_function = Some(func.name.clone());
        self.variables.clear();
        self.variable_types.clear();
        self.array_vars.clear();
        self.param_vars.clear();
        self.struct_params.clear();
        self.ref_vars.clear();
        // Start at -32 because prologue pushes 4 callee-saved regs after mov rbp,rsp
        // occupying [rbp-8], [rbp-16], [rbp-24], [rbp-32]
        // With decrement-first convention, first var will be at -40
        self.stack_offset = -32;

        let is_interrupt = func.attributes.is_interrupt;
        let is_exception = func.attributes.is_exception;
        let is_naked = func.attributes.is_naked;

        // Label de entrada
        if let Some(compiled) = self.functions.get(&func.name) {
            let label = compiled.label;
            self.ir.emit(ADeadOp::Label(label));
        }

        if is_interrupt || is_exception {
            // @interrupt / @exception: push all registers (auto-generated wrapper)
            self.emit_interrupt_prologue();
        } else if !is_naked {
            // Normal function prologue
            self.emit_prologue();

            // Register and save parameters (MSVC x64: RCX, RDX, R8, R9)
            for (i, param) in func.params.iter().enumerate() {
                let param_offset = if i <= 3 {
                    self.stack_offset -= 8;
                    let off = self.stack_offset;
                    off
                } else {
                    16 + ((i - 4) as i32 * 8)
                };
                self.variables.insert(param.name.clone(), param_offset);
                self.variable_types
                    .insert(param.name.clone(), param.param_type.clone());

                // Save register params to stack
                if i <= 3 {
                    let src_reg = match i {
                        0 => Reg::RCX,
                        1 => Reg::RDX,
                        2 => Reg::R8,
                        _ => Reg::R9,
                    };
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp: param_offset,
                        },
                        src: Operand::Reg(src_reg),
                    });
                }

                // Mark as function parameter (uses 8-byte stride for pointer indexing)
                self.param_vars.insert(param.name.clone());
                // Mark struct parameters for pointer-based field access
                if matches!(
                    &param.param_type,
                    Type::Struct(_) | Type::Named(_) | Type::Class(_)
                ) {
                    self.struct_params.insert(param.name.clone());
                }
                // Mark reference parameters for auto-deref
                if matches!(&param.param_type, Type::Reference(_)) {
                    self.ref_vars.insert(param.name.clone());
                }
            }
        }
        // @naked: no prologue at all

        // Body
        for stmt in &func.body {
            self.emit_statement(stmt);
        }

        if is_interrupt || is_exception {
            // @interrupt / @exception: pop all registers + iretq
            self.emit_interrupt_epilogue();
        } else if !is_naked {
            // Patch prologue with actual stack frame size
            self.patch_prologue();
            // Normal function epilogue
            self.emit_epilogue();
        }
        // @naked: no epilogue at all

        self.current_function = None;
    }

    fn compile_top_level(&mut self, stmts: &[Stmt]) {
        self.current_function = Some("__entry".to_string());
        self.variables.clear();
        self.variable_types.clear();
        self.array_vars.clear();
        self.param_vars.clear();
        // Start at -32 because prologue pushes 4 callee-saved regs after mov rbp,rsp
        // With decrement-first convention, first var will be at -40
        self.stack_offset = -32;

        // For bare-metal (Target::Raw), emit instructions directly — no prologue/epilogue.
        // Boot sectors and flat binaries need raw machine code, not 64-bit function frames.
        let is_raw = self.target == Target::Raw;

        if !is_raw {
            self.emit_prologue();
        }

        for stmt in stmts {
            self.emit_statement(stmt);
        }

        if !is_raw {
            self.patch_prologue();
            self.emit_epilogue();
        }
        self.current_function = None;
    }

    // ========================================
    // Prologue / Epilogue
    // ========================================

    fn emit_prologue(&mut self) {
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RBP),
        });
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RBP),
            src: Operand::Reg(Reg::RSP),
        });
        // Save callee-saved registers used by TempAllocator (RBX, R12)
        // Windows x64 ABI requires these to be preserved across calls
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RBX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R12),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RSI),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RDI),
        });
        // Dynamic stack frame: emit placeholder, patch after function body
        self.prologue_sub_index = Some(self.ir.len());
        self.ir.emit(ADeadOp::Sub {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm32(0), // placeholder — patched in patch_prologue()
        });
        // Reset temp allocator for this function
        self.temp_alloc = TempAllocator::new();
    }

    /// Patch the prologue's sub rsp with the actual stack frame size
    fn patch_prologue(&mut self) {
        if let Some(idx) = self.prologue_sub_index.take() {
            // Calculate actual frame size: locals + shadow space (32 for Windows) + alignment
            let locals_size = (-self.stack_offset) as i32; // stack_offset is negative
            let shadow_space = if self.target == Target::Windows {
                32
            } else {
                0
            };
            let raw_size = locals_size + shadow_space;
            // Align to 16 bytes (required by x64 ABI)
            let aligned_size = ((raw_size + 15) / 16) * 16;
            // Minimum 32 bytes for small functions (Windows shadow space)
            let final_size = if aligned_size < 32 { 32 } else { aligned_size };

            if let Some(op) = self.ir.ops_mut().get_mut(idx) {
                *op = ADeadOp::Sub {
                    dst: Operand::Reg(Reg::RSP),
                    src: Operand::Imm32(final_size),
                };
            }
        }
    }

    fn emit_epilogue(&mut self) {
        // Restore RSP to point before callee-saved pushes
        // We pushed RBP, RBX, R12, RSI, RDI after mov rbp,rsp
        // So lea rsp, [rbp-32] points to where RDI was pushed
        self.ir.emit(ADeadOp::Lea {
            dst: Reg::RSP,
            src: Operand::Mem {
                base: Reg::RBP,
                disp: -32,
            },
        });
        // Restore callee-saved registers (reverse order of prologue)
        self.ir.emit(ADeadOp::Pop { dst: Reg::RDI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RSI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R12 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBP });
        self.ir.emit(ADeadOp::Ret);
    }

    // ========================================
    // Interrupt Prologue / Epilogue
    // ========================================

    fn emit_interrupt_prologue(&mut self) {
        // Push all general purpose registers (64-bit)
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RAX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RBX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RCX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RDX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RSI),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RDI),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RBP),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R8),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R9),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R10),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R11),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R12),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R13),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R14),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R15),
        });
    }

    fn emit_interrupt_epilogue(&mut self) {
        // Pop all general purpose registers (reverse order)
        self.ir.emit(ADeadOp::Pop { dst: Reg::R15 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R14 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R13 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R12 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R11 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R10 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R9 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R8 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBP });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RDI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RSI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RDX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
        // IRETQ — return from interrupt
        self.ir.emit(ADeadOp::Iret);
    }

    // ========================================
    // Statements
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
            Stmt::Return(expr) => self.emit_return(expr.as_ref()),
            Stmt::Expr(expr) => {
                self.emit_expression(expr);
            }
            Stmt::Pass => {}

            // ========== OS-LEVEL / MACHINE CODE (v3.1-OS) ==========
            Stmt::Cli => {
                self.ir.emit(ADeadOp::Cli);
            }
            Stmt::Sti => {
                self.ir.emit(ADeadOp::Sti);
            }
            Stmt::Hlt => {
                self.ir.emit(ADeadOp::Hlt);
            }
            Stmt::Iret => {
                self.ir.emit(ADeadOp::Iret);
            }
            Stmt::Cpuid => {
                self.ir.emit(ADeadOp::Cpuid);
            }
            Stmt::IntCall { vector } => {
                self.ir.emit(ADeadOp::Int { vector: *vector });
            }
            Stmt::RegAssign { reg_name, value } => {
                self.emit_reg_assign(reg_name, value);
            }
            Stmt::MemWrite { addr, value } => {
                self.emit_mem_write(addr, value);
            }
            Stmt::PortOut { port, value } => {
                self.emit_port_out(port, value);
            }
            Stmt::RawBlock { bytes } => {
                self.ir.emit(ADeadOp::RawBytes(bytes.clone()));
            }
            Stmt::OrgDirective { address } => {
                // Store origin for address calculations
                self.base_address = *address;
            }
            Stmt::AlignDirective { alignment } => {
                // Emit NOP padding to align to boundary
                let align = *alignment as usize;
                if align > 0 {
                    // We'll emit a placeholder; actual alignment resolved at link time
                    // For flat binary, pad with NOPs
                    self.ir.emit(ADeadOp::RawBytes(vec![0x90])); // marker NOP
                }
            }
            Stmt::FarJump { selector, offset } => {
                self.ir.emit(ADeadOp::FarJmp {
                    selector: *selector,
                    offset: *offset,
                });
            }

            // ========== DEREF ASSIGN: *ptr = value ==========
            Stmt::DerefAssign { pointer, value } => {
                // Evaluate value first → RAX
                self.emit_expression(value);
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                // Evaluate pointer → RAX (address)
                self.emit_expression(pointer);
                // RAX = address, pop value into RBX
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                // Store value at [pointer]: *ptr = value
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Mem {
                        base: Reg::RBX,
                        disp: 0,
                    },
                    src: Operand::Reg(Reg::RAX),
                });
            }

            // ========== ARROW ASSIGN: ptr->field = value ==========
            Stmt::ArrowAssign {
                pointer,
                field,
                value,
            } => {
                let field_offset = self.get_field_offset(field);
                // Evaluate value first → push to stack
                self.emit_expression(value);
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                // Evaluate pointer → RAX
                self.emit_expression(pointer);
                // RAX = pointer, stack top = value
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                // Store value at [RBX + field_offset]
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Mem {
                        base: Reg::RBX,
                        disp: field_offset,
                    },
                    src: Operand::Reg(Reg::RAX),
                });
            }

            // ========== FREE: free(ptr) ==========
            Stmt::Free(expr) => {
                // Evaluate pointer → RCX (first arg, Windows x64 ABI)
                self.emit_expression(expr);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RCX),
                    src: Operand::Reg(Reg::RAX),
                });
                // Call free via dynamic IAT lookup
                self.emit_call_iat("free");
            }

            // OOP field assignment: self.field = value or obj.field = value (GCC/LLVM ABI)
            Stmt::FieldAssign {
                object,
                field,
                value,
            } => {
                // Check if object is 'this' pointer - need indirect access
                if let Expr::Variable(name) = object {
                    if name == "this" {
                        // this->field = value: load this pointer, store value at offset
                        let field_offset = self
                            .variable_types
                            .get("this")
                            .and_then(|ty| match ty {
                                Type::Pointer(inner) => match inner.as_ref() {
                                    Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                        Some(self.get_class_field_offset(n, field))
                                    }
                                    _ => None,
                                },
                                _ => None,
                            })
                            .unwrap_or_else(|| self.get_field_offset(field));
                        // Evaluate value first
                        self.emit_expression(value);
                        self.ir.emit(ADeadOp::Push {
                            src: Operand::Reg(Reg::RAX),
                        });
                        // Load this pointer
                        if let Some(&offset) = self.variables.get("this") {
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RBX),
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: offset,
                                },
                            });
                        }
                        // Pop value and store at [this + field_offset]
                        self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Mem {
                                base: Reg::RBX,
                                disp: field_offset,
                            },
                            src: Operand::Reg(Reg::RAX),
                        });
                        return;
                    }
                }
                // Handle indexed struct: arr[i].field = value
                if let Expr::Index {
                    object: arr_obj,
                    index: idx,
                } = object
                {
                    if let Expr::Variable(arr_name) = arr_obj.as_ref() {
                        if let Some(&base_offset) = self.variables.get(arr_name.as_str()) {
                            // Get struct-aware field offset from array's inner type
                            let field_offset =
                                if let Some(ty) = self.variable_types.get(arr_name.as_str()) {
                                    match ty {
                                        Type::Array(inner, _) => match inner.as_ref() {
                                            Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                                self.get_class_field_offset(n, field)
                                            }
                                            _ => self.get_field_offset(field),
                                        },
                                        _ => self.get_field_offset(field),
                                    }
                                } else {
                                    self.get_field_offset(field)
                                };
                            // Evaluate value → RAX
                            self.emit_expression(value);
                            self.ir.emit(ADeadOp::Push {
                                src: Operand::Reg(Reg::RAX),
                            });
                            // Evaluate index → RAX
                            self.emit_expression(idx);
                            // Compute address: base + index * struct_stride + field_offset
                            // Get struct type from variable_types to determine stride
                            let stride = if let Some(ty) =
                                self.variable_types.get(arr_name.as_str())
                            {
                                match ty {
                                    Type::Array(inner, _) => match inner.as_ref() {
                                        Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                            self.class_layouts.get(n).map(|l| l.size).unwrap_or(8)
                                        }
                                        _ => 8,
                                    },
                                    _ => 8,
                                }
                            } else {
                                8
                            };
                            // RAX = index, multiply by stride
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RBX),
                                src: Operand::Imm32(stride),
                            });
                            self.ir.emit(ADeadOp::Mul {
                                dst: Reg::RAX,
                                src: Reg::RBX,
                            });
                            // RAX = index * stride, add base address
                            self.ir.emit(ADeadOp::Lea {
                                dst: Reg::RBX,
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: base_offset,
                                },
                            });
                            self.ir.emit(ADeadOp::Add {
                                dst: Operand::Reg(Reg::RBX),
                                src: Operand::Reg(Reg::RAX),
                            });
                            // RBX = &arr[index], pop value and store at [RBX + field_offset]
                            self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Mem {
                                    base: Reg::RBX,
                                    disp: field_offset,
                                },
                                src: Operand::Reg(Reg::RAX),
                            });
                            return;
                        }
                    }
                }
                // Regular field assignment: obj.field = value
                // Handle nested struct: r.origin.x = value
                let var_name = match object {
                    Expr::This => format!("self.{}", field),
                    Expr::Variable(obj_name) => format!("{}.{}", obj_name, field),
                    Expr::FieldAccess {
                        object: inner_obj,
                        field: inner_field,
                    } => match inner_obj.as_ref() {
                        Expr::Variable(obj_name) => {
                            format!("{}.{}.{}", obj_name, inner_field, field)
                        }
                        _ => format!("__field.{}.{}", inner_field, field),
                    },
                    _ => format!("__field.{}", field),
                };
                self.emit_assign(&var_name, value);
            }

            // ========== VARIABLE DECLARATION (C/C++ structs, arrays, scalars) ==========
            Stmt::VarDecl {
                var_type,
                name,
                value,
            } => {
                self.variable_types.insert(name.clone(), var_type.clone());

                // Determine variable category and allocation
                let is_array = matches!(var_type, Type::Array(_, _));
                // Only treat as struct if we have an actual layout for it
                // Typedefs like HINSTANCE (void*), HRESULT (long) are Named but are scalars
                let is_struct = match var_type {
                    Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                        self.class_layouts.contains_key(n)
                    }
                    _ => false,
                };

                if is_array {
                    // If this array var was already registered as a struct field (e.g., c.data),
                    // don't re-allocate — just mark it as an array for correct IndexAssign
                    if self.variables.contains_key(name.as_str()) {
                        self.array_vars.insert(name.clone());
                        // Initialize with value if provided
                        if let Some(Expr::Array(elements)) = value {
                            let base = self.variables[name.as_str()];
                            for (i, elem) in elements.iter().enumerate() {
                                self.emit_expression(elem);
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Mem {
                                        base: Reg::RBP,
                                        disp: base + (i as i32 * 8),
                                    },
                                    src: Operand::Reg(Reg::RAX),
                                });
                            }
                        }
                        return;
                    }
                    // ARRAY: allocate contiguous slots
                    let count = match var_type {
                        Type::Array(_, Some(n)) => *n as i32,
                        _ => {
                            if let Some(Expr::Array(elems)) = value {
                                elems.len() as i32
                            } else {
                                8
                            }
                        }
                    };
                    // For arrays of structs, each element takes the struct's full size
                    let elem_size = match var_type {
                        Type::Array(inner, _) => match inner.as_ref() {
                            Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                self.class_layouts.get(n).map(|l| l.size).unwrap_or(8)
                            }
                            _ => 8,
                        },
                        _ => 8,
                    };
                    let arr_size = count * elem_size;
                    self.stack_offset -= arr_size;
                    let base = self.stack_offset;
                    self.variables.insert(name.clone(), base);
                    self.array_vars.insert(name.clone());

                    // Initialize array
                    if let Some(Expr::Array(elements)) = value {
                        for (i, elem) in elements.iter().enumerate() {
                            self.emit_expression(elem);
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: base + (i as i32 * 8),
                                },
                                src: Operand::Reg(Reg::RAX),
                            });
                        }
                    } else {
                        // Zero-initialize (use total qwords, not element count)
                        let qwords = arr_size / 8;
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                        for i in 0..qwords {
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: base + (i * 8),
                                },
                                src: Operand::Reg(Reg::RAX),
                            });
                        }
                    }
                } else if is_struct {
                    // STRUCT/CLASS: allocate space and register fields
                    let struct_name = match var_type {
                        Type::Struct(n) | Type::Named(n) | Type::Class(n) => n.clone(),
                        _ => String::new(),
                    };

                    // Get layout from class_layouts (registered during compile phase 0)
                    let (struct_size, fields) =
                        if let Some(layout) = self.class_layouts.get(&struct_name) {
                            (layout.size, layout.fields.clone())
                        } else {
                            // Fallback: common C struct fields
                            (16, vec![("x".to_string(), 0), ("y".to_string(), 8)])
                        };

                    // Allocate stack space
                    self.stack_offset -= struct_size;
                    let base = self.stack_offset;
                    self.variables.insert(name.clone(), base);

                    // Register each field as "var.field" with correct offset
                    // Also register nested struct fields (e.g., r.origin.x for struct Rect)
                    for (field_name, field_off) in &fields {
                        let flat_name = format!("{}.{}", name, field_name);
                        self.variables.insert(flat_name.clone(), base + field_off);
                        // If this field is an array type, mark it in array_vars
                        // so IndexAssign uses direct RBP-relative access instead of pointer deref
                        if let Some(vt) = self.variable_types.get(&flat_name) {
                            if matches!(vt, Type::Array(_, _)) {
                                self.array_vars.insert(flat_name.clone());
                            }
                        }
                        // Check if this field name matches a known struct type
                        // by looking up field_types stored in the layout
                        if let Some(field_type_name) =
                            self.class_layouts.get(&struct_name).and_then(|layout| {
                                layout
                                    .field_types
                                    .iter()
                                    .find(|(fn_, _)| fn_ == field_name)
                                    .map(|(_, ft)| ft.clone())
                            })
                        {
                            if let Some(sub_layout) =
                                self.class_layouts.get(&field_type_name).cloned()
                            {
                                for (sub_field, sub_off) in &sub_layout.fields {
                                    self.variables.insert(
                                        format!("{}.{}.{}", name, field_name, sub_field),
                                        base + field_off + sub_off,
                                    );
                                }
                            }
                        }
                    }

                    // Check if initializer is a function call returning a packed struct
                    let is_call_init = matches!(value, Some(Expr::Call { .. }));

                    if is_call_init && fields.len() == 2 {
                        // Struct initialized from function call: emit call, unpack RAX
                        if let Some(val) = value {
                            self.emit_expression(val);
                        }
                        // RAX has packed struct: [field1:32 | field0:32]
                        // Unpack field0 (low 32 bits)
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RBX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Imm64(0xFFFFFFFF),
                        });
                        self.ir.emit(ADeadOp::And {
                            dst: Reg::RBX,
                            src: Reg::RCX,
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: base + fields[0].1,
                            },
                            src: Operand::Reg(Reg::RBX),
                        });
                        // Unpack field1 (high 32 bits)
                        self.ir.emit(ADeadOp::Shr {
                            dst: Reg::RAX,
                            amount: 32,
                        });
                        self.ir.emit(ADeadOp::And {
                            dst: Reg::RAX,
                            src: Reg::RCX,
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: base + fields[1].1,
                            },
                            src: Operand::Reg(Reg::RAX),
                        });
                    } else {
                        // Zero-initialize struct
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                        for i in 0..(struct_size / 8) {
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: base + (i * 8),
                                },
                                src: Operand::Reg(Reg::RAX),
                            });
                        }
                    }
                } else {
                    // SCALAR: single 8-byte slot
                    // If already registered (e.g., struct field like c.size), reuse existing offset
                    let offset = if let Some(&existing) = self.variables.get(name.as_str()) {
                        existing
                    } else {
                        self.stack_offset -= 8;
                        let off = self.stack_offset;
                        self.variables.insert(name.clone(), off);
                        off
                    };

                    if let Some(val) = value {
                        self.emit_expression(val);
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                            src: Operand::Reg(Reg::RAX),
                        });
                    } else {
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                            src: Operand::Reg(Reg::RAX),
                        });
                    }
                }
            }
            Stmt::CompoundAssign { name, op, value } => {
                self.emit_compound_assign(name, op, value);
            }
            // ========== ARRAY INDEX ASSIGNMENT: arr[i] = value ==========
            Stmt::IndexAssign {
                object,
                index,
                value,
            } => {
                // Handle this->field[index] = value (ArrowAccess on array field through pointer)
                if let Expr::ArrowAccess { pointer, field } = object {
                    let field_offset = if let Expr::Variable(ptr_name) = pointer.as_ref() {
                        if let Some(ty) = self.variable_types.get(ptr_name) {
                            match ty {
                                Type::Pointer(inner) => match inner.as_ref() {
                                    Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                        self.get_class_field_offset(n, field)
                                    }
                                    _ => self.get_field_offset(field),
                                },
                                Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                    self.get_class_field_offset(n, field)
                                }
                                _ => self.get_field_offset(field),
                            }
                        } else {
                            self.get_field_offset(field)
                        }
                    } else {
                        self.get_field_offset(field)
                    };
                    // Evaluate value first
                    self.emit_expression(value);
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    // Evaluate index
                    self.emit_expression(index);
                    self.ir.emit(ADeadOp::Shl {
                        dst: Reg::RAX,
                        amount: 3,
                    }); // i*8
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    // Load pointer (this)
                    self.emit_expression(pointer);
                    // RAX = pointer; compute pointer + field_offset + i*8
                    if field_offset != 0 {
                        self.ir.emit(ADeadOp::Add {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Imm32(field_offset),
                        });
                    }
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RBX }); // i*8
                    self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    // Store value at [pointer + field_offset + i*8]
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RCX }); // value
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RAX,
                            disp: 0,
                        },
                        src: Operand::Reg(Reg::RCX),
                    });
                } else if let Expr::Variable(name) = object {
                    if let Some(&base_offset) = self.variables.get(name.as_str()) {
                        let is_local_array = self.array_vars.contains(name.as_str());

                        if is_local_array {
                            // LOCAL ARRAY: ascending layout, arr[i] at base + i*8
                            if let Expr::Number(idx) = index {
                                let elem_offset = base_offset + (*idx as i32 * 8);
                                self.emit_expression(value);
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Mem {
                                        base: Reg::RBP,
                                        disp: elem_offset,
                                    },
                                    src: Operand::Reg(Reg::RAX),
                                });
                            } else {
                                // Dynamic index for local array
                                self.emit_expression(value);
                                self.ir.emit(ADeadOp::Push {
                                    src: Operand::Reg(Reg::RAX),
                                });
                                self.emit_expression(index);
                                self.ir.emit(ADeadOp::Shl {
                                    dst: Reg::RAX,
                                    amount: 3,
                                });
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RBX),
                                    src: Operand::Reg(Reg::RAX),
                                });
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Imm32(base_offset),
                                });
                                self.ir.emit(ADeadOp::Add {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Reg(Reg::RBX),
                                });
                                self.ir.emit(ADeadOp::Add {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Reg(Reg::RBP),
                                });
                                self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Mem {
                                        base: Reg::RAX,
                                        disp: 0,
                                    },
                                    src: Operand::Reg(Reg::RCX),
                                });
                            }
                        } else {
                            // POINTER VARIABLE (e.g. int *arr parameter):
                            // Load pointer, then store at [ptr + i*stride]
                            let stride = self.element_stride(name);
                            self.emit_expression(value);
                            self.ir.emit(ADeadOp::Push {
                                src: Operand::Reg(Reg::RAX),
                            });
                            self.emit_expression(index);
                            self.emit_index_scale(stride);
                            self.ir.emit(ADeadOp::Push {
                                src: Operand::Reg(Reg::RAX),
                            });
                            // Load pointer value
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RAX),
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: base_offset,
                                },
                            });
                            self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
                            // ptr + i*stride
                            self.ir.emit(ADeadOp::Add {
                                dst: Operand::Reg(Reg::RAX),
                                src: Operand::Reg(Reg::RBX),
                            });
                            // Store value at [ptr + i*stride]
                            self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Mem {
                                    base: Reg::RAX,
                                    disp: 0,
                                },
                                src: Operand::Reg(Reg::RCX),
                            });
                        }
                    } else {
                        // Unknown variable - skip
                    }
                } else {
                    // Non-variable object - evaluate as pointer
                    self.emit_expression(value);
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.emit_expression(index);
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.emit_expression(object);
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
                    self.ir.emit(ADeadOp::Shl {
                        dst: Reg::RBX,
                        amount: 3,
                    });
                    self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RAX,
                            disp: 0,
                        },
                        src: Operand::Reg(Reg::RCX),
                    });
                }
            }
            Stmt::Increment {
                name,
                is_pre: _,
                is_increment,
            } => {
                if let Some(&offset) = self.variables.get(name.as_str()) {
                    if *is_increment {
                        self.ir.emit(ADeadOp::Inc {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    } else {
                        self.ir.emit(ADeadOp::Dec {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    }
                }
            }
            Stmt::DoWhile { body, condition } => {
                let loop_start = self.ir.new_label();
                let loop_end = self.ir.new_label();

                self.loop_stack.push((loop_end, loop_start));

                self.ir.emit(ADeadOp::Label(loop_start));

                for s in body {
                    self.emit_statement(s);
                }

                self.emit_expression(condition);
                self.ir.emit(ADeadOp::Cmp {
                    left: Operand::Reg(Reg::RAX),
                    right: Operand::Imm32(0),
                });
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::NotEqual,
                    target: loop_start,
                });
                self.ir.emit(ADeadOp::Label(loop_end));

                self.loop_stack.pop();
            }

            // ========== LABELS Y JUMPS (v3.3-Boot) ==========
            Stmt::LabelDef { name } => {
                let label = self.get_or_create_named_label(name);
                self.ir.emit(ADeadOp::Label(label));
            }
            Stmt::JumpTo { label: label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jmp { target: label });
            }
            Stmt::JumpIfZero { label: label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::Equal,
                    target: label,
                });
            }
            Stmt::JumpIfNotZero { label: label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::NotEqual,
                    target: label,
                });
            }
            Stmt::JumpIfCarry { label: label_name } => {
                // JC = Jump if Carry (CF=1) — use raw bytes: 0x72 rel8
                let label = self.get_or_create_named_label(label_name);
                // For now, emit as conditional jump placeholder
                // The encoder will need to handle carry flag jumps
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::Less,
                    target: label,
                });
            }
            Stmt::JumpIfNotCarry { label: label_name } => {
                // JNC = Jump if Not Carry (CF=0) — use raw bytes: 0x73 rel8
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::GreaterEq,
                    target: label,
                });
            }
            Stmt::DataBytes { bytes } => {
                self.ir.emit(ADeadOp::RawBytes(bytes.clone()));
            }
            Stmt::DataWords { words } => {
                let mut bytes = Vec::new();
                for w in words {
                    bytes.extend_from_slice(&w.to_le_bytes());
                }
                self.ir.emit(ADeadOp::RawBytes(bytes));
            }
            Stmt::DataDwords { dwords } => {
                let mut bytes = Vec::new();
                for d in dwords {
                    bytes.extend_from_slice(&d.to_le_bytes());
                }
                self.ir.emit(ADeadOp::RawBytes(bytes));
            }
            Stmt::TimesDirective { count, byte } => {
                let bytes = vec![*byte; *count];
                self.ir.emit(ADeadOp::RawBytes(bytes));
            }

            Stmt::Break => {
                if let Some(&(break_label, _)) = self.loop_stack.last() {
                    self.ir.emit(ADeadOp::Jmp {
                        target: break_label,
                    });
                }
            }
            Stmt::Continue => {
                if let Some(&(_, continue_label)) = self.loop_stack.last() {
                    self.ir.emit(ADeadOp::Jmp {
                        target: continue_label,
                    });
                }
            }

            _ => {}
        }
    }

    // ========================================
    // Named Labels (v3.3-Boot)
    // ========================================

    /// Get or create a named label. If the label already exists, return it.
    /// Otherwise, create a new label and store it in the named_labels map.
    fn get_or_create_named_label(&mut self, name: &str) -> Label {
        if let Some(&label) = self.named_labels.get(name) {
            label
        } else {
            let label = self.ir.new_label();
            self.named_labels.insert(name.to_string(), label);
            label
        }
    }

    // ========================================
    // OS-Level Helpers
    // ========================================

    fn string_to_reg(name: &str) -> Option<Reg> {
        match name {
            "rax" => Some(Reg::RAX),
            "rbx" => Some(Reg::RBX),
            "rcx" => Some(Reg::RCX),
            "rdx" => Some(Reg::RDX),
            "rsi" => Some(Reg::RSI),
            "rdi" => Some(Reg::RDI),
            "rbp" => Some(Reg::RBP),
            "rsp" => Some(Reg::RSP),
            "r8" => Some(Reg::R8),
            "r9" => Some(Reg::R9),
            "r10" => Some(Reg::R10),
            "r11" => Some(Reg::R11),
            "r12" => Some(Reg::R12),
            "r13" => Some(Reg::R13),
            "r14" => Some(Reg::R14),
            "r15" => Some(Reg::R15),
            "eax" => Some(Reg::EAX),
            "ebx" => Some(Reg::EBX),
            "ecx" => Some(Reg::ECX),
            "edx" => Some(Reg::EDX),
            "esi" => Some(Reg::ESI),
            "edi" => Some(Reg::EDI),
            "esp" => Some(Reg::ESP),
            "ebp" => Some(Reg::EBP),
            "ax" => Some(Reg::AX),
            "bx" => Some(Reg::BX),
            "cx" => Some(Reg::CX),
            "dx" => Some(Reg::DX),
            "si" => Some(Reg::SI),
            "di" => Some(Reg::DI),
            "sp" => Some(Reg::SP),
            "bp" => Some(Reg::BP),
            "al" => Some(Reg::AL),
            "ah" => Some(Reg::AH),
            "bl" => Some(Reg::BL),
            "bh" => Some(Reg::BH),
            "cl" => Some(Reg::CL),
            "ch" => Some(Reg::CH),
            "dl" => Some(Reg::DL),
            "dh" => Some(Reg::DH),
            "cr0" => Some(Reg::CR0),
            "cr2" => Some(Reg::CR2),
            "cr3" => Some(Reg::CR3),
            "cr4" => Some(Reg::CR4),
            "cs" => Some(Reg::CS),
            "ds" => Some(Reg::DS),
            "es" => Some(Reg::ES),
            "fs" => Some(Reg::FS),
            "gs" => Some(Reg::GS),
            "ss" => Some(Reg::SS),
            _ => None,
        }
    }

    fn emit_reg_assign(&mut self, reg_name: &str, value: &Expr) {
        self.emit_expression(value);
        if let Some(reg) = Self::string_to_reg(reg_name) {
            if reg.is_control() {
                // mov crN, rax
                let cr_num = match reg {
                    Reg::CR0 => 0,
                    Reg::CR2 => 2,
                    Reg::CR3 => 3,
                    Reg::CR4 => 4,
                    _ => 0,
                };
                self.ir.emit(ADeadOp::MovToCr {
                    cr: cr_num,
                    src: Reg::RAX,
                });
            } else if reg.is_segment() {
                // Segment register assignment via raw bytes
                // mov <seg>, ax requires specific encoding
                let seg_code: u8 = match reg {
                    Reg::DS => 0xD8,
                    Reg::ES => 0xC0,
                    Reg::SS => 0xD0,
                    Reg::FS => 0xE0,
                    Reg::GS => 0xE8,
                    _ => 0xD8,
                };
                // 8E /r = mov Sreg, r/m16
                self.ir.emit(ADeadOp::RawBytes(vec![0x8E, seg_code]));
            } else {
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(reg),
                    src: Operand::Reg(Reg::RAX),
                });
            }
        }
    }

    fn emit_mem_write(&mut self, addr: &Expr, value: &Expr) {
        // Evaluate value → RAX, then addr → RBX, then mov [RBX], RAX
        self.emit_expression(value);
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RAX),
        });
        self.emit_expression(addr);
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RBX),
            src: Operand::Reg(Reg::RAX),
        });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
        // mov [rbx], rax
        self.ir.emit(ADeadOp::RawBytes(vec![0x48, 0x89, 0x03]));
    }

    fn emit_port_out(&mut self, port: &Expr, value: &Expr) {
        // Evaluate value → AL, port → immediate or DX
        self.emit_expression(value);
        match port {
            Expr::Number(p) if *p >= 0 && *p <= 255 => {
                self.ir.emit(ADeadOp::OutByte {
                    port: Operand::Imm8(*p as i8),
                    src: Operand::Reg(Reg::AL),
                });
            }
            _ => {
                // Port in DX
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                self.emit_expression(port);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RDX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                self.ir.emit(ADeadOp::OutByte {
                    port: Operand::Reg(Reg::DX),
                    src: Operand::Reg(Reg::AL),
                });
            }
        }
    }

    fn emit_compound_assign(&mut self, name: &str, op: &CompoundOp, value: &Expr) {
        // Load current value
        if let Some(&offset) = self.variables.get(name) {
            self.emit_expression(value);
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBX),
                src: Operand::Reg(Reg::RAX),
            });
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Mem {
                    base: Reg::RBP,
                    disp: offset,
                },
            });
            match op {
                CompoundOp::AddAssign => self.ir.emit(ADeadOp::Add {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Reg(Reg::RBX),
                }),
                CompoundOp::SubAssign => self.ir.emit(ADeadOp::Sub {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Reg(Reg::RBX),
                }),
                CompoundOp::MulAssign => self.ir.emit(ADeadOp::Mul {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                }),
                CompoundOp::DivAssign => self.ir.emit(ADeadOp::Div { src: Reg::RBX }),
                CompoundOp::AndAssign => self.ir.emit(ADeadOp::And {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                }),
                CompoundOp::OrAssign => self.ir.emit(ADeadOp::Or {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                }),
                CompoundOp::XorAssign => self.ir.emit(ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                }),
                CompoundOp::ShlAssign => {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RCX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    self.ir.emit(ADeadOp::ShlCl { dst: Reg::RAX });
                }
                CompoundOp::ShrAssign => {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RCX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    self.ir.emit(ADeadOp::ShrCl { dst: Reg::RAX });
                }
                CompoundOp::ModAssign => {
                    self.ir.emit(ADeadOp::Div { src: Reg::RBX });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RDX),
                    });
                }
            }
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Mem {
                    base: Reg::RBP,
                    disp: offset,
                },
                src: Operand::Reg(Reg::RAX),
            });
        }
    }

    // ========================================
    // Print / Println
    // ========================================

    fn emit_print(&mut self, expr: &Expr) {
        if let Expr::String(s) = expr {
            let processed = s
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r");
            if !self.strings.contains(&processed) {
                self.strings.push(processed.clone());
            }
            let string_addr = self.get_string_address(&processed);

            match self.target {
                Target::Linux => {
                    // sys_write(1, buf, len)
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Imm32(1),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDI),
                        src: Operand::Imm32(1),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RSI),
                        src: Operand::Imm64(string_addr),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDX),
                        src: Operand::Imm32(processed.len() as i32),
                    });
                    self.ir.emit(ADeadOp::Syscall);
                }
                Target::Windows | Target::Raw => {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RCX),
                        src: Operand::Imm64(string_addr),
                    });
                    self.emit_call_printf();
                }
            }
        } else {
            self.emit_expression(expr);

            let is_float = matches!(expr, Expr::Float(_));
            let is_integer = matches!(
                expr,
                Expr::Number(_)
                    | Expr::Variable(_)
                    | Expr::BinaryOp { .. }
                    | Expr::Bool(_)
                    | Expr::Call { .. }
                    | Expr::IntCast(_)
                    | Expr::Len(_)
                    | Expr::FieldAccess { .. }
                    | Expr::ArrowAccess { .. }
                    | Expr::Index { .. }
                    | Expr::Comparison { .. }
                    | Expr::Ternary { .. }
                    | Expr::UnaryOp { .. }
                    | Expr::SizeOf(_)
                    | Expr::Deref(_)
                    | Expr::Cast { .. }
                    | Expr::BitwiseOp { .. }
                    | Expr::BitwiseNot(_)
                    | Expr::PreIncrement(_)
                    | Expr::PreDecrement(_)
                    | Expr::PostIncrement(_)
                    | Expr::PostDecrement(_)
            );

            match self.target {
                Target::Windows | Target::Raw => {
                    if is_float {
                        let fmt_addr = self.get_string_address("%.2f");
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::MovQ {
                            dst: Reg::XMM1,
                            src: Reg::RDX,
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Imm64(fmt_addr),
                        });
                        self.emit_call_printf();
                    } else if is_integer {
                        let fmt_addr = self.get_string_address("%d");
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Imm64(fmt_addr),
                        });
                        self.emit_call_printf();
                    } else {
                        let fmt_addr = self.get_string_address("%s");
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Imm64(fmt_addr),
                        });
                        self.emit_call_printf();
                    }
                }
                Target::Linux => {}
            }
        }
    }

    fn emit_println(&mut self, expr: &Expr) {
        self.emit_print(expr);
        // Print newline
        let newline = "\n".to_string();
        if !self.strings.contains(&newline) {
            self.strings.push(newline);
        }
        let nl_addr = self.get_string_address("\n");
        match self.target {
            Target::Windows | Target::Raw => {
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RCX),
                    src: Operand::Imm64(nl_addr),
                });
                self.emit_call_printf();
            }
            Target::Linux => {}
        }
    }

    fn emit_print_num(&mut self, expr: &Expr) {
        self.emit_expression(expr);
        let fmt_addr = self.get_string_address("%d");
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RDX),
            src: Operand::Reg(Reg::RAX),
        });
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RCX),
            src: Operand::Imm64(fmt_addr),
        });
        self.emit_call_printf();
    }

    fn emit_call_printf(&mut self) {
        self.emit_call_iat("printf");
    }

    /// Emit a call to an IAT-imported function by name.
    /// Looks up the IAT slot RVA from the registry using assumed idata_rva=0x2000.
    fn emit_call_iat(&mut self, func_name: &str) {
        let assumed_idata_rva: u32 = 0x2000;
        let idata_result = iat_registry::build_idata(assumed_idata_rva, &[]);
        let slot = iat_registry::slot_for_function(func_name)
            .unwrap_or_else(|| panic!("IAT function not found: {}", func_name));
        let iat_rva = idata_result.slot_to_iat_rva[slot];

        self.ir.emit(ADeadOp::Sub {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm8(32),
        });
        self.ir.emit(ADeadOp::CallIAT { iat_rva });
        self.ir.emit(ADeadOp::Add {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm8(32),
        });
    }

    /// Returns the register for argument `index` based on target calling convention
    fn arg_register(&self, index: usize) -> Reg {
        match self.target {
            Target::Windows => match index {
                0 => Reg::RCX,
                1 => Reg::RDX,
                2 => Reg::R8,
                3 => Reg::R9,
                _ => Reg::RCX,
            },
            Target::Linux | Target::Raw => match index {
                0 => Reg::RDI,
                1 => Reg::RSI,
                2 => Reg::RDX,
                3 => Reg::RCX,
                _ => Reg::RDI,
            },
        }
    }

    // ========================================
    // Assign
    // ========================================

    fn emit_assign(&mut self, name: &str, value: &Expr) {
        // Reference variable: write through pointer
        if self.ref_vars.contains(name) {
            self.emit_expression(value);
            if let Some(&offset) = self.variables.get(name) {
                // RAX = value to store, load pointer into RBX, store through it
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Mem {
                        base: Reg::RBP,
                        disp: offset,
                    },
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Mem {
                        base: Reg::RBX,
                        disp: 0,
                    },
                    src: Operand::Reg(Reg::RAX),
                });
            }
            return;
        }

        // Optimización: x = x + 1 → inc, x = x - 1 → dec
        if let Some(&offset) = self.variables.get(name) {
            if let Expr::BinaryOp { op, left, right } = value {
                if let Expr::Variable(var_name) = left.as_ref() {
                    if var_name == name {
                        if let Expr::Number(n) = right.as_ref() {
                            if *n == 1 {
                                match op {
                                    BinOp::Add => {
                                        self.ir.emit(ADeadOp::Inc {
                                            dst: Operand::Mem {
                                                base: Reg::RBP,
                                                disp: offset,
                                            },
                                        });
                                        return;
                                    }
                                    BinOp::Sub => {
                                        self.ir.emit(ADeadOp::Dec {
                                            dst: Operand::Mem {
                                                base: Reg::RBP,
                                                disp: offset,
                                            },
                                        });
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

        self.emit_expression(value);

        // Check global variable first
        if self.variables.get(name).is_none() {
            // Check global or static local
            let global_name = if self.global_vars.contains_key(name) {
                Some(name.to_string())
            } else if let Some(func_name) = &self.current_function.clone() {
                let mangled = format!("{}::{}", func_name, name);
                if self.global_vars.contains_key(&mangled) {
                    Some(mangled)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(gname) = global_name {
                self.emit_store_global(&gname);
                return;
            }
        }

        let offset = if let Some(&off) = self.variables.get(name) {
            off
        } else {
            self.stack_offset -= 8;
            let off = self.stack_offset;
            self.variables.insert(name.to_string(), off);
            off
        };

        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Mem {
                base: Reg::RBP,
                disp: offset,
            },
            src: Operand::Reg(Reg::RAX),
        });
    }

    // ========================================
    // Control Flow
    // ========================================

    fn emit_if(&mut self, condition: &Expr, then_body: &[Stmt], else_body: Option<&[Stmt]>) {
        self.emit_condition(condition);
        self.ir.emit(ADeadOp::Test {
            left: Reg::RAX,
            right: Reg::RAX,
        });

        let else_label = self.ir.new_label();
        self.ir.emit(ADeadOp::Jcc {
            cond: Condition::Equal,
            target: else_label,
        });

        for stmt in then_body {
            self.emit_statement(stmt);
        }

        if let Some(else_stmts) = else_body {
            let end_label = self.ir.new_label();
            self.ir.emit(ADeadOp::Jmp { target: end_label });
            self.ir.emit(ADeadOp::Label(else_label));
            for stmt in else_stmts {
                self.emit_statement(stmt);
            }
            self.ir.emit(ADeadOp::Label(end_label));
        } else {
            self.ir.emit(ADeadOp::Label(else_label));
        }
    }

    fn emit_while(&mut self, condition: &Expr, body: &[Stmt]) {
        let loop_start = self.ir.new_label();
        let loop_end = self.ir.new_label();
        let continue_label = self.ir.new_label();

        self.loop_stack.push((loop_end, continue_label));

        self.ir.emit(ADeadOp::Label(loop_start));
        self.emit_condition(condition);
        self.ir.emit(ADeadOp::Test {
            left: Reg::RAX,
            right: Reg::RAX,
        });
        self.ir.emit(ADeadOp::Jcc {
            cond: Condition::Equal,
            target: loop_end,
        });

        // Detect for-loop pattern: if last stmt is Increment/CompoundAssign/Assign,
        // treat it as the "update" part. continue should jump to it, not skip it.
        let update_count = Self::count_trailing_update(body);
        let main_body = &body[..body.len() - update_count];
        let update_body = &body[body.len() - update_count..];

        for stmt in main_body {
            self.emit_statement(stmt);
        }

        // continue lands here — before the update, so update always runs
        self.ir.emit(ADeadOp::Label(continue_label));

        for stmt in update_body {
            self.emit_statement(stmt);
        }

        self.ir.emit(ADeadOp::Jmp { target: loop_start });
        self.ir.emit(ADeadOp::Label(loop_end));

        self.loop_stack.pop();
    }

    /// Detect how many trailing statements in a while body are "for-loop update" statements.
    /// The C frontend converts `for(init;cond;update) body` into:
    ///   init; while(cond) { body; update; }
    /// We detect the trailing update so `continue` jumps to it instead of skipping it.
    fn count_trailing_update(body: &[Stmt]) -> usize {
        if body.is_empty() {
            return 0;
        }
        let last = &body[body.len() - 1];
        match last {
            Stmt::Increment { .. } => 1,
            Stmt::CompoundAssign { .. } => 1,
            Stmt::Assign { .. } => 1,
            Stmt::Expr(_) => 1,
            _ => 0,
        }
    }

    fn emit_for(&mut self, var: &str, start: &Expr, end: &Expr, body: &[Stmt]) {
        // Evaluar start → RCX, end → R8
        self.emit_expression(start);
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RCX),
            src: Operand::Reg(Reg::RAX),
        });
        self.emit_expression(end);
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::R8),
            src: Operand::Reg(Reg::RAX),
        });

        let var_offset = self.stack_offset;
        self.variables.insert(var.to_string(), var_offset);
        self.stack_offset -= 8;

        let loop_start = self.ir.new_label();
        let loop_end = self.ir.new_label();

        self.loop_stack.push((loop_end, loop_start));

        self.ir.emit(ADeadOp::Label(loop_start));
        self.ir.emit(ADeadOp::Cmp {
            left: Operand::Reg(Reg::RCX),
            right: Operand::Reg(Reg::R8),
        });
        self.ir.emit(ADeadOp::Jcc {
            cond: Condition::GreaterEq,
            target: loop_end,
        });

        // Guardar RCX en variable
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Mem {
                base: Reg::RBP,
                disp: var_offset,
            },
            src: Operand::Reg(Reg::RCX),
        });

        // Preservar RCX y R8
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RCX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R8),
        });

        for stmt in body {
            self.emit_statement(stmt);
        }

        self.ir.emit(ADeadOp::Pop { dst: Reg::R8 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
        self.ir.emit(ADeadOp::Inc {
            dst: Operand::Reg(Reg::RCX),
        });
        self.ir.emit(ADeadOp::Jmp { target: loop_start });
        self.ir.emit(ADeadOp::Label(loop_end));

        self.loop_stack.pop();
    }

    fn emit_return(&mut self, expr: Option<&Expr>) {
        if let Some(e) = expr {
            // Check if returning a struct variable — pack fields into RAX
            if let Expr::Variable(var_name) = e {
                if let Some(ty) = self.variable_types.get(var_name).cloned() {
                    let sn = match &ty {
                        Type::Struct(n) | Type::Named(n) | Type::Class(n) => Some(n.clone()),
                        _ => None,
                    };
                    if let Some(struct_name) = sn {
                        if let Some(layout) = self.class_layouts.get(&struct_name).cloned() {
                            if layout.fields.len() == 2 {
                                // Pack 2 fields: field[0] in low 32 bits, field[1] in high 32 bits
                                let f0_off = layout.fields[0].1;
                                let f1_off = layout.fields[1].1;
                                if let Some(&base) = self.variables.get(var_name) {
                                    // Load field[1] into RAX, shift left 32
                                    self.ir.emit(ADeadOp::Mov {
                                        dst: Operand::Reg(Reg::RAX),
                                        src: Operand::Mem {
                                            base: Reg::RBP,
                                            disp: base + f1_off,
                                        },
                                    });
                                    self.ir.emit(ADeadOp::Shl {
                                        dst: Reg::RAX,
                                        amount: 32,
                                    });
                                    // Load field[0] into RBX
                                    self.ir.emit(ADeadOp::Mov {
                                        dst: Operand::Reg(Reg::RBX),
                                        src: Operand::Mem {
                                            base: Reg::RBP,
                                            disp: base + f0_off,
                                        },
                                    });
                                    // Mask to 32 bits and OR
                                    self.ir.emit(ADeadOp::Mov {
                                        dst: Operand::Reg(Reg::RCX),
                                        src: Operand::Imm64(0xFFFFFFFF),
                                    });
                                    self.ir.emit(ADeadOp::And {
                                        dst: Reg::RBX,
                                        src: Reg::RCX,
                                    });
                                    self.ir.emit(ADeadOp::Or {
                                        dst: Reg::RAX,
                                        src: Reg::RBX,
                                    });
                                    // RAX now has packed struct: [field1:32 | field0:32]
                                    // Fall through to epilogue
                                }
                            } else {
                                self.emit_expression(e);
                            }
                        } else {
                            self.emit_expression(e);
                        }
                    } else {
                        self.emit_expression(e);
                    }
                } else {
                    self.emit_expression(e);
                }
            } else {
                self.emit_expression(e);
            }
        } else {
            self.ir.emit(ADeadOp::Xor {
                dst: Reg::EAX,
                src: Reg::EAX,
            });
        }
        // Full epilogue inline: must match emit_prologue's callee-saved register saves
        // Prologue pushes: RBP, then RBX, R12, RSI, RDI (at RBP-8..-32)
        self.ir.emit(ADeadOp::Lea {
            dst: Reg::RSP,
            src: Operand::Mem {
                base: Reg::RBP,
                disp: -32,
            },
        });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RDI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RSI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R12 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBP });
        self.ir.emit(ADeadOp::Ret);
    }

    // ========================================
    // Conditions
    // ========================================

    fn emit_condition(&mut self, expr: &Expr) {
        match expr {
            Expr::Comparison { op, left, right } => {
                // Use push/pop — safe for nested calls and recursion
                self.emit_expression(left);
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                self.emit_expression(right);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });

                self.ir.emit(ADeadOp::Cmp {
                    left: Operand::Reg(Reg::RAX),
                    right: Operand::Reg(Reg::RBX),
                });

                let cond = match op {
                    CmpOp::Eq => Condition::Equal,
                    CmpOp::Ne => Condition::NotEqual,
                    CmpOp::Lt => Condition::Less,
                    CmpOp::Le => Condition::LessEq,
                    CmpOp::Gt => Condition::Greater,
                    CmpOp::Ge => Condition::GreaterEq,
                };
                self.ir.emit(ADeadOp::SetCC { cond, dst: Reg::AL });
                self.ir.emit(ADeadOp::MovZx {
                    dst: Reg::RAX,
                    src: Reg::AL,
                });
            }
            Expr::Bool(b) => {
                let val = if *b { 1 } else { 0 };
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::EAX),
                    src: Operand::Imm32(val),
                });
            }
            _ => self.emit_expression(expr),
        }
    }

    // ========================================
    // Expressions → RAX
    // ========================================

    fn emit_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Number(n) => {
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(*n as u64),
                });
            }
            Expr::Float(f) => {
                let bits = f.to_bits();
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(bits),
                });
            }
            Expr::Bool(b) => {
                let val = if *b { 1u64 } else { 0u64 };
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(val),
                });
            }
            Expr::Variable(name) => {
                if let Some(&offset) = self.variables.get(name) {
                    if self.array_vars.contains(name) {
                        // Array variable: load its ADDRESS (LEA), not its value
                        self.ir.emit(ADeadOp::Lea {
                            dst: Reg::RAX,
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    } else if self.ref_vars.contains(name) {
                        // Reference variable: auto-deref (load pointer, then load value)
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RAX,
                                disp: 0,
                            },
                        });
                    } else {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    }
                } else if self.global_vars.contains_key(name) {
                    // Global variable: load from absolute address in data section
                    self.emit_load_global(name);
                } else if let Some(func_name) = &self.current_function.clone() {
                    // Check for static local: mangled as func::var
                    let mangled = format!("{}::{}", func_name, name);
                    if self.global_vars.contains_key(&mangled) {
                        self.emit_load_global(&mangled);
                    } else if let Some(func) = self.functions.get(name) {
                        let label = func.label;
                        self.ir.emit(ADeadOp::LeaLabel {
                            dst: Reg::RAX,
                            label,
                        });
                    } else {
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::EAX,
                            src: Reg::EAX,
                        });
                    }
                } else if let Some(func) = self.functions.get(name) {
                    // Function name used as value = function pointer (address of function)
                    let label = func.label;
                    self.ir.emit(ADeadOp::LeaLabel {
                        dst: Reg::RAX,
                        label,
                    });
                } else {
                    self.ir.emit(ADeadOp::Xor {
                        dst: Reg::EAX,
                        src: Reg::EAX,
                    });
                }
            }
            Expr::BinaryOp { op, left, right } => {
                // Use push/pop to preserve left across right evaluation.
                // This is safe for all cases including nested calls and recursion.
                self.emit_expression(left);
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                self.emit_expression(right);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });

                match op {
                    BinOp::Add => self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    }),
                    BinOp::Sub => self.ir.emit(ADeadOp::Sub {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    }),
                    BinOp::Mul => self.ir.emit(ADeadOp::Mul {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BinOp::Div => self.ir.emit(ADeadOp::Div { src: Reg::RBX }),
                    BinOp::Mod => {
                        self.ir.emit(ADeadOp::Div { src: Reg::RBX });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Reg(Reg::RDX),
                        });
                    }
                    BinOp::And => self.ir.emit(ADeadOp::And {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BinOp::Or => self.ir.emit(ADeadOp::Or {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                }
            }
            Expr::UnaryOp { op, expr: inner } => {
                self.emit_expression(inner);
                match op {
                    UnaryOp::Neg => self.ir.emit(ADeadOp::Neg { dst: Reg::RAX }),
                    UnaryOp::Not => self.ir.emit(ADeadOp::Not { dst: Reg::RAX }),
                }
            }
            Expr::Call { name, args } => {
                self.emit_call(name, args);
            }
            Expr::Comparison { .. } => self.emit_condition(expr),
            Expr::Input => {
                self.emit_input();
            }
            Expr::IntCast(inner) => {
                self.emit_expression(inner);
            }
            Expr::FloatCast(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::CvtSi2Sd {
                    dst: Reg::XMM0,
                    src: Reg::RAX,
                });
                self.ir.emit(ADeadOp::MovQ {
                    dst: Reg::RAX,
                    src: Reg::XMM0,
                });
            }
            Expr::BoolCast(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::Test {
                    left: Reg::RAX,
                    right: Reg::RAX,
                });
                self.ir.emit(ADeadOp::SetCC {
                    cond: Condition::NotEqual,
                    dst: Reg::AL,
                });
                self.ir.emit(ADeadOp::MovZx {
                    dst: Reg::RAX,
                    src: Reg::AL,
                });
            }
            // OS-Level expressions
            Expr::RegRead { reg_name } => {
                if let Some(reg) = Self::string_to_reg(reg_name) {
                    if reg.is_control() {
                        let cr_num = match reg {
                            Reg::CR0 => 0,
                            Reg::CR2 => 2,
                            Reg::CR3 => 3,
                            Reg::CR4 => 4,
                            _ => 0,
                        };
                        self.ir.emit(ADeadOp::MovFromCr {
                            cr: cr_num,
                            dst: Reg::RAX,
                        });
                    } else {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Reg(reg),
                        });
                    }
                }
            }
            Expr::MemRead { addr } => {
                self.emit_expression(addr);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                // mov rax, [rbx]
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Mem {
                        base: Reg::RBX,
                        disp: 0,
                    },
                });
            }
            Expr::PortIn { port } => {
                match port.as_ref() {
                    Expr::Number(p) if *p >= 0 && *p <= 255 => {
                        self.ir.emit(ADeadOp::InByte {
                            port: Operand::Imm8(*p as i8),
                        });
                        // Result in AL, zero-extend to RAX
                        self.ir.emit(ADeadOp::MovZx {
                            dst: Reg::RAX,
                            src: Reg::AL,
                        });
                    }
                    _ => {
                        self.emit_expression(port);
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::InByte {
                            port: Operand::Reg(Reg::DX),
                        });
                        self.ir.emit(ADeadOp::MovZx {
                            dst: Reg::RAX,
                            src: Reg::AL,
                        });
                    }
                }
            }
            Expr::CpuidExpr => {
                self.ir.emit(ADeadOp::Cpuid);
                // EAX already has result
            }
            // Bitwise operations — using register allocation
            Expr::BitwiseOp { op, left, right } => {
                self.emit_expression(left);

                if let Some(temp) = self.temp_alloc.alloc() {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(temp),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.emit_expression(right);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(temp),
                    });
                    self.temp_alloc.free(temp);
                } else {
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.emit_expression(right);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                }
                match op {
                    BitwiseOp::And => self.ir.emit(ADeadOp::And {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BitwiseOp::Or => self.ir.emit(ADeadOp::Or {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BitwiseOp::Xor => self.ir.emit(ADeadOp::Xor {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BitwiseOp::LeftShift => {
                        // RBX has the shift amount from right expression
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Reg(Reg::RBX),
                        });
                        self.ir.emit(ADeadOp::ShlCl { dst: Reg::RAX });
                    }
                    BitwiseOp::RightShift => {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Reg(Reg::RBX),
                        });
                        self.ir.emit(ADeadOp::ShrCl { dst: Reg::RAX });
                    }
                }
            }
            Expr::BitwiseNot(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::BitwiseNot { dst: Reg::RAX });
            }
            Expr::PostIncrement(inner) => {
                // Post: return old value, then increment in memory
                if let Expr::Variable(name) = inner.as_ref() {
                    if let Some(&offset) = self.variables.get(name.as_str()) {
                        // Load old value into RAX (this is the return value)
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                        // Increment the variable in memory
                        self.ir.emit(ADeadOp::Inc {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                        // RAX still has old value
                    }
                } else {
                    self.emit_expression(inner);
                    self.ir.emit(ADeadOp::Inc {
                        dst: Operand::Reg(Reg::RAX),
                    });
                }
            }
            Expr::PreIncrement(inner) => {
                // Pre: increment in memory, then return new value
                if let Expr::Variable(name) = inner.as_ref() {
                    if let Some(&offset) = self.variables.get(name.as_str()) {
                        // Increment the variable in memory first
                        self.ir.emit(ADeadOp::Inc {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                        // Load new value into RAX
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    }
                } else {
                    self.emit_expression(inner);
                    self.ir.emit(ADeadOp::Inc {
                        dst: Operand::Reg(Reg::RAX),
                    });
                }
            }
            Expr::PostDecrement(inner) => {
                if let Expr::Variable(name) = inner.as_ref() {
                    if let Some(&offset) = self.variables.get(name.as_str()) {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                        self.ir.emit(ADeadOp::Dec {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    }
                } else {
                    self.emit_expression(inner);
                    self.ir.emit(ADeadOp::Dec {
                        dst: Operand::Reg(Reg::RAX),
                    });
                }
            }
            Expr::PreDecrement(inner) => {
                if let Expr::Variable(name) = inner.as_ref() {
                    if let Some(&offset) = self.variables.get(name.as_str()) {
                        self.ir.emit(ADeadOp::Dec {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    }
                } else {
                    self.emit_expression(inner);
                    self.ir.emit(ADeadOp::Dec {
                        dst: Operand::Reg(Reg::RAX),
                    });
                }
            }
            Expr::Nullptr | Expr::Null => {
                self.ir.emit(ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RAX,
                });
            }
            Expr::LabelAddr { label_name } => {
                // Get the label and emit its address as an immediate
                // The actual address will be resolved by the encoder
                let label = self.get_or_create_named_label(label_name);
                // For now, emit a placeholder that will be resolved
                // We emit the label address reference which the encoder will resolve
                self.ir.emit(ADeadOp::LabelAddrRef {
                    label,
                    size: 4, // 32-bit address
                    base_addr: self.base_address as u32,
                });
            }
            Expr::String(s) => {
                let processed = s
                    .replace("\\n", "\n")
                    .replace("\\t", "\t")
                    .replace("\\r", "\r");
                if !self.strings.contains(&processed) {
                    self.strings.push(processed.clone());
                }
                let addr = self.get_string_address(&processed);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(addr),
                });
            }
            // ========== FIELD ACCESS: obj.field ==========
            Expr::FieldAccess { object, field } => {
                if let Expr::Variable(obj_name) = object.as_ref() {
                    // Get struct type name for proper field offset lookup
                    let struct_name = self.variable_types.get(obj_name).and_then(|ty| match ty {
                        Type::Struct(n) | Type::Named(n) | Type::Class(n) => Some(n.clone()),
                        _ => None,
                    });

                    if self.struct_params.contains(obj_name) {
                        // STRUCT PARAMETER: access through pointer (passed by reference)
                        if let Some(&ptr_offset) = self.variables.get(obj_name) {
                            let field_offset = struct_name
                                .as_ref()
                                .map(|sn| self.get_class_field_offset(sn, field))
                                .unwrap_or_else(|| self.get_field_offset(field));
                            // Load pointer from stack
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RBX),
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: ptr_offset,
                                },
                            });
                            // Load field at [pointer + field_offset]
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RAX),
                                src: Operand::Mem {
                                    base: Reg::RBX,
                                    disp: field_offset,
                                },
                            });
                            return;
                        }
                    }

                    // LOCAL STRUCT: try registered "var.field" first
                    let var_field = format!("{}.{}", obj_name, field);
                    if let Some(&offset) = self.variables.get(&var_field) {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                        return;
                    }

                    // Fallback: compute offset from base
                    if let Some(&base_offset) = self.variables.get(obj_name) {
                        let field_offset = struct_name
                            .as_ref()
                            .map(|sn| self.get_class_field_offset(sn, field))
                            .unwrap_or_else(|| self.get_field_offset(field));
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: base_offset + field_offset,
                            },
                        });
                        return;
                    }
                }

                // Handle nested FieldAccess: r.origin.x
                if let Expr::FieldAccess {
                    object: inner_obj,
                    field: inner_field,
                } = object.as_ref()
                {
                    if let Expr::Variable(obj_name) = inner_obj.as_ref() {
                        // For struct parameters: access nested field through pointer
                        if self.struct_params.contains(obj_name) {
                            if let Some(&ptr_offset) = self.variables.get(obj_name) {
                                let struct_name =
                                    self.variable_types.get(obj_name).and_then(|ty| match ty {
                                        Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                            Some(n.clone())
                                        }
                                        _ => None,
                                    });
                                // Get outer field offset (e.g., "size" in Rect)
                                let outer_off = struct_name
                                    .as_ref()
                                    .map(|sn| self.get_class_field_offset(sn, inner_field))
                                    .unwrap_or_else(|| self.get_field_offset(inner_field));
                                // Get inner field offset (e.g., "x" in Point)
                                // Find the sub-struct type from field_types
                                let sub_struct = struct_name
                                    .as_ref()
                                    .and_then(|sn| self.class_layouts.get(sn))
                                    .and_then(|layout| {
                                        layout
                                            .field_types
                                            .iter()
                                            .find(|(fn_, _)| fn_ == inner_field)
                                            .map(|(_, ft)| ft.clone())
                                    });
                                let inner_off = sub_struct
                                    .as_ref()
                                    .map(|ssn| self.get_class_field_offset(ssn, field))
                                    .unwrap_or_else(|| self.get_field_offset(field));
                                // Load pointer, access at combined offset
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RBX),
                                    src: Operand::Mem {
                                        base: Reg::RBP,
                                        disp: ptr_offset,
                                    },
                                });
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Mem {
                                        base: Reg::RBX,
                                        disp: outer_off + inner_off,
                                    },
                                });
                                return;
                            }
                        }
                        // For local structs: try registered "var.inner.field"
                        let nested_name = format!("{}.{}.{}", obj_name, inner_field, field);
                        if let Some(&offset) = self.variables.get(&nested_name) {
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RAX),
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: offset,
                                },
                            });
                            return;
                        }
                    }
                }

                // Handle indexed struct field access: arr[i].field
                if let Expr::Index {
                    object: arr_obj,
                    index: idx,
                } = object.as_ref()
                {
                    if let Expr::Variable(arr_name) = arr_obj.as_ref() {
                        if let Some(&base_offset) = self.variables.get(arr_name.as_str()) {
                            // Get struct-aware field offset and stride
                            let (field_offset, stride) =
                                if let Some(ty) = self.variable_types.get(arr_name.as_str()) {
                                    match ty {
                                        Type::Array(inner, _) => match inner.as_ref() {
                                            Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                                let fo = self.get_class_field_offset(n, field);
                                                let s = self
                                                    .class_layouts
                                                    .get(n)
                                                    .map(|l| l.size)
                                                    .unwrap_or(8);
                                                (fo, s)
                                            }
                                            _ => (self.get_field_offset(field), 8),
                                        },
                                        _ => (self.get_field_offset(field), 8),
                                    }
                                } else {
                                    (self.get_field_offset(field), 8)
                                };
                            // Evaluate index → RAX
                            self.emit_expression(idx);
                            // RAX = index * stride
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RBX),
                                src: Operand::Imm32(stride),
                            });
                            self.ir.emit(ADeadOp::Mul {
                                dst: Reg::RAX,
                                src: Reg::RBX,
                            });
                            // LEA base address
                            self.ir.emit(ADeadOp::Lea {
                                dst: Reg::RBX,
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: base_offset,
                                },
                            });
                            // RBX = base + index * stride
                            self.ir.emit(ADeadOp::Add {
                                dst: Operand::Reg(Reg::RBX),
                                src: Operand::Reg(Reg::RAX),
                            });
                            // Load field at [RBX + field_offset]
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RAX),
                                src: Operand::Mem {
                                    base: Reg::RBX,
                                    disp: field_offset,
                                },
                            });
                            return;
                        }
                    }
                }

                // Handle Expr::This for C++ methods
                if let Expr::This = object.as_ref() {
                    let var_name = format!("self.{}", field);
                    if let Some(&offset) = self.variables.get(&var_name) {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                        return;
                    }
                }

                // Default: zero
                self.ir.emit(ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RAX,
                });
            }
            // Arrow access: ptr->field — load pointer, then access field at offset (GCC/LLVM ABI)
            Expr::ArrowAccess { pointer, field } => {
                // Get struct type from pointer variable for accurate field offset
                let field_offset = if let Expr::Variable(ptr_name) = pointer.as_ref() {
                    if let Some(ty) = self.variable_types.get(ptr_name) {
                        match ty {
                            Type::Pointer(inner) => match inner.as_ref() {
                                Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                    self.get_class_field_offset(n, field)
                                }
                                _ => self.get_field_offset(field),
                            },
                            Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                self.get_class_field_offset(n, field)
                            }
                            _ => self.get_field_offset(field),
                        }
                    } else {
                        self.get_field_offset(field)
                    }
                } else {
                    self.get_field_offset(field)
                };

                // Load pointer value
                self.emit_expression(pointer);
                // RAX now contains the pointer
                // Load field at [RAX + offset]
                if field_offset == 0 {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RAX,
                            disp: 0,
                        },
                    });
                } else {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBX,
                            disp: field_offset,
                        },
                    });
                }
            }
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                // Method call: obj.method(args) → Class::method(&obj, args)
                // Determine class name from variable's type
                let class_name = match object.as_ref() {
                    Expr::Variable(name) => {
                        if let Some(ty) = self.variable_types.get(name) {
                            match ty {
                                Type::Named(n) | Type::Struct(n) | Type::Class(n) => n.clone(),
                                _ => {
                                    // Try to find a matching function by scanning all registered functions
                                    let mut found_class = String::new();
                                    for func_name in self.functions.keys() {
                                        if func_name.ends_with(&format!("::{}", method)) {
                                            if let Some(pos) = func_name.rfind("::") {
                                                found_class = func_name[..pos].to_string();
                                                break;
                                            }
                                        }
                                    }
                                    if found_class.is_empty() {
                                        name.clone()
                                    } else {
                                        found_class
                                    }
                                }
                            }
                        } else {
                            // No type info - scan functions for matching method
                            let mut found_class = String::new();
                            for func_name in self.functions.keys() {
                                if func_name.ends_with(&format!("::{}", method)) {
                                    if let Some(pos) = func_name.rfind("::") {
                                        found_class = func_name[..pos].to_string();
                                        break;
                                    }
                                }
                            }
                            if found_class.is_empty() {
                                name.clone()
                            } else {
                                found_class
                            }
                        }
                    }
                    _ => "Unknown".to_string(),
                };

                let func_name = format!("{}::{}", class_name, method);

                // Push this pointer to stack to preserve across arg evaluation
                match object.as_ref() {
                    Expr::Variable(name) => {
                        if let Some(&offset) = self.variables.get(name) {
                            self.ir.emit(ADeadOp::Lea {
                                dst: Reg::RAX,
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: offset,
                                },
                            });
                            self.ir.emit(ADeadOp::Push {
                                src: Operand::Reg(Reg::RAX),
                            });
                        } else {
                            self.ir.emit(ADeadOp::Xor {
                                dst: Reg::RAX,
                                src: Reg::RAX,
                            });
                            self.ir.emit(ADeadOp::Push {
                                src: Operand::Reg(Reg::RAX),
                            });
                        }
                    }
                    _ => {
                        self.emit_expression(object);
                        self.ir.emit(ADeadOp::Push {
                            src: Operand::Reg(Reg::RAX),
                        });
                    }
                }

                // Evaluate remaining arguments into RDX, R8, R9
                let arg_regs = [Reg::RDX, Reg::R8, Reg::R9];
                for (i, arg) in args.iter().enumerate() {
                    self.emit_expression(arg);
                    if i < arg_regs.len() {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(arg_regs[i]),
                            src: Operand::Reg(Reg::RAX),
                        });
                    } else {
                        self.ir.emit(ADeadOp::Push {
                            src: Operand::Reg(Reg::RAX),
                        });
                    }
                }
                // Pop this pointer into RCX
                self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });

                // Call the method
                if let Some(compiled) = self.functions.get(&func_name) {
                    let label = compiled.label;
                    self.ir.emit(ADeadOp::Call {
                        target: CallTarget::Relative(label),
                    });
                } else {
                    // Try without class prefix (for free functions)
                    if let Some(compiled) = self.functions.get(method) {
                        let label = compiled.label;
                        self.ir.emit(ADeadOp::Call {
                            target: CallTarget::Relative(label),
                        });
                    } else {
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                    }
                }

                if args.len() > 3 {
                    let stack_args = (args.len() - 3) as i32 * 8;
                    self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RSP),
                        src: Operand::Imm32(stack_args),
                    });
                }
            }
            // ========== TERNARY: cond ? then : else ==========
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let else_label = self.ir.new_label();
                let end_label = self.ir.new_label();

                self.emit_expression(condition);
                self.ir.emit(ADeadOp::Cmp {
                    left: Operand::Reg(Reg::RAX),
                    right: Operand::Imm32(0),
                });
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::Equal,
                    target: else_label,
                });

                self.emit_expression(then_expr);
                self.ir.emit(ADeadOp::Jmp { target: end_label });

                self.ir.emit(ADeadOp::Label(else_label));
                self.emit_expression(else_expr);

                self.ir.emit(ADeadOp::Label(end_label));
            }
            // ========== ARRAY ACCESS: arr[i] ==========
            Expr::Index { object, index } => {
                // Handle this->field[index] (ArrowAccess on array field through pointer)
                if let Expr::ArrowAccess { pointer, field } = object.as_ref() {
                    // Compute address of array field: pointer + field_offset
                    let field_offset = if let Expr::Variable(ptr_name) = pointer.as_ref() {
                        if let Some(ty) = self.variable_types.get(ptr_name) {
                            match ty {
                                Type::Pointer(inner) => match inner.as_ref() {
                                    Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                        self.get_class_field_offset(n, field)
                                    }
                                    _ => self.get_field_offset(field),
                                },
                                Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                    self.get_class_field_offset(n, field)
                                }
                                _ => self.get_field_offset(field),
                            }
                        } else {
                            self.get_field_offset(field)
                        }
                    } else {
                        self.get_field_offset(field)
                    };
                    // Evaluate index
                    self.emit_expression(index);
                    self.ir.emit(ADeadOp::Shl {
                        dst: Reg::RAX,
                        amount: 3,
                    }); // i*8
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    // Load pointer (this)
                    self.emit_expression(pointer);
                    // RAX = pointer; compute pointer + field_offset + i*8
                    if field_offset != 0 {
                        self.ir.emit(ADeadOp::Add {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Imm32(field_offset),
                        });
                    }
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RBX }); // i*8
                    self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    // Load value at [pointer + field_offset + i*8]
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBX,
                            disp: 0,
                        },
                    });
                } else if let Expr::Variable(name) = object.as_ref() {
                    if let Some(&base_offset) = self.variables.get(name.as_str()) {
                        let is_local_array = self.array_vars.contains(name.as_str());

                        if is_local_array {
                            // LOCAL ARRAY: ascending layout, arr[i] at base + i*8
                            if let Expr::Number(idx) = index.as_ref() {
                                let elem_offset = base_offset + (*idx as i32 * 8);
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Mem {
                                        base: Reg::RBP,
                                        disp: elem_offset,
                                    },
                                });
                            } else {
                                // Dynamic index for local array
                                self.emit_expression(index);
                                self.ir.emit(ADeadOp::Shl {
                                    dst: Reg::RAX,
                                    amount: 3,
                                });
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RBX),
                                    src: Operand::Reg(Reg::RAX),
                                });
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Imm32(base_offset),
                                });
                                self.ir.emit(ADeadOp::Add {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Reg(Reg::RBX),
                                });
                                self.ir.emit(ADeadOp::Add {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Reg(Reg::RBP),
                                });
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RBX),
                                    src: Operand::Reg(Reg::RAX),
                                });
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RAX),
                                    src: Operand::Mem {
                                        base: Reg::RBX,
                                        disp: 0,
                                    },
                                });
                            }
                        } else {
                            // POINTER VARIABLE (e.g. function parameter int *arr):
                            // Load pointer value, then index: [ptr + i*stride]
                            let stride = self.element_stride(name);
                            self.emit_expression(index);
                            self.emit_index_scale(stride);
                            self.ir.emit(ADeadOp::Push {
                                src: Operand::Reg(Reg::RAX),
                            });
                            // Load the pointer value from the variable
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RAX),
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: base_offset,
                                },
                            });
                            self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
                            // ptr + i*stride
                            self.ir.emit(ADeadOp::Add {
                                dst: Operand::Reg(Reg::RAX),
                                src: Operand::Reg(Reg::RBX),
                            });
                            // Load value at [ptr + i*stride]
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RBX),
                                src: Operand::Reg(Reg::RAX),
                            });
                            self.emit_load_with_stride(Reg::RBX, stride);
                        }
                    } else {
                        // Unknown variable - evaluate as pointer expression
                        self.emit_expression(index);
                        self.ir.emit(ADeadOp::Push {
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.emit_expression(object);
                        self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
                        self.ir.emit(ADeadOp::Shl {
                            dst: Reg::RBX,
                            amount: 3,
                        });
                        self.ir.emit(ADeadOp::Add {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Reg(Reg::RBX),
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RBX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Mem {
                                base: Reg::RBX,
                                disp: 0,
                            },
                        });
                    }
                } else {
                    // For other expressions (pointer dereference, etc.)
                    self.emit_expression(index);
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.emit_expression(object);
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
                    self.ir.emit(ADeadOp::Shl {
                        dst: Reg::RBX,
                        amount: 3,
                    });
                    self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBX,
                            disp: 0,
                        },
                    });
                }
            }
            // ========== ARRAY LITERAL: [a, b, c] ==========
            Expr::Array(elems) => {
                // Allocate stack space for elements, store each, return base address
                let count = elems.len();
                let base_offset = self.stack_offset - (count as i32 * 8);
                for (i, elem) in elems.iter().enumerate() {
                    self.emit_expression(elem);
                    let elem_offset = base_offset + (i as i32 * 8);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp: elem_offset,
                        },
                        src: Operand::Reg(Reg::RAX),
                    });
                }
                self.stack_offset = base_offset;
                // Return base address (lea rax, [rbp+base_offset])
                self.ir.emit(ADeadOp::Lea {
                    dst: Reg::RAX,
                    src: Operand::Mem {
                        base: Reg::RBP,
                        disp: base_offset,
                    },
                });
            }
            // ========== ADDRESS-OF: &var ==========
            Expr::AddressOf(inner) => {
                if let Expr::Variable(name) = inner.as_ref() {
                    if let Some(&offset) = self.variables.get(name.as_str()) {
                        self.ir.emit(ADeadOp::Lea {
                            dst: Reg::RAX,
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    } else {
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                    }
                } else if let Expr::Index { object, index } = inner.as_ref() {
                    // &arr[i] — compute address of element
                    if let Expr::Variable(arr_name) = object.as_ref() {
                        if let Some(&base_offset) = self.variables.get(arr_name.as_str()) {
                            let is_local_array = self.array_vars.contains(arr_name.as_str());
                            // Determine element stride
                            let stride = if let Some(ty) =
                                self.variable_types.get(arr_name.as_str())
                            {
                                match ty {
                                    Type::Array(inner_ty, _) => match inner_ty.as_ref() {
                                        Type::Struct(n) | Type::Named(n) | Type::Class(n) => {
                                            self.class_layouts.get(n).map(|l| l.size).unwrap_or(8)
                                        }
                                        _ => 8,
                                    },
                                    _ => 8,
                                }
                            } else {
                                8
                            };
                            // Evaluate index → RAX
                            self.emit_expression(index);
                            // RAX = index, multiply by stride
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Reg(Reg::RBX),
                                src: Operand::Imm32(stride),
                            });
                            self.ir.emit(ADeadOp::Mul {
                                dst: Reg::RAX,
                                src: Reg::RBX,
                            });
                            if is_local_array {
                                // Local array: LEA base, then add offset
                                self.ir.emit(ADeadOp::Lea {
                                    dst: Reg::RBX,
                                    src: Operand::Mem {
                                        base: Reg::RBP,
                                        disp: base_offset,
                                    },
                                });
                            } else {
                                // Pointer param: load pointer value
                                self.ir.emit(ADeadOp::Mov {
                                    dst: Operand::Reg(Reg::RBX),
                                    src: Operand::Mem {
                                        base: Reg::RBP,
                                        disp: base_offset,
                                    },
                                });
                            }
                            self.ir.emit(ADeadOp::Add {
                                dst: Operand::Reg(Reg::RAX),
                                src: Operand::Reg(Reg::RBX),
                            });
                        } else {
                            self.ir.emit(ADeadOp::Xor {
                                dst: Reg::RAX,
                                src: Reg::RAX,
                            });
                        }
                    } else {
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                    }
                } else {
                    self.ir.emit(ADeadOp::Xor {
                        dst: Reg::RAX,
                        src: Reg::RAX,
                    });
                }
            }
            // ========== DEREFERENCE: *ptr ==========
            Expr::Deref(inner) => {
                self.emit_expression(inner);
                // RAX has pointer value, load what it points to
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Mem {
                        base: Reg::RBX,
                        disp: 0,
                    },
                });
            }
            // ========== SIZEOF ==========
            Expr::SizeOf(arg) => {
                let size: i64 = match arg.as_ref() {
                    crate::frontend::ast::SizeOfArg::Type(ty) => self.sizeof_type(ty),
                    crate::frontend::ast::SizeOfArg::Expr(inner_expr) => match inner_expr {
                        Expr::Variable(name) => {
                            if let Some(ty) = self.variable_types.get(name.as_str()).cloned() {
                                self.sizeof_type(&ty)
                            } else {
                                8
                            }
                        }
                        Expr::String(s) => (s.len() + 1) as i64,
                        _ => 8,
                    },
                };
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(size as u64),
                });
            }
            // ========== MALLOC ==========
            Expr::Malloc(size_expr) => {
                // Evaluate size argument → RCX (first arg, Windows x64 ABI)
                // Our runtime uses 8-byte qword slots, so scale allocation to
                // ensure enough space (sizeof(int)=4 but stride=8)
                self.emit_expression(size_expr);
                self.ir.emit(ADeadOp::Shl {
                    dst: Reg::RAX,
                    amount: 1,
                }); // ×2 for qword safety
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RCX),
                    src: Operand::Reg(Reg::RAX),
                });
                // Call malloc via dynamic IAT lookup
                self.emit_call_iat("malloc");
                // Result (pointer) is in RAX
            }
            // ========== REALLOC ==========
            Expr::Realloc { ptr, new_size } => {
                // For now, just call malloc with new_size (simplified)
                self.emit_expression(new_size);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RCX),
                    src: Operand::Reg(Reg::RAX),
                });
                // Call malloc via dynamic IAT lookup
                self.emit_call_iat("malloc");
                let _ = ptr; // TODO: proper realloc with ptr as first arg
            }
            // ========== CAST ==========
            Expr::Cast {
                target_type,
                expr: inner,
            } => {
                self.emit_expression(inner);
                // Apply truncation based on target type size
                let target_size = crate::isa::c_isa::c99_sizeof_for_expr(target_type);
                match target_size {
                    1 => {
                        // Truncate to 8 bits (char): movzx rax, al
                        self.ir.emit(ADeadOp::MovZx {
                            dst: Reg::RAX,
                            src: Reg::AL,
                        });
                    }
                    2 => {
                        // Truncate to 16 bits (short): and rax, 0xFFFF
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RBX),
                            src: Operand::Imm32(0xFFFF),
                        });
                        self.ir.emit(ADeadOp::And {
                            dst: Reg::RAX,
                            src: Reg::RBX,
                        });
                    }
                    4 => {
                        // Truncate to 32 bits (int): mov eax, eax (zero-extends upper 32)
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::EAX),
                            src: Operand::Reg(Reg::EAX),
                        });
                    }
                    _ => {
                        // 8 bytes or unknown — no truncation needed
                    }
                }
            }
            _ => {
                self.ir.emit(ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RAX,
                });
            }
        }
    }

    /// Check if a Named type has a real struct layout (vs being a typedef like HINSTANCE)
    fn is_real_struct_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Struct(n) | Type::Class(n) => self.class_layouts.contains_key(n),
            Type::Named(n) => self.class_layouts.contains_key(n),
            _ => false,
        }
    }

    fn emit_call(&mut self, name: &str, args: &[Expr]) {
        // ========== INTRINSIC: __store16(ptr, offset, value) ==========
        // Emits a 16-bit store: mov WORD [ptr + offset], value
        // Used for VGA text mode (each cell = char:8 + attr:8 = 16 bits)
        if name == "__store16" && args.len() == 3 {
            let const_offset = match &args[1] {
                Expr::Number(n) => Some(*n as i32),
                _ => None,
            };
            if let Some(disp) = const_offset {
                self.emit_expression(&args[2]);
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RAX) });
                self.emit_expression(&args[0]);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                self.ir.emit(ADeadOp::Store16 {
                    base: Reg::RBX,
                    disp,
                    src: Reg::RAX,
                });
            } else {
                self.emit_expression(&args[2]);
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RAX) });
                self.emit_expression(&args[1]);
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RAX) });
                self.emit_expression(&args[0]);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
                self.ir.emit(ADeadOp::Add {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RCX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                self.ir.emit(ADeadOp::Store16 {
                    base: Reg::RBX,
                    disp: 0,
                    src: Reg::RAX,
                });
            }
            return;
        }

        // ========== INTRINSIC: __store32(ptr, offset, value) ==========
        // Emits a 32-bit store: mov DWORD [ptr + offset], value
        // Used for VGA writes, GUID construction, 4-byte struct field writes
        if name == "__store32" && args.len() == 3 {
            // Check if offset is a constant
            let const_offset = match &args[1] {
                Expr::Number(n) => Some(*n as i32),
                _ => None,
            };

            if let Some(disp) = const_offset {
                // Fast path: constant offset → Store32 with displacement
                self.emit_expression(&args[2]);
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RAX) });
                self.emit_expression(&args[0]);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                self.ir.emit(ADeadOp::Store32 {
                    base: Reg::RBX,
                    disp,
                    src: Reg::RAX,
                });
            } else {
                // Dynamic offset: base + offset computed at runtime
                // Evaluate value → push, evaluate offset → push, evaluate base → RBX
                self.emit_expression(&args[2]);
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RAX) });
                self.emit_expression(&args[1]);
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RAX) });
                self.emit_expression(&args[0]);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                // Pop offset → RCX, add to base
                self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
                self.ir.emit(ADeadOp::Add {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RCX),
                });
                // Pop value → RAX
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                // Store32: mov DWORD [RBX + 0], EAX
                self.ir.emit(ADeadOp::Store32 {
                    base: Reg::RBX,
                    disp: 0,
                    src: Reg::RAX,
                });
            }
            return;
        }

        // ========== INTRINSIC: __inb(port) ==========
        // Reads a byte from I/O port. Returns value in AL (zero-extended to RAX).
        // Used for PS/2 keyboard reading (port 0x60, 0x64)
        if name == "__inb" && args.len() == 1 {
            match &args[0] {
                Expr::Number(p) if *p >= 0 && *p <= 255 => {
                    self.ir.emit(ADeadOp::InByte {
                        port: Operand::Imm8(*p as i8),
                    });
                }
                _ => {
                    // Dynamic port: evaluate → DX, then in al, dx
                    self.emit_expression(&args[0]);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::InByte {
                        port: Operand::Reg(Reg::DX),
                    });
                }
            }
            // Zero-extend AL → RAX
            self.ir.emit(ADeadOp::MovZx {
                dst: Reg::RAX,
                src: Reg::AL,
            });
            return;
        }

        // ========== INTRINSIC: __outb(port, value) ==========
        // Writes a byte to I/O port.
        // Used for PIC, PIT, keyboard controller
        if name == "__outb" && args.len() == 2 {
            // Evaluate value → AL
            self.emit_expression(&args[1]);
            self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RAX) });
            // Evaluate port
            match &args[0] {
                Expr::Number(p) if *p >= 0 && *p <= 255 => {
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                    self.ir.emit(ADeadOp::OutByte {
                        port: Operand::Imm8(*p as i8),
                        src: Operand::Reg(Reg::AL),
                    });
                }
                _ => {
                    self.emit_expression(&args[0]);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                    self.ir.emit(ADeadOp::OutByte {
                        port: Operand::Reg(Reg::DX),
                        src: Operand::Reg(Reg::AL),
                    });
                }
            }
            return;
        }

        // ========== INTRINSIC: __inl(port) ==========
        // Reads a dword from I/O port. Returns value in EAX.
        // Used for PCI config space reading (port 0xCFC)
        if name == "__inl" && args.len() == 1 {
            match &args[0] {
                Expr::Number(p) if *p >= 0 && *p <= 255 => {
                    self.ir.emit(ADeadOp::InDword {
                        port: Operand::Imm8(*p as i8),
                    });
                }
                _ => {
                    self.emit_expression(&args[0]);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::InDword {
                        port: Operand::Reg(Reg::DX),
                    });
                }
            }
            return;
        }

        // ========== INTRINSIC: __outl(port, value) ==========
        // Writes a dword to I/O port.
        // Used for PCI config space address (port 0xCF8)
        if name == "__outl" && args.len() == 2 {
            self.emit_expression(&args[1]);
            self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RAX) });
            match &args[0] {
                Expr::Number(p) if *p >= 0 && *p <= 255 => {
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                    self.ir.emit(ADeadOp::OutDword {
                        port: Operand::Imm8(*p as i8),
                        src: Operand::Reg(Reg::EAX),
                    });
                }
                _ => {
                    self.emit_expression(&args[0]);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                    self.ir.emit(ADeadOp::OutDword {
                        port: Operand::Reg(Reg::DX),
                        src: Operand::Reg(Reg::EAX),
                    });
                }
            }
            return;
        }

        // ========== INTRINSIC: __cpuid_eax/ebx/ecx/edx(leaf) ==========
        // Execute CPUID with EAX=leaf, return specified register
        if args.len() == 1 {
            let cpuid_reg = if name == "__cpuid_eax" { Some(Reg::RAX) }
                else if name == "__cpuid_ebx" { Some(Reg::RBX) }
                else if name == "__cpuid_ecx" { Some(Reg::RCX) }
                else if name == "__cpuid_edx" { Some(Reg::RDX) }
                else { None };
            if let Some(result_reg) = cpuid_reg {
                // Save callee-saved registers that CPUID clobbers
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RBX) });
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RCX) });
                self.ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RDX) });
                // Set EAX = leaf, ECX = 0 (sub-leaf)
                self.emit_expression(&args[0]);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RCX),
                    src: Operand::Imm64(0),
                });
                // Execute CPUID
                self.ir.emit(ADeadOp::Cpuid);
                // Move desired result to R8 (safe temp)
                if result_reg != Reg::RAX {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::R8),
                        src: Operand::Reg(result_reg),
                    });
                }
                // Restore clobbered registers
                self.ir.emit(ADeadOp::Pop { dst: Reg::RDX });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
                // Move result to RAX (return value)
                if result_reg != Reg::RAX {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::R8),
                    });
                }
                return;
            }
        }

        // ========== INTRINSIC: __hlt() ==========
        // Halt CPU until next interrupt
        if name == "__hlt" && args.len() == 0 {
            self.ir.emit(ADeadOp::Hlt);
            return;
        }

        // ========== INTRINSIC: __cli() / __sti() ==========
        if name == "__cli" && args.len() == 0 {
            self.ir.emit(ADeadOp::Cli);
            return;
        }
        if name == "__sti" && args.len() == 0 {
            self.ir.emit(ADeadOp::Sti);
            return;
        }

        // Windows x64 ABI: first 4 args in RCX, RDX, R8, R9
        // Args 5+ go on the stack at [rsp+32], [rsp+40], etc. (after shadow space)
        let total_args = args.len();
        let reg_args = total_args.min(4);
        let stack_args = if total_args > 4 { total_args - 4 } else { 0 };

        // Phase 1: Push stack args (5+) in REVERSE order onto stack
        // These will sit above the shadow space after we allocate it
        if stack_args > 0 {
            for i in (4..total_args).rev() {
                self.emit_expression(&args[i]);
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
            }
        }

        // Phase 2: Evaluate register args (1-4), push to stack temporarily
        for i in 0..reg_args {
            let arg = &args[i];
            // Check if argument is a struct variable with real layout - pass by reference
            if let Expr::Variable(var_name) = arg {
                if let Some(ty) = self.variable_types.get(var_name).cloned() {
                    if self.is_real_struct_type(&ty) {
                        if let Some(&offset) = self.variables.get(var_name) {
                            self.ir.emit(ADeadOp::Lea {
                                dst: Reg::RAX,
                                src: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: offset,
                                },
                            });
                            self.ir.emit(ADeadOp::Push {
                                src: Operand::Reg(Reg::RAX),
                            });
                            continue;
                        }
                    }
                }
            }
            // Normal argument: evaluate expression
            self.emit_expression(arg);
            self.ir.emit(ADeadOp::Push {
                src: Operand::Reg(Reg::RAX),
            });
        }

        // Phase 3: Pop register args into arg registers (reverse order)
        for i in (0..reg_args).rev() {
            let dst = self.arg_register(i);
            self.ir.emit(ADeadOp::Pop { dst });
        }

        // Check if this is an IAT-imported function (printf, scanf, malloc, free, Win32, DX12...)
        let iat_name = match name {
            "printf" | "std::printf" => Some("printf"),
            "scanf" | "std::scanf" => Some("scanf"),
            "malloc" => Some("malloc"),
            "free" => Some("free"),
            _ => {
                if iat_registry::slot_for_function(name).is_some() {
                    Some(name)
                } else {
                    None
                }
            }
        };
        if let Some(iat_func) = iat_name {
            // For IAT calls with >4 args, we need custom shadow space handling
            // because the stack args are already above RSP
            if stack_args > 0 {
                // Allocate shadow space (32 bytes) below the stack args
                self.ir.emit(ADeadOp::Sub {
                    dst: Operand::Reg(Reg::RSP),
                    src: Operand::Imm8(32),
                });
                // Emit the call directly (not through emit_call_iat which adds its own shadow)
                let assumed_idata_rva: u32 = 0x2000;
                let idata_result = iat_registry::build_idata(assumed_idata_rva, &[]);
                let slot = iat_registry::slot_for_function(iat_func)
                    .unwrap_or_else(|| panic!("IAT function not found: {}", iat_func));
                let iat_rva = idata_result.slot_to_iat_rva[slot];
                self.ir.emit(ADeadOp::CallIAT { iat_rva });
                // Clean up: shadow space + stack args
                let cleanup = 32 + (stack_args as i32 * 8);
                self.ir.emit(ADeadOp::Add {
                    dst: Operand::Reg(Reg::RSP),
                    src: Operand::Imm32(cleanup),
                });
            } else {
                self.emit_call_iat(iat_func);
            }
            return;
        }

        // Shadow space
        self.ir.emit(ADeadOp::Sub {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm8(32),
        });

        // Call usando label de la función, o indirecto si es function pointer
        if let Some(func) = self.functions.get(name) {
            let label = func.label;
            self.ir.emit(ADeadOp::Call {
                target: CallTarget::Relative(label),
            });
        } else if self.variables.contains_key(name) {
            // Function pointer: load pointer from variable, call through R10
            // (R10 is volatile and not used for args in Windows x64 ABI)
            let offset = self.variables[name];
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::R10),
                src: Operand::Mem {
                    base: Reg::RBP,
                    disp: offset,
                },
            });
            self.ir.emit(ADeadOp::Call {
                target: CallTarget::Register(Reg::R10),
            });
        } else {
            self.ir.emit(ADeadOp::Call {
                target: CallTarget::Name(name.to_string()),
            });
        }

        // Restaurar stack
        self.ir.emit(ADeadOp::Add {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm8(32),
        });
    }

    fn emit_input(&mut self) {
        let input_offset = self.stack_offset;
        self.stack_offset -= 8;

        self.ir.emit(ADeadOp::Xor {
            dst: Reg::RAX,
            src: Reg::RAX,
        });
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Mem {
                base: Reg::RBP,
                disp: input_offset,
            },
            src: Operand::Reg(Reg::RAX),
        });

        let fmt_addr = self.get_string_address("%d");
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RCX),
            src: Operand::Imm64(fmt_addr),
        });
        self.ir.emit(ADeadOp::Lea {
            dst: Reg::RDX,
            src: Operand::Mem {
                base: Reg::RBP,
                disp: input_offset,
            },
        });

        // Call scanf via dynamic IAT lookup
        self.emit_call_iat("scanf");

        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RAX),
            src: Operand::Mem {
                base: Reg::RBP,
                disp: input_offset,
            },
        });
    }

    /// Retorna referencia a la IR generada (para debug/inspección).
    pub fn ir(&self) -> &ADeadIR {
        &self.ir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::Parser;

    #[test]
    fn test_hello_world_compiles() {
        let source = r#"
fn main() {
    println("Hello, ADead-BIB!")
}
"#;
        let program = Parser::parse_program(source).unwrap();
        let mut compiler = IsaCompiler::new(Target::Windows);
        let (code, data, _, _) = compiler.compile(&program);
        assert!(!code.is_empty(), "Code should not be empty");
        assert!(!data.is_empty(), "Data should contain strings");
    }

    #[test]
    fn test_variables_compile() {
        let source = r#"
fn main() {
    let x = 42
    let y = 10
    let result = x + y
    println(result)
}
"#;
        let program = Parser::parse_program(source).unwrap();
        let mut compiler = IsaCompiler::new(Target::Windows);
        let (code, _data, _, _) = compiler.compile(&program);
        assert!(!code.is_empty());
    }

    #[test]
    fn test_ir_is_readable() {
        let source = r#"
fn main() {
    println("Hello!")
}
"#;
        let program = Parser::parse_program(source).unwrap();
        let mut compiler = IsaCompiler::new(Target::Windows);
        let _ = compiler.compile(&program);

        // Verify we can print every instruction
        for op in compiler.ir().ops() {
            let s = format!("{}", op);
            assert!(!s.is_empty());
        }
    }

    // ================================================================
    // END-TO-END C → MACHINE CODE TESTS
    // ================================================================
    // These tests verify the FULL pipeline:
    //   C source → Lexer → Parser → C AST → IR → ISA Compiler → x86-64 bytes
    //
    // Inspired by GCC torture tests. Each test:
    //   1. Parses C source with compile_c_to_program()
    //   2. Compiles to machine code with IsaCompiler
    //   3. Verifies non-empty code/data output
    //   4. Verifies the ADeadIR is valid and printable
    // ================================================================

    fn compile_c_e2e(c_source: &str) -> (Vec<u8>, Vec<u8>, usize) {
        use crate::frontend::c::compile_c_to_program;
        let program = compile_c_to_program(c_source).expect("C parse failed");
        let mut compiler = IsaCompiler::new(Target::Windows);
        let (code, data, _, _) = compiler.compile(&program);
        let ir_len = compiler.ir().ops().len();
        // Verify every IR op is displayable
        for op in compiler.ir().ops() {
            let s = format!("{}", op);
            assert!(!s.is_empty(), "IR op should be printable");
        }
        (code, data, ir_len)
    }

    #[test]
    fn test_c_e2e_hello_world() {
        let (code, data, ir_len) = compile_c_e2e(
            r#"
            int main() {
                printf("Hello from ADead-BIB C!\n");
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty(), "should generate code");
        assert!(!data.is_empty(), "should have string data");
        assert!(ir_len > 5, "should have multiple IR ops, got {}", ir_len);
    }

    #[test]
    fn test_c_e2e_arithmetic() {
        let (code, _, ir_len) = compile_c_e2e(
            r#"
            int add(int a, int b) { return a + b; }
            int sub(int a, int b) { return a - b; }
            int mul(int a, int b) { return a * b; }
            int main() {
                int r = add(3, 4);
                int s = sub(10, 3);
                int m = mul(5, 6);
                printf("r=%d s=%d m=%d\n", r, s, m);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
        assert!(ir_len > 20, "should have many IR ops for 4 functions");
    }

    #[test]
    fn test_c_e2e_control_flow() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int main() {
                int sum = 0;
                for (int i = 0; i < 10; i++) {
                    if (i % 2 == 0) {
                        sum += i;
                    }
                }
                int j = 100;
                while (j > 0) {
                    j = j - 10;
                }
                return sum;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_recursion() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int factorial(int n) {
                if (n <= 1) return 1;
                return n * factorial(n - 1);
            }
            int main() {
                int r = factorial(10);
                printf("10! = %d\n", r);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_arrays() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int main() {
                int arr[5];
                for (int i = 0; i < 5; i++) {
                    arr[i] = (i + 1) * 10;
                }
                int total = 0;
                for (int i = 0; i < 5; i++) {
                    total += arr[i];
                }
                printf("total=%d\n", total);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_pointers() {
        let (code, _, _) = compile_c_e2e(
            r#"
            void swap(int *a, int *b) {
                int temp = *a;
                *a = *b;
                *b = temp;
            }
            int main() {
                int x = 10;
                int y = 20;
                swap(&x, &y);
                printf("x=%d y=%d\n", x, y);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_switch() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int classify(int n) {
                switch (n) {
                    case 0: return 0;
                    case 1: return 10;
                    case 2: return 20;
                    default: return -1;
                }
            }
            int main() {
                printf("0=%d 1=%d 2=%d 9=%d\n",
                    classify(0), classify(1), classify(2), classify(9));
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_dowhile() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int main() {
                int count = 0;
                do {
                    count++;
                } while (count < 10);
                printf("count=%d\n", count);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_ternary() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int max(int a, int b) { return (a > b) ? a : b; }
            int min(int a, int b) { return (a < b) ? a : b; }
            int main() {
                printf("max=%d min=%d\n", max(10, 20), min(10, 20));
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_struct() {
        let (code, _, _) = compile_c_e2e(
            r#"
            struct Point { int x; int y; };
            int main() {
                struct Point p;
                p.x = 10;
                p.y = 20;
                printf("point=(%d,%d)\n", p.x, p.y);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_enum() {
        let (code, _, _) = compile_c_e2e(
            r#"
            enum Color { RED = 0, GREEN = 1, BLUE = 2 };
            int main() {
                int c = GREEN;
                printf("color=%d\n", c);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_multiple_strings() {
        let (code, data, _) = compile_c_e2e(
            r#"
            int main() {
                printf("Line 1\n");
                printf("Line 2\n");
                printf("Line 3\n");
                printf("Done\n");
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
        assert!(!data.is_empty());
    }

    #[test]
    fn test_c_e2e_nested_calls() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int square(int x) { return x * x; }
            int add(int a, int b) { return a + b; }
            int main() {
                int r = add(square(3), square(4));
                printf("3^2+4^2=%d\n", r);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_many_variables() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int main() {
                int a = 1, b = 2, c = 3, d = 4, e = 5;
                int f = 6, g = 7, h = 8;
                int total = a + b + c + d + e + f + g + h;
                printf("total=%d\n", total);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_bubble_sort() {
        let (code, _, ir_len) = compile_c_e2e(
            r#"
            void sort(int *arr, int n) {
                for (int i = 0; i < n - 1; i++) {
                    for (int j = 0; j < n - i - 1; j++) {
                        if (arr[j] > arr[j + 1]) {
                            int t = arr[j];
                            arr[j] = arr[j + 1];
                            arr[j + 1] = t;
                        }
                    }
                }
            }
            int main() {
                int a[] = {5, 3, 1, 4, 2};
                sort(a, 5);
                printf("sorted=%d %d %d %d %d\n",
                    a[0], a[1], a[2], a[3], a[4]);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
        assert!(ir_len > 50, "bubble sort should generate many IR ops");
    }

    #[test]
    fn test_c_e2e_bitwise_ops() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int main() {
                int a = 0xFF;
                int b = a & 0x0F;
                int c = a | 0xF00;
                int d = a ^ 0xFF;
                int e = a << 4;
                int f = a >> 4;
                printf("b=%d c=%d d=%d e=%d f=%d\n", b, c, d, e, f);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_compound_assigns() {
        let (code, _, _) = compile_c_e2e(
            r#"
            int main() {
                int x = 100;
                x += 50;
                x -= 30;
                x *= 2;
                x /= 3;
                x %= 7;
                printf("x=%d\n", x);
                return 0;
            }
        "#,
        );
        assert!(!code.is_empty());
    }

    #[test]
    fn test_c_e2e_full_hello_c_example() {
        // This is the big integration test — compile the full hello.c showcase
        let source = std::fs::read_to_string("examples/c/hello.c").expect("hello.c should exist");
        let program =
            crate::frontend::c::compile_c_to_program(&source).expect("hello.c should parse");
        let mut compiler = IsaCompiler::new(Target::Windows);
        let (code, data, _, _) = compiler.compile(&program);
        assert!(!code.is_empty(), "hello.c should generate code");
        assert!(!data.is_empty(), "hello.c should have string data");
        assert!(
            compiler.ir().ops().len() > 100,
            "hello.c should generate 100+ IR ops, got {}",
            compiler.ir().ops().len()
        );
    }
}
