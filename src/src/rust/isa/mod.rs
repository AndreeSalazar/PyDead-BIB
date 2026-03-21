// ============================================================
// ADead-BIB ISA Layer Abstraction
// ============================================================
// Representación estructurada de instrucciones x86-64.
//
// En lugar de emitir bytes directamente (emit_bytes(&[0x55])),
// construimos una IR tipada: ADeadOp::Push { src: Reg(RBP) }
//
// Flujo: AST → ADeadIR (Vec<ADeadOp>) → Encoder → Bytes
//
// Esto permite:
// - Validación de instrucciones en tiempo de compilación
// - Optimizaciones sobre la IR antes de emitir bytes
// - Multi-target sin reescribir codegen completo
// - Debugging legible (print de instrucciones, no hex)
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

pub mod bit_resolver;
pub mod c_isa;
pub mod codegen;
pub mod compiler;
pub mod cpp_isa;
pub mod decoder;
pub mod encoder;
pub mod isa_compiler;
pub mod optimizer;
pub mod reg_alloc;
pub mod soa_optimizer;
pub mod vex_emitter;
pub mod ymm_allocator;

// Re-export modular compiler
pub use compiler::{
    CpuMode as CpuModeModular, IsaCompiler as IsaCompilerModular, Target as TargetModular,
};

// ============================================================
// Registers
// ============================================================

/// Registros x86-64 usados por el compilador ADead-BIB.
///
/// Incluye registros de propósito general (64-bit, 32-bit, 8-bit)
/// y registros SSE para operaciones de punto flotante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reg {
    // 64-bit general purpose
    RAX,
    RBX,
    RCX,
    RDX,
    RSI,
    RDI,
    RBP,
    RSP,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,

    // 32-bit (sub-registers)
    EAX,
    EBX,
    ECX,
    EDX,
    ESI,
    EDI,
    ESP,
    EBP,

    // 16-bit (sub-registers)
    AX,
    BX,
    CX,
    DX,
    SI,
    DI,
    SP,
    BP,

    // 8-bit (sub-registers)
    AL,
    AH,
    BL,
    BH,
    CL,
    CH,
    DL,
    DH,

    // SSE/AVX registers (128-bit XMM / 256-bit YMM)
    XMM0,
    XMM1,
    XMM2,
    XMM3,
    XMM4,
    XMM5,
    XMM6,
    XMM7,
    XMM8,
    XMM9,
    XMM10,
    XMM11,
    XMM12,
    XMM13,
    XMM14,
    XMM15,

    // AVX2 256-bit registers (v8.0)
    YMM0,
    YMM1,
    YMM2,
    YMM3,
    YMM4,
    YMM5,
    YMM6,
    YMM7,
    YMM8,
    YMM9,
    YMM10,
    YMM11,
    YMM12,
    YMM13,
    YMM14,
    YMM15,

    // Control registers (OS-level)
    CR0,
    CR2,
    CR3,
    CR4,

    // Debug registers
    DR0,
    DR1,
    DR2,
    DR3,
    DR6,
    DR7,

    // Segment registers
    CS,
    DS,
    ES,
    FS,
    GS,
    SS,
}

impl Reg {
    /// Retorna true si el registro es de 64 bits.
    pub fn is_64bit(&self) -> bool {
        matches!(
            self,
            Reg::RAX
                | Reg::RBX
                | Reg::RCX
                | Reg::RDX
                | Reg::RSI
                | Reg::RDI
                | Reg::RBP
                | Reg::RSP
                | Reg::R8
                | Reg::R9
                | Reg::R10
                | Reg::R11
                | Reg::R12
                | Reg::R13
                | Reg::R14
                | Reg::R15
        )
    }

    /// Retorna true si el registro es de 16 bits.
    pub fn is_16bit(&self) -> bool {
        matches!(
            self,
            Reg::AX | Reg::BX | Reg::CX | Reg::DX | Reg::SI | Reg::DI | Reg::SP | Reg::BP
        )
    }

    /// Retorna true si es un registro de control.
    pub fn is_control(&self) -> bool {
        matches!(self, Reg::CR0 | Reg::CR2 | Reg::CR3 | Reg::CR4)
    }

    /// Retorna true si es un registro de segmento.
    pub fn is_segment(&self) -> bool {
        matches!(
            self,
            Reg::CS | Reg::DS | Reg::ES | Reg::FS | Reg::GS | Reg::SS
        )
    }

