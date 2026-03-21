// ============================================================
// ADead-BIB IR — PDP-11/VAX/x86/68000 Heritage Extensions
// ============================================================
//
// C arquitecturas originales dieron estas lecciones:
//
// PDP-11  → punteros son hardware (auto-increment/decrement)
// VAX     → stack frames son reales (formal calling convention)
// x86     → memoria es plana y directa (segment-aware)
// 68000   → registros tienen propósito (typed registers)
//
// ADead-BIB toma esas lecciones para hacer el IR más fiel al C original.
// ============================================================

use super::{Type, Value};

#[allow(dead_code)]

/// PDP-11 Heritage: Addressing Modes
///
/// The PDP-11 had powerful addressing modes that C was designed around:
/// - Register direct: Rn
/// - Register deferred: (Rn)
/// - Auto-increment: (Rn)+  ← pointer++
/// - Auto-decrement: -(Rn)  ← --pointer
/// - Indexed: X(Rn)
/// - Immediate: #n
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    /// Direct register access
    Direct,

    /// Indirect through register (pointer dereference)
    Indirect,

    /// Auto-increment: access then increment (like *p++)
    /// PDP-11: MOV (R)+, dest
    AutoIncrement,

    /// Auto-decrement: decrement then access (like *--p)
    /// PDP-11: MOV -(R), dest
    AutoDecrement,

    /// Indexed: base + offset
    /// PDP-11: MOV X(R), dest
    Indexed { offset: i32 },

    /// Immediate value
    Immediate,

    /// PC-relative (for position-independent code)
    PcRelative { offset: i32 },
}

/// VAX Heritage: Stack Frame Structure
///
/// VAX had formal stack frames with:
/// - Argument pointer (AP)
/// - Frame pointer (FP)
/// - Stack pointer (SP)
/// - Saved registers mask
/// - Return address
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Size of local variables area
    pub locals_size: u32,

    /// Size of arguments area
    pub args_size: u32,

    /// Registers that need to be saved
    pub saved_registers: Vec<u8>,

    /// Whether frame pointer is used
    pub uses_frame_pointer: bool,

    /// Alignment requirement
    pub alignment: u32,

    /// Red zone size (for leaf functions)
    pub red_zone: u32,
}

impl StackFrame {
    pub fn new() -> Self {
        StackFrame {
            locals_size: 0,
            args_size: 0,
            saved_registers: Vec::new(),
            uses_frame_pointer: true,
            alignment: 16, // x86-64 ABI
            red_zone: 128, // System V ABI
        }
    }

    /// Calculate total frame size
    pub fn total_size(&self) -> u32 {
        let saved_regs_size = (self.saved_registers.len() as u32) * 8;
        let total = self.locals_size + self.args_size + saved_regs_size + 8; // +8 for return addr
                                                                             // Align to required boundary
        (total + self.alignment - 1) & !(self.alignment - 1)
    }

    /// VAX-style CALLS instruction emulation
    /// Generates proper stack frame setup
    pub fn emit_prologue(&self) -> Vec<PdpInstruction> {
        let mut instrs = Vec::new();

        if self.uses_frame_pointer {
            // push rbp
            instrs.push(PdpInstruction::Push(Register::RBP));
            // mov rbp, rsp
            instrs.push(PdpInstruction::Move {
                src: Register::RSP,
                dst: Register::RBP,
                mode: AddressingMode::Direct,
            });
        }

        // sub rsp, frame_size
        if self.locals_size > 0 {
            instrs.push(PdpInstruction::SubImm {
                dst: Register::RSP,
                imm: self.locals_size as i64,
            });
        }

        // Save callee-saved registers
        for &reg in &self.saved_registers {
            instrs.push(PdpInstruction::Push(Register::from_index(reg)));
        }

        instrs
    }

    /// VAX-style RET instruction emulation
    pub fn emit_epilogue(&self) -> Vec<PdpInstruction> {
        let mut instrs = Vec::new();

        // Restore callee-saved registers (reverse order)
        for &reg in self.saved_registers.iter().rev() {
            instrs.push(PdpInstruction::Pop(Register::from_index(reg)));
        }

        if self.uses_frame_pointer {
            // mov rsp, rbp
            instrs.push(PdpInstruction::Move {
                src: Register::RBP,
                dst: Register::RSP,
                mode: AddressingMode::Direct,
            });
            // pop rbp
            instrs.push(PdpInstruction::Pop(Register::RBP));
        } else if self.locals_size > 0 {
            // add rsp, frame_size
            instrs.push(PdpInstruction::AddImm {
                dst: Register::RSP,
                imm: self.locals_size as i64,
            });
        }

        instrs.push(PdpInstruction::Ret);
        instrs
    }
}

/// x86 Heritage: Segment Awareness
///
/// x86 has segment registers for memory protection:
/// - CS: Code segment
/// - DS: Data segment
/// - SS: Stack segment
/// - ES/FS/GS: Extra segments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Code,  // CS
    Data,  // DS
    Stack, // SS
    Extra, // ES
    FS,    // Thread-local storage
    GS,    // Per-CPU data (kernel)
}

