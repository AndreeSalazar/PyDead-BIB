// AST (Abstract Syntax Tree) para ADead-BIB
// Lenguaje de uso general con OOP - Binario + HEX

// Use unified type system
pub use super::types::RegSize;
pub use super::types::Type;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
    Variable(String),
    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Comparison {
        op: CmpOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    // Arrays y colecciones
    Array(Vec<Expr>),
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        object: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
    },
    // OOP
    New {
        class_name: String,
        args: Vec<Expr>,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    This,
    Super,
    // Input del usuario
    Input, // input() - lee un número del teclado
    // Funcional
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    // Built-in functions v1.3.0
    Len(Box<Expr>), // len(expr) - longitud de array/string
    Push {
        // arr.push(val) o push(arr, val)
        array: Box<Expr>,
        value: Box<Expr>,
    },
    Pop(Box<Expr>),       // arr.pop() o pop(arr)
    IntCast(Box<Expr>),   // int(expr) - convertir a entero
    FloatCast(Box<Expr>), // float(expr) - convertir a flotante
    StrCast(Box<Expr>),   // str(expr) - convertir a string
    BoolCast(Box<Expr>),  // bool(expr) - convertir a booleano
    // String operations
    StringConcat {
        // "a" + "b"
        left: Box<Expr>,
        right: Box<Expr>,
    },

    // ========== PUNTEROS Y MEMORIA (v3.2) ==========
    /// Dereference: *ptr
    Deref(Box<Expr>),

    /// Address-of: &var
    AddressOf(Box<Expr>),

    /// Arrow access: ptr->field
    ArrowAccess {
        pointer: Box<Expr>,
        field: String,
    },

    /// Sizeof: sizeof(type) o sizeof(expr)
    SizeOf(Box<SizeOfArg>),

    /// Malloc: malloc(size)
    Malloc(Box<Expr>),

    /// Realloc: realloc(ptr, new_size)
    Realloc {
        ptr: Box<Expr>,
        new_size: Box<Expr>,
    },

    /// Cast: (int*)ptr
    Cast {
        target_type: Type,
        expr: Box<Expr>,
    },

    /// Nullptr literal
    Nullptr,

    /// Pre-increment: ++x
    PreIncrement(Box<Expr>),

    /// Pre-decrement: --x
    PreDecrement(Box<Expr>),

    /// Post-increment: x++
    PostIncrement(Box<Expr>),

    /// Post-decrement: x--
    PostDecrement(Box<Expr>),

    // Bitwise operations
    BitwiseOp {
        op: BitwiseOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Bitwise NOT: ~x
    BitwiseNot(Box<Expr>),

    // ========== OS-LEVEL / MACHINE CODE (v3.1-OS) ==========
    /// Read from CPU register: reg(rax), reg(cr0), etc.
    RegRead {
        reg_name: String,
    },

    /// Read from memory address: read_mem(addr)
    MemRead {
        addr: Box<Expr>,
    },

    /// Read from I/O port: port_in(port_num)
    PortIn {
        port: Box<Expr>,
    },

    /// CPUID result (returns conceptual value)
    CpuidExpr,

    // ========== LABEL ADDRESS (v3.3-Boot) ==========
    /// label_addr(name) — Returns the absolute address of a named label
    /// Used for writing label addresses to memory (e.g., for far jump pointers)
    LabelAddr {
        label_name: String,
    },
}

/// Argumento de sizeof
#[derive(Debug, Clone)]
pub enum SizeOfArg {
    Type(Type),
    Expr(Expr),
}

/// Operadores bitwise
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BitwiseOp {
    And,        // &
    Or,         // |
    Xor,        // ^
    LeftShift,  // <<
    RightShift, // >>
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CmpOp {
    Eq, // ==
    Ne, // !=
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Print(Expr),
    Println(Expr), // println con \n automático
    PrintNum(Expr),
    Assign {
        name: String,
        value: Expr,
    },
    IndexAssign {
        object: Expr,
        index: Expr,
        value: Expr,
    },
    FieldAssign {
        object: Expr,
        field: String,
        value: Expr,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        var: String,
        start: Expr,
        end: Expr,
        body: Vec<Stmt>,
    },
    ForEach {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Pass,
    Assert {
        condition: Expr,
        message: Option<Expr>,
    },
    Expr(Expr),

    // ========== PUNTEROS Y MEMORIA (v3.2) ==========
    /// Declaración con tipo: int x = 5, int* ptr = &x
    VarDecl {
        var_type: Type,
        name: String,
        value: Option<Expr>,
    },

    /// Asignación a puntero dereferenciado: *ptr = value
    DerefAssign {
        pointer: Expr,
        value: Expr,
    },

    /// Asignación via arrow: ptr->field = value
    ArrowAssign {
        pointer: Expr,
        field: String,
        value: Expr,
    },

    /// Free memory: free(ptr)
    Free(Expr),

    /// Delete (C++ style): delete ptr, delete[] arr
    Delete {
        expr: Expr,
        is_array: bool,
    },

    /// Do-While loop
    DoWhile {
        body: Vec<Stmt>,
        condition: Expr,
    },

    /// Switch statement
    Switch {
        expr: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Vec<Stmt>>,
    },

    /// Compound assignment: x += 5, x -= 3, etc.
    CompoundAssign {
        name: String,
        op: CompoundOp,
        value: Expr,
    },

    /// Increment/Decrement statement: x++, ++x, x--, --x
    Increment {
        name: String,
        is_pre: bool,
        is_increment: bool,
    },

    // ========== OS-LEVEL / MACHINE CODE (v3.1-OS) ==========
    /// CLI — Disable interrupts
    Cli,

    /// STI — Enable interrupts
    Sti,

    /// HLT — Halt CPU
    Hlt,

    /// IRET — Return from interrupt handler
    Iret,

    /// INT n — Software interrupt call
    IntCall {
        vector: u8,
    },

    /// reg rax = value — Write to CPU register
    RegAssign {
        reg_name: String,
        value: Expr,
    },

    /// write_mem(addr, value) — Write to memory address
    MemWrite {
        addr: Expr,
        value: Expr,
    },

    /// port_out(port, value) — Write byte to I/O port
    PortOut {
        port: Expr,
        value: Expr,
    },

    /// raw { 0xEB, 0xFE } — Inline raw machine code bytes
    RawBlock {
        bytes: Vec<u8>,
    },

    /// org 0x7C00 — Set origin address
    OrgDirective {
        address: u64,
    },

    /// align 16 — Alignment directive
    AlignDirective {
        alignment: u64,
    },

    /// far_jump(selector, offset) — Far jump for mode switching
    FarJump {
        selector: u16,
        offset: u32,
    },

    /// cpuid — Execute CPUID instruction
    Cpuid,

    // ========== LABELS Y JUMPS (v3.3-Boot) ==========
    /// label_name: — Define a named label at current position
    LabelDef {
        name: String,
    },

    /// jmp label_name — Jump to a named label
    JumpTo {
        label: String,
    },

    /// jz label_name — Jump if zero to a named label
    JumpIfZero {
        label: String,
    },

    /// jnz label_name — Jump if not zero to a named label
    JumpIfNotZero {
        label: String,
    },

    /// jc label_name — Jump if carry to a named label
    JumpIfCarry {
        label: String,
    },

    /// jnc label_name — Jump if not carry to a named label
    JumpIfNotCarry {
        label: String,
    },

    /// Data definition: db "string" or db 0x55, 0xAA
    DataBytes {
        bytes: Vec<u8>,
    },

    /// Data definition: dw 0x1234 (16-bit words)
    DataWords {
        words: Vec<u16>,
    },

    /// Data definition: dd 0x12345678 (32-bit dwords)
    DataDwords {
        dwords: Vec<u32>,
    },

    /// times N db 0 — Repeat byte N times
    TimesDirective {
        count: usize,
        byte: u8,
    },

    // ========== DEBUGINFO (v5.0) ==========
    /// Line tracking from parser for UB detector
    LineMarker(usize),
}

/// Case de switch
#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub value: Expr,
    pub body: Vec<Stmt>,
    pub has_break: bool,
}

/// Operadores de asignación compuesta
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompoundOp {
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
    ModAssign, // %=
    AndAssign, // &=
    OrAssign,  // |=
    XorAssign, // ^=
    ShlAssign, // <<=
    ShrAssign, // >>=
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub param_type: Type,            // Unified type (always present)
    pub default_value: Option<Expr>, // Default value
}

impl Param {
    pub fn new(name: String) -> Self {
        Self {
            name,
            param_type: Type::Auto,
            default_value: None,
        }
    }

    pub fn with_type(name: String, type_name: String) -> Self {
        Self {
            name,
            param_type: Type::from_c_name(&type_name),
            default_value: None,
        }
    }

    pub fn typed(name: String, param_type: Type) -> Self {
        Self {
            name,
            param_type,
            default_value: None,
        }
    }
}

/// Atributos de función para OS-level (v3.1-OS)
#[derive(Debug, Clone, Default)]
pub struct FunctionAttributes {
    /// @interrupt — auto push/pop registers + iretq
    pub is_interrupt: bool,
    /// @exception — like interrupt but for CPU exceptions (with error code)
    pub is_exception: bool,
    /// @naked — no prologue/epilogue generated
    pub is_naked: bool,
    /// @export("C") — C-compatible symbol name
    pub export_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub resolved_return_type: Type,
    pub body: Vec<Stmt>,
    pub attributes: FunctionAttributes,
}

// OOP: Interface/Trait
#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub methods: Vec<MethodSignature>,
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub resolved_return_type: Type,
}

// OOP: Clase con herencia y polimorfismo
#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub parent: Option<String>,  // Herencia
    pub implements: Vec<String>, // Interfaces implementadas
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub constructor: Option<Method>, // __init__
    pub destructor: Option<Method>,  // __del__
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub type_name: Option<String>,
    pub field_type: Type,
    pub default_value: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub resolved_return_type: Type,
    pub body: Vec<Stmt>,
    pub is_virtual: bool,  // Para polimorfismo
    pub is_override: bool, // Override de método padre
    pub is_static: bool,   // Método estático
}