    /// Retorna true si es un registro de debug.
    pub fn is_debug(&self) -> bool {
        matches!(
            self,
            Reg::DR0 | Reg::DR1 | Reg::DR2 | Reg::DR3 | Reg::DR6 | Reg::DR7
        )
    }

    /// Retorna true si el registro es de 32 bits.
    pub fn is_32bit(&self) -> bool {
        matches!(
            self,
            Reg::EAX | Reg::EBX | Reg::ECX | Reg::EDX | Reg::ESI | Reg::EDI | Reg::ESP | Reg::EBP
        )
    }

    /// Retorna true si el registro es de 8 bits.
    pub fn is_8bit(&self) -> bool {
        matches!(
            self,
            Reg::AL | Reg::AH | Reg::BL | Reg::BH | Reg::CL | Reg::CH | Reg::DL | Reg::DH
        )
    }

    /// Retorna true si es un registro SSE/XMM (128-bit).
    pub fn is_xmm(&self) -> bool {
        matches!(
            self,
            Reg::XMM0 | Reg::XMM1 | Reg::XMM2 | Reg::XMM3
                | Reg::XMM4 | Reg::XMM5 | Reg::XMM6 | Reg::XMM7
                | Reg::XMM8 | Reg::XMM9 | Reg::XMM10 | Reg::XMM11
                | Reg::XMM12 | Reg::XMM13 | Reg::XMM14 | Reg::XMM15
        )
    }

    /// Retorna true si es un registro AVX2/YMM (256-bit). v8.0
    pub fn is_ymm(&self) -> bool {
        matches!(
            self,
            Reg::YMM0 | Reg::YMM1 | Reg::YMM2 | Reg::YMM3
                | Reg::YMM4 | Reg::YMM5 | Reg::YMM6 | Reg::YMM7
                | Reg::YMM8 | Reg::YMM9 | Reg::YMM10 | Reg::YMM11
                | Reg::YMM12 | Reg::YMM13 | Reg::YMM14 | Reg::YMM15
        )
    }

    /// Retorna true si es un registro vectorial (XMM o YMM). v8.0
    pub fn is_vector(&self) -> bool {
        self.is_xmm() || self.is_ymm()
    }
}

impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Reg::RAX => "rax",
            Reg::RBX => "rbx",
            Reg::RCX => "rcx",
            Reg::RDX => "rdx",
            Reg::RSI => "rsi",
            Reg::RDI => "rdi",
            Reg::RBP => "rbp",
            Reg::RSP => "rsp",
            Reg::R8 => "r8",
            Reg::R9 => "r9",
            Reg::R10 => "r10",
            Reg::R11 => "r11",
            Reg::R12 => "r12",
            Reg::R13 => "r13",
            Reg::R14 => "r14",
            Reg::R15 => "r15",
            Reg::EAX => "eax",
            Reg::EBX => "ebx",
            Reg::ECX => "ecx",
            Reg::EDX => "edx",
            Reg::ESI => "esi",
            Reg::EDI => "edi",
            Reg::ESP => "esp",
            Reg::EBP => "ebp",
            Reg::AX => "ax",
            Reg::BX => "bx",
            Reg::CX => "cx",
            Reg::DX => "dx",
            Reg::SI => "si",
            Reg::DI => "di",
            Reg::SP => "sp",
            Reg::BP => "bp",
            Reg::AL => "al",
            Reg::AH => "ah",
            Reg::BL => "bl",
            Reg::BH => "bh",
            Reg::CL => "cl",
            Reg::CH => "ch",
            Reg::DL => "dl",
            Reg::DH => "dh",
            Reg::XMM0 => "xmm0",
            Reg::XMM1 => "xmm1",
            Reg::XMM2 => "xmm2",
            Reg::XMM3 => "xmm3",
            Reg::XMM4 => "xmm4",
            Reg::XMM5 => "xmm5",
            Reg::XMM6 => "xmm6",
            Reg::XMM7 => "xmm7",
            Reg::XMM8 => "xmm8",
            Reg::XMM9 => "xmm9",
            Reg::XMM10 => "xmm10",
            Reg::XMM11 => "xmm11",
            Reg::XMM12 => "xmm12",
            Reg::XMM13 => "xmm13",
            Reg::XMM14 => "xmm14",
            Reg::XMM15 => "xmm15",
            Reg::YMM0 => "ymm0",
            Reg::YMM1 => "ymm1",
            Reg::YMM2 => "ymm2",
            Reg::YMM3 => "ymm3",
            Reg::YMM4 => "ymm4",
            Reg::YMM5 => "ymm5",
            Reg::YMM6 => "ymm6",
            Reg::YMM7 => "ymm7",
            Reg::YMM8 => "ymm8",
            Reg::YMM9 => "ymm9",
            Reg::YMM10 => "ymm10",
            Reg::YMM11 => "ymm11",
            Reg::YMM12 => "ymm12",
            Reg::YMM13 => "ymm13",
            Reg::YMM14 => "ymm14",
            Reg::YMM15 => "ymm15",
            Reg::CR0 => "cr0",
            Reg::CR2 => "cr2",
            Reg::CR3 => "cr3",
            Reg::CR4 => "cr4",
            Reg::DR0 => "dr0",
            Reg::DR1 => "dr1",
            Reg::DR2 => "dr2",
            Reg::DR3 => "dr3",
            Reg::DR6 => "dr6",
            Reg::DR7 => "dr7",
            Reg::CS => "cs",
            Reg::DS => "ds",
            Reg::ES => "es",
            Reg::FS => "fs",
            Reg::GS => "gs",
            Reg::SS => "ss",
        };
        write!(f, "{}", name)
    }
}