/// 68000 Heritage: Typed Registers
///
/// The 68000 had separate register files:
/// - D0-D7: Data registers
/// - A0-A7: Address registers (A7 = SP)
///
/// This influenced C's distinction between data and pointers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    // x86-64 GPRs (data-oriented, like 68000 Dn)
    RAX,
    RBX,
    RCX,
    RDX,
    RSI,
    RDI,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,

    // Stack/Frame pointers (address-oriented, like 68000 An)
    RSP,
    RBP,

    // Instruction pointer
    RIP,

    // Flags
    RFLAGS,

    // SSE registers (for floating point)
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
}

impl Register {
    pub fn from_index(idx: u8) -> Self {
        match idx {
            0 => Register::RAX,
            1 => Register::RCX,
            2 => Register::RDX,
            3 => Register::RBX,
            4 => Register::RSP,
            5 => Register::RBP,
            6 => Register::RSI,
            7 => Register::RDI,
            8 => Register::R8,
            9 => Register::R9,
            10 => Register::R10,
            11 => Register::R11,
            12 => Register::R12,
            13 => Register::R13,
            14 => Register::R14,
            15 => Register::R15,
            _ => Register::RAX,
        }
    }

    pub fn to_index(&self) -> u8 {
        match self {
            Register::RAX => 0,
            Register::RCX => 1,
            Register::RDX => 2,
            Register::RBX => 3,
            Register::RSP => 4,
            Register::RBP => 5,
            Register::RSI => 6,
            Register::RDI => 7,
            Register::R8 => 8,
            Register::R9 => 9,
            Register::R10 => 10,
            Register::R11 => 11,
            Register::R12 => 12,
            Register::R13 => 13,
            Register::R14 => 14,
            Register::R15 => 15,
            _ => 0,
        }
    }

    /// Is this a callee-saved register? (68000 heritage: some regs preserved)
    pub fn is_callee_saved(&self) -> bool {
        matches!(
            self,
            Register::RBX
                | Register::RBP
                | Register::R12
                | Register::R13
                | Register::R14
                | Register::R15
        )
    }

    /// Is this an address register? (68000 An heritage)
    pub fn is_address_register(&self) -> bool {
        matches!(
            self,
            Register::RSP | Register::RBP | Register::RSI | Register::RDI
        )
    }
}

/// PDP-11 Style Instructions
///
/// These map closely to PDP-11 instruction set, which C was designed for.
#[derive(Debug, Clone)]
pub enum PdpInstruction {
    /// MOV src, dst — with addressing mode
    Move {
        src: Register,
        dst: Register,
        mode: AddressingMode,
    },

    /// MOV #imm, dst — immediate to register
    MoveImm { dst: Register, imm: i64 },

    /// LOAD with auto-increment (like *p++)
    LoadAutoInc {
        dst: Register,
        ptr: Register,
        size: u8, // 1, 2, 4, 8 bytes
    },

    /// STORE with auto-decrement (like *--p = val)
    StoreAutoDec {
        src: Register,
        ptr: Register,
        size: u8,
    },

    /// ADD src, dst
    Add { src: Register, dst: Register },

    /// ADD #imm, dst
    AddImm { dst: Register, imm: i64 },

    /// SUB src, dst
    Sub { src: Register, dst: Register },

    /// SUB #imm, dst
    SubImm { dst: Register, imm: i64 },

    /// CMP src, dst
    Cmp { src: Register, dst: Register },

    /// PUSH reg (auto-decrement SP)
    Push(Register),

    /// POP reg (auto-increment SP)
    Pop(Register),

    /// JSR addr — Jump to Subroutine (VAX CALLS heritage)
    Call { target: String, args: Vec<Register> },

    /// RTS — Return from Subroutine
    Ret,

    /// BR label — Branch
    Branch { target: String },

    /// Bcc label — Conditional branch
    BranchCond {
        condition: Condition,
        target: String,
    },
}

/// Branch conditions (PDP-11/x86 heritage)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Equal,        // BEQ / JE
    NotEqual,     // BNE / JNE
    Less,         // BLT / JL
    LessEqual,    // BLE / JLE
    Greater,      // BGT / JG
    GreaterEqual, // BGE / JGE
    Below,        // unsigned: BLO / JB
    BelowEqual,   // unsigned: BLOS / JBE
    Above,        // unsigned: BHI / JA
    AboveEqual,   // unsigned: BHIS / JAE
    Negative,     // BMI / JS
    Positive,     // BPL / JNS
    Overflow,     // BVS / JO
    NoOverflow,   // BVC / JNO
    Carry,        // BCS / JC
    NoCarry,      // BCC / JNC
}

/// IR Extension: Auto-increment/decrement load/store
///
/// These capture PDP-11 semantics directly in the IR,
/// making C pointer arithmetic more efficient.
#[derive(Debug, Clone)]
pub enum MemoryOp {
    /// Standard load
    Load { ty: Type, ptr: Value },