// Rust-style struct
#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<StructField>,
    /// @packed — no padding, exact memory layout (for hardware structs)
    pub is_packed: bool,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: Type,
}

// Rust-style impl block
#[derive(Debug, Clone)]
pub struct Impl {
    pub struct_name: String,
    pub trait_name: Option<String>, // Some("TraitName") for `impl Trait for Struct`
    pub methods: Vec<Function>,
}

// Trait definition (v1.6.0)
#[derive(Debug, Clone)]
pub struct Trait {
    pub name: String,
    pub methods: Vec<TraitMethod>,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub resolved_return_type: Type,
    pub default_body: Option<Vec<Stmt>>, // Default implementation (optional)
}

// Sistema de imports
#[derive(Debug, Clone)]
pub struct Import {
    pub module: String,
    pub items: Vec<String>,    // from module import item1, item2
    pub alias: Option<String>, // import module as alias
}

/// Atributos de programa (#![...])
#[derive(Debug, Clone, Default)]
pub struct ProgramAttributes {
    pub mode: OutputMode,          // #![mode(raw|pe|elf)]
    pub base_address: Option<u64>, // #![base(0x1000)]
    pub clean_level: CleanLevel,   // #![clean(normal|aggressive|none)]
    pub cpu_mode: CpuModeAttr,     // #![cpu(real16|protected32|long64)]
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum OutputMode {
    #[default]
    PE, // Windows PE (default)
    ELF,  // Linux ELF
    Raw,  // Bytes puros sin headers
    Flat, // Flat binary (boot sectors, bare-metal)
}

/// CPU mode attribute for OS-level code generation
#[derive(Debug, Clone, Default, PartialEq)]
pub enum CpuModeAttr {
    Real16,      // 16-bit real mode (boot sector)
    Protected32, // 32-bit protected mode
    #[default]
    Long64, // 64-bit long mode (default)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum CleanLevel {
    #[default]
    Normal,
    Aggressive,
    None,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub attributes: ProgramAttributes, // Atributos del programa
    pub imports: Vec<Import>,
    pub interfaces: Vec<Interface>,
    pub traits: Vec<Trait>, // Rust-style traits (v1.6.0)
    pub classes: Vec<Class>,
    pub structs: Vec<Struct>, // Rust-style structs
    pub impls: Vec<Impl>,     // Rust-style impl blocks
    pub functions: Vec<Function>,
    pub statements: Vec<Stmt>, // Top-level statements (scripts)
}

impl Program {
    pub fn new() -> Self {
        Self {
            attributes: ProgramAttributes::default(),
            imports: Vec::new(),
            interfaces: Vec::new(),
            traits: Vec::new(),
            classes: Vec::new(),
            structs: Vec::new(),
            impls: Vec::new(),
            functions: Vec::new(),
            statements: Vec::new(),
        }
    }

    pub fn add_trait(&mut self, t: Trait) {
        self.traits.push(t);
    }

    pub fn add_struct(&mut self, s: Struct) {
        self.structs.push(s);
    }

    pub fn add_impl(&mut self, i: Impl) {
        self.impls.push(i);
    }

    pub fn add_import(&mut self, import: Import) {
        self.imports.push(import);
    }

    pub fn add_function(&mut self, func: Function) {
        self.functions.push(func);
    }

    pub fn add_class(&mut self, class: Class) {
        self.classes.push(class);
    }

    pub fn add_interface(&mut self, iface: Interface) {
        self.interfaces.push(iface);
    }

    pub fn add_statement(&mut self, stmt: Stmt) {
        self.statements.push(stmt);
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}