// ============================================================
// Operands
// ============================================================

/// Operandos de instrucciones x86-64.
///
/// Cubre todos los modos de direccionamiento:
/// - Registro directo
/// - Inmediatos (8, 16, 32, 64 bits)
/// - Memoria con base + desplazamiento: `[rbp + disp]`
/// - Memoria con index y scale: `[base + index*scale + disp]` (arrays)
/// - RIP-relative: `[rip + disp]` (para IAT/tablas)
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Registro directo: rax, rbx, etc.
    Reg(Reg),
    /// Inmediato de 64 bits (mov rax, imm64)
    Imm64(u64),
    /// Inmediato de 32 bits (mov eax, imm32 / sub rsp, imm32)
    Imm32(i32),
    /// Inmediato de 16 bits (mov ax, imm16)
    Imm16(i16),
    /// Inmediato de 8 bits (shl rax, 3 / sub rsp, 32)
    Imm8(i8),
    /// Memoria simple: [base + disp] (ej: [rbp - 8])
    Mem { base: Reg, disp: i32 },
    /// Memoria con SIB: [base + index*scale + disp] (arrays, structs)
    /// scale must be 1, 2, 4, or 8
    MemSIB {
        base: Reg,
        index: Reg,
        scale: u8,
        disp: i32,
    },
    /// RIP-relative: [rip + disp] (para call indirecto via IAT)
    RipRel(i32),
}

impl std::fmt::Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Reg(r) => write!(f, "{}", r),
            Operand::Imm64(v) => write!(f, "0x{:X}", v),
            Operand::Imm32(v) => write!(f, "{}", v),
            Operand::Imm16(v) => write!(f, "{}", v),
            Operand::Imm8(v) => write!(f, "{}", v),
            Operand::Mem { base, disp } => {
                if *disp >= 0 {
                    write!(f, "[{}+{}]", base, disp)
                } else {
                    write!(f, "[{}{}]", base, disp)
                }
            }
            Operand::MemSIB {
                base,
                index,
                scale,
                disp,
            } => {
                if *disp >= 0 {
                    write!(f, "[{}+{}*{}+{}]", base, index, scale, disp)
                } else {
                    write!(f, "[{}+{}*{}{}]", base, index, scale, disp)
                }
            }
            Operand::RipRel(disp) => write!(f, "[rip+{}]", disp),
        }
    }
}

// ============================================================
// Conditions (for Jcc, SetCC)
// ============================================================

/// Condiciones para saltos condicionales y set condicional.
///
/// Mapean directamente a los códigos de condición x86-64:
/// - Equal → ZF=1 (je/sete)
/// - Less → SF≠OF (jl/setl)
/// - etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// ZF=1 — je / sete
    Equal,
    /// ZF=0 — jne / setne
    NotEqual,
    /// SF≠OF — jl / setl
    Less,
    /// ZF=1 OR SF≠OF — jle / setle
    LessEq,
    /// ZF=0 AND SF=OF — jg / setg
    Greater,
    /// SF=OF — jge / setge
    GreaterEq,
    /// Incondicional (usado en Jmp genérico)
    Always,
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Condition::Equal => "e",
            Condition::NotEqual => "ne",
            Condition::Less => "l",
            Condition::LessEq => "le",
            Condition::Greater => "g",
            Condition::GreaterEq => "ge",
            Condition::Always => "",
        };
        write!(f, "{}", name)
    }
}