    /// Load with post-increment: val = *p; p += size
    /// Maps to PDP-11: MOV (R)+, dst
    LoadPostInc {
        ty: Type,
        ptr: Value,
        increment: i32,
    },

    /// Load with pre-decrement: p -= size; val = *p
    /// Maps to PDP-11: MOV -(R), dst
    LoadPreDec {
        ty: Type,
        ptr: Value,
        decrement: i32,
    },

    /// Standard store
    Store { value: Value, ptr: Value },

    /// Store with post-increment: *p = val; p += size
    StorePostInc {
        value: Value,
        ptr: Value,
        increment: i32,
    },

    /// Store with pre-decrement: p -= size; *p = val
    StorePreDec {
        value: Value,
        ptr: Value,
        decrement: i32,
    },
}

impl MemoryOp {
    /// Convert C pointer operations to PDP-11 style
    ///
    /// Example: *p++ becomes LoadPostInc
    /// Example: *--p becomes LoadPreDec
    pub fn from_c_pointer_op(
        is_load: bool,
        is_pre: bool,
        is_increment: bool,
        ty: Type,
        ptr: Value,
        value: Option<Value>,
        size: i32,
    ) -> Self {
        match (is_load, is_pre, is_increment) {
            (true, false, true) => MemoryOp::LoadPostInc {
                ty,
                ptr,
                increment: size,
            },
            (true, true, false) => MemoryOp::LoadPreDec {
                ty,
                ptr,
                decrement: size,
            },
            (true, _, _) => MemoryOp::Load { ty, ptr },
            (false, false, true) => MemoryOp::StorePostInc {
                value: value.unwrap(),
                ptr,
                increment: size,
            },
            (false, true, false) => MemoryOp::StorePreDec {
                value: value.unwrap(),
                ptr,
                decrement: size,
            },
            (false, _, _) => MemoryOp::Store {
                value: value.unwrap(),
                ptr,
            },
        }
    }
}

/// Calling Convention (VAX/x86-64 heritage)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    /// System V AMD64 ABI (Linux, macOS)
    /// Args: RDI, RSI, RDX, RCX, R8, R9, then stack
    /// Return: RAX, RDX
    /// Callee-saved: RBX, RBP, R12-R15
    SysV,

    /// Microsoft x64 ABI (Windows)
    /// Args: RCX, RDX, R8, R9, then stack
    /// Return: RAX
    /// Callee-saved: RBX, RBP, RDI, RSI, R12-R15
    Win64,

    /// VAX-style (for compatibility)
    /// All args on stack, AP points to arg list
    Vax,

    /// Fastcall (first 2 args in registers)
    Fastcall,
}

impl CallingConvention {
    /// Get argument registers for this convention
    pub fn arg_registers(&self) -> &'static [Register] {
        match self {
            CallingConvention::SysV => &[
                Register::RDI,
                Register::RSI,
                Register::RDX,
                Register::RCX,
                Register::R8,
                Register::R9,
            ],
            CallingConvention::Win64 => &[Register::RCX, Register::RDX, Register::R8, Register::R9],
            CallingConvention::Vax => &[], // All on stack
            CallingConvention::Fastcall => &[Register::RCX, Register::RDX],
        }
    }

    /// Get return register
    pub fn return_register(&self) -> Register {
        Register::RAX
    }

    /// Get callee-saved registers
    pub fn callee_saved(&self) -> &'static [Register] {
        match self {
            CallingConvention::SysV => &[
                Register::RBX,
                Register::RBP,
                Register::R12,
                Register::R13,
                Register::R14,
                Register::R15,
            ],
            CallingConvention::Win64 => &[
                Register::RBX,
                Register::RBP,
                Register::RDI,
                Register::RSI,
                Register::R12,
                Register::R13,
                Register::R14,
                Register::R15,
            ],
            _ => &[Register::RBX, Register::RBP],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_frame() {
        let mut frame = StackFrame::new();
        frame.locals_size = 32;
        frame.saved_registers = vec![3, 12, 13]; // RBX, R12, R13

        assert!(frame.total_size() >= 32 + 24 + 8); // locals + 3 regs + ret addr

        let prologue = frame.emit_prologue();
        assert!(!prologue.is_empty());

        let epilogue = frame.emit_epilogue();
        assert!(!epilogue.is_empty());
    }

    #[test]
    fn test_addressing_modes() {
        let mode = AddressingMode::AutoIncrement;
        assert_eq!(mode, AddressingMode::AutoIncrement);

        let indexed = AddressingMode::Indexed { offset: 16 };
        if let AddressingMode::Indexed { offset } = indexed {
            assert_eq!(offset, 16);
        }
    }

    #[test]
    fn test_calling_convention() {
        let sysv = CallingConvention::SysV;
        assert_eq!(sysv.arg_registers().len(), 6);
        assert_eq!(sysv.return_register(), Register::RAX);

        let win64 = CallingConvention::Win64;
        assert_eq!(win64.arg_registers().len(), 4);
    }
}