// ============================================================
// Labels
// ============================================================

/// Label para saltos y targets de call.
///
/// Identificador numérico único generado por `ADeadIR::new_label()`.
/// Se resuelve a un offset concreto durante la fase de encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".L{}", self.0)
    }
}

// ============================================================
// Call Target
// ============================================================

/// Target de una instrucción CALL.
///
/// - `Relative(Label)`: call a una función interna (rel32)
/// - `RipRelative(i32)`: call indirecto via IAT `[rip+disp]`
/// - `Name(String)`: call a función por nombre (se resuelve después)
#[derive(Debug, Clone, PartialEq)]
pub enum CallTarget {
    /// Call relativo a un label interno (call rel32)
    Relative(Label),
    /// Call indirecto via RIP-relative (call [rip+disp], para IAT)
    RipRelative(i32),
    /// Call a función por nombre (se resuelve después)
    Name(String),
    /// Call indirecto via registro (call rax) — function pointers
    Register(Reg),
}

impl std::fmt::Display for CallTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallTarget::Relative(label) => write!(f, "{}", label),
            CallTarget::RipRelative(disp) => write!(f, "[rip+{}]", disp),
            CallTarget::Name(name) => write!(f, "{}", name),
            CallTarget::Register(reg) => write!(f, "*{:?}", reg),
        }
    }
}

// ============================================================
// ADeadOp — Instruction Set
// ============================================================

/// Instrucción x86-64 en representación estructurada.
///
/// Cada variante corresponde a una o más instrucciones x86-64
/// que codegen_v2.rs emitía como bytes directos. Ahora se
/// construyen como datos tipados y se codifican en la fase de
/// encoding.
///
/// # Ejemplo
/// ```text
/// // Antes (codegen_v2):
/// emit_bytes(&[0x55]);                    // push rbp
/// emit_bytes(&[0x48, 0x89, 0xE5]);       // mov rbp, rsp
///
/// // Ahora (ISA layer):
/// ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RBP) });
/// ir.emit(ADeadOp::Mov { dst: Operand::Reg(Reg::RBP), src: Operand::Reg(Reg::RSP) });
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ADeadOp {
    // ---- Data Movement ----
    /// MOV dst, src — Movimiento de datos entre registros, memoria e inmediatos
    Mov { dst: Operand, src: Operand },

    /// MOVZX dst, src — Zero-extend (ej: movzx rax, al)
    MovZx { dst: Reg, src: Reg },

    /// Store16: mov WORD [base+disp], reg — 16-bit store (0x66 prefix)
    /// Used for VGA text mode writes (each cell = char + attr = 2 bytes)
    Store16 { base: Reg, disp: i32, src: Reg },

    /// Store32: mov DWORD [base+disp], reg — 32-bit store (no REX.W)
    /// Used for writing 4-byte fields (GUID, D3D12 structs, etc.)
    Store32 { base: Reg, disp: i32, src: Reg },

    /// LEA dst, [base+disp] — Load effective address
    Lea { dst: Reg, src: Operand },

    // ---- Arithmetic ----
    /// ADD dst, src
    Add { dst: Operand, src: Operand },

    /// SUB dst, src
    Sub { dst: Operand, src: Operand },

    /// IMUL dst, src — Multiplicación con signo
    Mul { dst: Reg, src: Reg },

    /// IDIV src — División con signo (RDX:RAX / src → RAX, RDX)
    /// Implica CQO antes de la división.
    Div { src: Reg },

    /// AND dst, src — Bitwise AND
    And { dst: Reg, src: Reg },

    /// OR dst, src — Bitwise OR
    Or { dst: Reg, src: Reg },

    /// XOR dst, src — Bitwise XOR (también usado para zeroing: xor eax, eax)
    Xor { dst: Reg, src: Reg },

    /// INC dst — Incremento
    Inc { dst: Operand },

    /// DEC dst — Decremento
    Dec { dst: Operand },

    /// NEG dst — Negación aritmética (two's complement)
    Neg { dst: Reg },

    /// NOT lógico — Implementado como test+sete+movzx
    Not { dst: Reg },

    /// SHL dst, amount — Shift left
    Shl { dst: Reg, amount: u8 },

    // ---- Comparison & Flags ----
    /// CMP left, right — Comparación (sets flags)
    Cmp { left: Operand, right: Operand },

    /// TEST left, right — AND lógico sin guardar resultado (sets flags)
    Test { left: Reg, right: Reg },

    /// SETcc dst — Set byte según condición (ej: sete al)
    SetCC { cond: Condition, dst: Reg },

    // ---- Stack ----
    /// PUSH src — Push al stack
    Push { src: Operand },

    /// POP dst — Pop del stack
    Pop { dst: Reg },

    // ---- Control Flow ----
    /// CALL target — Llamada a función
    Call { target: CallTarget },

    /// JMP target — Salto incondicional
    Jmp { target: Label },

    /// Jcc target — Salto condicional
    Jcc { cond: Condition, target: Label },

    /// RET — Retorno de función
    Ret,

    /// SYSCALL — Llamada al sistema (Linux)
    Syscall,

    // ---- SSE / Floating Point ----
    /// CVTSI2SD dst, src — Convertir entero a double (int → xmm)
    CvtSi2Sd { dst: Reg, src: Reg },

    /// MOVQ dst, src — Mover entre registro GP y XMM (64-bit)
    MovQ { dst: Reg, src: Reg },

    // ---- Pseudo-instructions ----
    /// Pseudo-instrucción: marca la posición de un label.
    /// No emite bytes, solo registra el offset para resolución de saltos.
    Label(Label),

    /// NOP — No operation
    Nop,

    /// Escape hatch: bytes crudos para casos no cubiertos.
    /// Usar solo cuando no existe una variante tipada equivalente.
    RawBytes(Vec<u8>),

    /// Call indirecto via IAT (Import Address Table) para Windows.
    /// El encoder calcula el offset RIP-relative automáticamente.
    /// iat_rva: RVA del slot IAT (ej: 0x2040 para printf, 0x2048 para scanf)
    CallIAT { iat_rva: u32 },

    // ================================================================
    // OS-Level / Privileged Instructions (ADead-BIB v3.1-OS)
    // ================================================================
    /// CLI — Clear Interrupt Flag (disable interrupts)
    Cli,

    /// STI — Set Interrupt Flag (enable interrupts)
    Sti,

    /// HLT — Halt CPU (wait for interrupt)
    Hlt,

    /// IRETQ — Return from interrupt (64-bit)
    Iret,

    /// INT n — Software interrupt (e.g., INT 0x10 for BIOS, INT 0x80 for Linux syscall)
    Int { vector: u8 },

    /// LGDT [mem] — Load Global Descriptor Table register
    Lgdt { src: Operand },

    /// LIDT [mem] — Load Interrupt Descriptor Table register
    Lidt { src: Operand },

    /// MOV CRn, reg — Write to control register
    MovToCr { cr: u8, src: Reg },

    /// MOV reg, CRn — Read from control register
    MovFromCr { cr: u8, dst: Reg },

    /// CPUID — CPU identification
    Cpuid,

    /// RDMSR — Read Model Specific Register (ECX=index, result in EDX:EAX)
    Rdmsr,

    /// WRMSR — Write Model Specific Register (ECX=index, value in EDX:EAX)
    Wrmsr,

    /// INVLPG [addr] — Invalidate TLB entry for page containing addr
    Invlpg { addr: Operand },

    /// IN AL, imm8 / IN AL, DX — Read byte from I/O port
    InByte { port: Operand },

    /// OUT imm8, AL / OUT DX, AL — Write byte to I/O port
    OutByte { port: Operand, src: Operand },

    /// IN EAX, imm8 / IN EAX, DX — Read dword from I/O port
    InDword { port: Operand },

    /// OUT imm8, EAX / OUT DX, EAX — Write dword to I/O port
    OutDword { port: Operand, src: Operand },

    /// SHR dst, amount — Shift right logical
    Shr { dst: Reg, amount: u8 },

    /// Bitwise NOT: ~x (NOT dst — one's complement)
    BitwiseNot { dst: Reg },

    /// SHL dst, CL — Shift left by variable amount in CL register
    ShlCl { dst: Reg },

    /// SHR dst, CL — Shift right by variable amount in CL register
    ShrCl { dst: Reg },

    /// Far JMP — Jump with segment selector change (for mode switching)
    FarJmp { selector: u16, offset: u32 },

    /// LEA reg, [rip+label] — Load effective address of a label into register
    /// Used for function pointers: fn_ptr = some_function
    LeaLabel { dst: Reg, label: Label },

    /// Label address reference — emits the absolute address of a label as bytes
    /// Used for writing label addresses to memory (e.g., for far jump pointers)
    /// The encoder resolves this to the actual address after all labels are placed.
    LabelAddrRef {
        label: Label,
        /// Size of the address to emit (2 = word, 4 = dword)
        size: u8,
        /// Base address to add (e.g., 0x8000 for stage2 loaded at 0x8000)
        base_addr: u32,
    },
}

impl std::fmt::Display for ADeadOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ADeadOp::Mov { dst, src } => write!(f, "mov {}, {}", dst, src),
            ADeadOp::MovZx { dst, src } => write!(f, "movzx {}, {}", dst, src),
            ADeadOp::Store16 { base, disp, src } => write!(f, "mov word [{}{:+}], {}", base, disp, src),
            ADeadOp::Store32 { base, disp, src } => write!(f, "mov dword [{}{:+}], {}", base, disp, src),
            ADeadOp::Lea { dst, src } => write!(f, "lea {}, {}", dst, src),
            ADeadOp::Add { dst, src } => write!(f, "add {}, {}", dst, src),
            ADeadOp::Sub { dst, src } => write!(f, "sub {}, {}", dst, src),
            ADeadOp::Mul { dst, src } => write!(f, "imul {}, {}", dst, src),
            ADeadOp::Div { src } => write!(f, "cqo; idiv {}", src),
            ADeadOp::And { dst, src } => write!(f, "and {}, {}", dst, src),
            ADeadOp::Or { dst, src } => write!(f, "or {}, {}", dst, src),
            ADeadOp::Xor { dst, src } => write!(f, "xor {}, {}", dst, src),
            ADeadOp::Inc { dst } => write!(f, "inc {}", dst),
            ADeadOp::Dec { dst } => write!(f, "dec {}", dst),
            ADeadOp::Neg { dst } => write!(f, "neg {}", dst),
            ADeadOp::Not { dst } => write!(f, "not.logical {}", dst),
            ADeadOp::Shl { dst, amount } => write!(f, "shl {}, {}", dst, amount),
            ADeadOp::Cmp { left, right } => write!(f, "cmp {}, {}", left, right),
            ADeadOp::Test { left, right } => write!(f, "test {}, {}", left, right),
            ADeadOp::SetCC { cond, dst } => write!(f, "set{} {}", cond, dst),
            ADeadOp::Push { src } => write!(f, "push {}", src),
            ADeadOp::Pop { dst } => write!(f, "pop {}", dst),
            ADeadOp::Call { target } => write!(f, "call {}", target),
            ADeadOp::Jmp { target } => write!(f, "jmp {}", target),
            ADeadOp::Jcc { cond, target } => write!(f, "j{} {}", cond, target),
            ADeadOp::Ret => write!(f, "ret"),
            ADeadOp::Syscall => write!(f, "syscall"),
            ADeadOp::CvtSi2Sd { dst, src } => write!(f, "cvtsi2sd {}, {}", dst, src),
            ADeadOp::MovQ { dst, src } => write!(f, "movq {}, {}", dst, src),
            ADeadOp::Label(label) => write!(f, "{}:", label),
            ADeadOp::Nop => write!(f, "nop"),
            ADeadOp::RawBytes(bytes) => {
                write!(f, "db ")?;
                for (i, b) in bytes.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "0x{:02X}", b)?;
                }
                Ok(())
            }
            ADeadOp::CallIAT { iat_rva } => write!(f, "call [iat:0x{:04X}]", iat_rva),
            // OS-Level instructions
            ADeadOp::Cli => write!(f, "cli"),
            ADeadOp::Sti => write!(f, "sti"),
            ADeadOp::Hlt => write!(f, "hlt"),
            ADeadOp::Iret => write!(f, "iretq"),
            ADeadOp::Int { vector } => write!(f, "int 0x{:02X}", vector),
            ADeadOp::Lgdt { src } => write!(f, "lgdt {}", src),
            ADeadOp::Lidt { src } => write!(f, "lidt {}", src),
            ADeadOp::MovToCr { cr, src } => write!(f, "mov cr{}, {}", cr, src),
            ADeadOp::MovFromCr { cr, dst } => write!(f, "mov {}, cr{}", dst, cr),
            ADeadOp::Cpuid => write!(f, "cpuid"),
            ADeadOp::Rdmsr => write!(f, "rdmsr"),
            ADeadOp::Wrmsr => write!(f, "wrmsr"),
            ADeadOp::Invlpg { addr } => write!(f, "invlpg {}", addr),
            ADeadOp::InByte { port } => write!(f, "in al, {}", port),
            ADeadOp::OutByte { port, src } => write!(f, "out {}, {}", port, src),
            ADeadOp::InDword { port } => write!(f, "in eax, {}", port),
            ADeadOp::OutDword { port, src } => write!(f, "out {}, {}", port, src),
            ADeadOp::Shr { dst, amount } => write!(f, "shr {}, {}", dst, amount),
            ADeadOp::BitwiseNot { dst } => write!(f, "not {}", dst),
            ADeadOp::ShlCl { dst } => write!(f, "shl {}, cl", dst),
            ADeadOp::ShrCl { dst } => write!(f, "shr {}, cl", dst),
            ADeadOp::FarJmp { selector, offset } => {
                write!(f, "jmp 0x{:04X}:0x{:08X}", selector, offset)
            }
            ADeadOp::LeaLabel { dst, label } => {
                write!(f, "lea {:?}, [rip+{}]", dst, label)
            }
            ADeadOp::LabelAddrRef {
                label,
                size,
                base_addr,
            } => {
                write!(
                    f,
                    "label_addr({}, size={}, base=0x{:X})",
                    label, size, base_addr
                )
            }
        }
    }
}

// ============================================================
// ADeadIR — Instruction Buffer
// ============================================================

/// Buffer de instrucciones ISA para el compilador ADead-BIB.
///
/// Acumula instrucciones `ADeadOp` en orden, gestiona labels
/// para saltos, y mantiene una tabla de strings.
///
/// # Uso
/// ```text
/// let mut ir = ADeadIR::new();
/// let loop_start = ir.new_label();
///
/// ir.emit(ADeadOp::Push { src: Operand::Reg(Reg::RBP) });
/// ir.emit(ADeadOp::Mov {
///     dst: Operand::Reg(Reg::RBP),
///     src: Operand::Reg(Reg::RSP),
/// });
/// ir.emit(ADeadOp::Label(loop_start));
/// ir.emit(ADeadOp::Jmp { target: loop_start });
/// ```
#[derive(Debug, Clone)]
pub struct ADeadIR {
    /// Instrucciones emitidas en orden
    ops: Vec<ADeadOp>,
    /// Contador de labels (cada new_label() incrementa)
    label_counter: u32,
    /// Tabla de strings (para datos estáticos referenciados por el código)
    string_table: Vec<String>,
}

impl ADeadIR {
    /// Crea un nuevo buffer de instrucciones vacío.
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            label_counter: 0,
            string_table: Vec::new(),
        }
    }

    /// Emite una instrucción al final del buffer.
    pub fn emit(&mut self, op: ADeadOp) {
        self.ops.push(op);
    }

    /// Genera un nuevo label único para saltos y targets.
    pub fn new_label(&mut self) -> Label {
        let id = self.label_counter;
        self.label_counter += 1;
        Label(id)
    }

    /// Retorna una referencia a las instrucciones emitidas.
    pub fn ops(&self) -> &[ADeadOp] {
        &self.ops
    }

    /// Retorna una referencia mutable a las instrucciones emitidas.
    pub fn ops_mut(&mut self) -> &mut Vec<ADeadOp> {
        &mut self.ops
    }

    /// Agrega un string a la tabla de strings y retorna su índice.
    pub fn add_string(&mut self, s: String) -> usize {
        let idx = self.string_table.len();
        self.string_table.push(s);
        idx
    }

    /// Retorna la tabla de strings.
    pub fn string_table(&self) -> &[String] {
        &self.string_table
    }

    /// Retorna el número total de instrucciones emitidas.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Retorna true si no se han emitido instrucciones.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

impl Default for ADeadIR {
    fn default() -> Self {
        Self::new()
    }
}
