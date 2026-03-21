// ============================================================
// ADead-BIB — Unified Calling Convention Table
// ============================================================
// Consolidates x86-64 ABI calling conventions from all three
// toolchains into a single authoritative reference:
//
//   Win64   (MSVC / Clang-cl)  — Windows x64 ABI
//   SysV    (GCC / Clang)      — Linux / macOS x64 ABI
//   cdecl   x86-32 default     — legacy / cross-platform
//   stdcall Win32 API standard — legacy / cross-platform
//   fastcall __fastcall x86-32 — legacy ECX/EDX
//   vectorcall Win vectorcall  — vector-heavy code
//   naked   No frame generated — hand-written ASM helpers
//
// Reference:
//   Win64:  https://docs.microsoft.com/cpp/build/x64-calling-convention
//   SysV:   https://gitlab.com/x86-psABIs/x86-64-ABI
// ============================================================

/// All calling conventions supported by ADead-BIB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallingConvention {
    // ── x86-64 ──────────────────────────────────────────────
    /// Microsoft Win64 ABI (the default on Windows x64).
    Win64,
    /// System V AMD64 ABI (default on Linux/macOS x64).
    SysV,

    // ── x86-32 (legacy, emitted in 32-bit flat binaries) ───
    /// x86 cdecl — caller cleans stack; default for C.
    Cdecl,
    /// x86 stdcall — callee cleans stack; Win32 API convention.
    Stdcall,
    /// x86 fastcall — ECX/EDX carry first two args.
    Fastcall,
    /// MSVC vectorcall — XMM0-5 carry vector args.
    Vectorcall,
    /// x86/x64 thiscall — ECX/RCX carries `this`.
    Thiscall,

    // ── LLVM-specific ───────────────────────────────────────
    /// LLVM FastCC — used for intra-module optimised calls.
    LlvmFast,
    /// LLVM ColdCC — rarely-called paths.
    LlvmCold,

    // ── Special ─────────────────────────────────────────────
    /// No prologue/epilogue; used for hand-written assembly stubs.
    Naked,
    /// Platform default (resolved to Win64/SysV at build time).
    Default,
}

impl CallingConvention {
    /// Resolve `Default` to the actual platform convention.
    pub fn resolve(self) -> Self {
        if let Self::Default = self {
            if cfg!(target_os = "windows") {
                Self::Win64
            } else {
                Self::SysV
            }
        } else {
            self
        }
    }
}

// ── Register definitions ────────────────────────────────────────────────────

/// x86-64 integer/pointer register indices (used in call frames).
///
/// Indices match the ModR/M encoding: 0=RAX … 15=R15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reg(pub u8);

impl Reg {
    pub const RAX: Reg = Reg(0);
    pub const RCX: Reg = Reg(1);
    pub const RDX: Reg = Reg(2);
    pub const RBX: Reg = Reg(3);
    pub const RSP: Reg = Reg(4);
    pub const RBP: Reg = Reg(5);
    pub const RSI: Reg = Reg(6);
    pub const RDI: Reg = Reg(7);
    pub const R8: Reg = Reg(8);
    pub const R9: Reg = Reg(9);
    pub const R10: Reg = Reg(10);
    pub const R11: Reg = Reg(11);
    pub const R12: Reg = Reg(12);
    pub const R13: Reg = Reg(13);
    pub const R14: Reg = Reg(14);
    pub const R15: Reg = Reg(15);

    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "rax",
            1 => "rcx",
            2 => "rdx",
            3 => "rbx",
            4 => "rsp",
            5 => "rbp",
            6 => "rsi",
            7 => "rdi",
            8 => "r8",
            9 => "r9",
            10 => "r10",
            11 => "r11",
            12 => "r12",
            13 => "r13",
            14 => "r14",
            15 => "r15",
            n => {
                let _ = n;
                "??"
            }
        }
    }
}

// ── Call Frame Description ──────────────────────────────────────────────────

/// Describes the ABI properties of a calling convention.
///
/// The backend uses these tables to generate correct prologues, argument
/// passing, and return sequences without per-target special-casing.
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Name of the convention (for debug output).
    pub name: &'static str,
    /// Integer/pointer argument registers, in order.
    pub int_param_regs: &'static [Reg],
    /// Floating-point argument registers (XMM), in order.
    pub fp_param_regs: &'static [u8], // XMM index 0..7
    /// Return value in integer register.
    pub int_return_reg: Reg,
    /// Return value in XMM register (index).
    pub fp_return_xmm: u8,
    /// Registers that the *callee* must preserve.
    pub callee_saved: &'static [Reg],
    /// Shadow space (bytes) that the *caller* allocates before the call.
    /// Win64: 32 bytes.  SysV: 0 bytes.
    pub shadow_bytes: usize,
    /// Red zone (bytes) below RSP the callee may use without adjusting.
    /// Win64: 0 bytes.  SysV: 128 bytes.
    pub red_zone_bytes: usize,
    /// Whether the callee cleans argument bytes from the stack.
    pub callee_cleanup: bool,
    /// Alignment requirement for the stack pointer on call entry (bytes).
    pub stack_align: u8,
}

// ── Static frame tables ─────────────────────────────────────────────────────

static WIN64_INT_PARAMS: &[Reg] = &[Reg::RCX, Reg::RDX, Reg::R8, Reg::R9];
static SYSV_INT_PARAMS: &[Reg] = &[Reg::RDI, Reg::RSI, Reg::RDX, Reg::RCX, Reg::R8, Reg::R9];

static WIN64_CALLEE_SAVED: &[Reg] = &[
    Reg::RBX,
    Reg::RBP,
    Reg::RDI,
    Reg::RSI,
    Reg::R12,
    Reg::R13,
    Reg::R14,
    Reg::R15,
];
static SYSV_CALLEE_SAVED: &[Reg] = &[Reg::RBX, Reg::RBP, Reg::R12, Reg::R13, Reg::R14, Reg::R15];
static CDECL_CALLEE_SAVED: &[Reg] = &[Reg::RBX, Reg::RBP, Reg::RSI, Reg::RDI];

/// Pre-built call frame for Win64.
pub const FRAME_WIN64: CallFrame = CallFrame {
    name: "Win64",
    int_param_regs: WIN64_INT_PARAMS,
    fp_param_regs: &[0, 1, 2, 3], // XMM0-3
    int_return_reg: Reg::RAX,
    fp_return_xmm: 0, // XMM0
    callee_saved: WIN64_CALLEE_SAVED,
    shadow_bytes: 32,
    red_zone_bytes: 0,
    callee_cleanup: false,
    stack_align: 16,
};

/// Pre-built call frame for System V AMD64.
pub const FRAME_SYSV: CallFrame = CallFrame {
    name: "SysV AMD64",
    int_param_regs: SYSV_INT_PARAMS,
    fp_param_regs: &[0, 1, 2, 3, 4, 5, 6, 7], // XMM0-7
    int_return_reg: Reg::RAX,
    fp_return_xmm: 0,
    callee_saved: SYSV_CALLEE_SAVED,
    shadow_bytes: 0,
    red_zone_bytes: 128,
    callee_cleanup: false,
    stack_align: 16,
};

/// Pre-built call frame for x86 cdecl.
pub const FRAME_CDECL: CallFrame = CallFrame {
    name: "cdecl",
    int_param_regs: &[], // All args on stack
    fp_param_regs: &[],
    int_return_reg: Reg::RAX, // EAX in 32-bit mode
    fp_return_xmm: 0,
    callee_saved: CDECL_CALLEE_SAVED,
    shadow_bytes: 0,
    red_zone_bytes: 0,
    callee_cleanup: false,
    stack_align: 4,
};

/// Pre-built call frame for x86 __stdcall.
pub const FRAME_STDCALL: CallFrame = CallFrame {
    name: "stdcall",
    int_param_regs: &[],
    fp_param_regs: &[],
    int_return_reg: Reg::RAX,
    fp_return_xmm: 0,
    callee_saved: CDECL_CALLEE_SAVED,
    shadow_bytes: 0,
    red_zone_bytes: 0,
    callee_cleanup: true,
    stack_align: 4,
};

// ── API Functions ───────────────────────────────────────────────────────────

/// Retrieve the call frame description for a given convention.
pub fn frame_for(conv: CallingConvention) -> &'static CallFrame {
    match conv.resolve() {
        CallingConvention::Win64 | CallingConvention::Thiscall => &FRAME_WIN64,
        CallingConvention::SysV | CallingConvention::LlvmFast | CallingConvention::LlvmCold => {
            &FRAME_SYSV
        }
        CallingConvention::Cdecl => &FRAME_CDECL,
        CallingConvention::Stdcall | CallingConvention::Fastcall => &FRAME_STDCALL,
        _ => {
            // Vectorcall, Naked, Default (post-resolve shouldn't happen) — fall back
            if cfg!(target_os = "windows") {
                &FRAME_WIN64
            } else {
                &FRAME_SYSV
            }
        }
    }
}

/// Return the number of shadow bytes a caller must allocate for this convention.
pub fn shadow_space(conv: CallingConvention) -> usize {
    frame_for(conv).shadow_bytes
}

/// Infer the calling convention from a list of attribute tokens.
///
/// Inspects `__cdecl`, `__stdcall`, `__fastcall`, `__vectorcall`,
/// `__thiscall`, `__attribute__((cdecl))`, etc.
pub fn detect_convention(attrs: &[&str]) -> CallingConvention {
    for a in attrs {
        let a_lower = a.to_lowercase();
        let a_trimmed = a_lower.trim_matches('_');
        match a_trimmed {
            "cdecl" => return CallingConvention::Cdecl,
            "stdcall" => return CallingConvention::Stdcall,
            "fastcall" => return CallingConvention::Fastcall,
            "vectorcall" => return CallingConvention::Vectorcall,
            "thiscall" => return CallingConvention::Thiscall,
            "win64cc" => return CallingConvention::Win64,
            "sysv" | "sysvcc" => return CallingConvention::SysV,
            "naked" => return CallingConvention::Naked,
            _ => {}
        }
    }
    CallingConvention::Default
}

/// Emit the `PUSH rbp / MOV rbp, rsp` standard frame prologue bytes.
///
/// Returns raw x86-64 bytes for the standard function prologue.
pub fn emit_standard_prologue() -> Vec<u8> {
    vec![
        0x55, // PUSH RBP
        0x48, 0x89, 0xE5, // MOV RBP, RSP
    ]
}

/// Emit the `POP rbp / RET` standard frame epilogue bytes.
pub fn emit_standard_epilogue() -> Vec<u8> {
    vec![
        0x5D, // POP RBP
        0xC3, // RET
    ]
}

/// Emit Win64 shadow-space allocation: `SUB RSP, 32`.
pub fn emit_win64_shadow_alloc() -> Vec<u8> {
    // SUB RSP, 0x20   (imm8 form: REX.W + 83 /5 ib)
    vec![0x48, 0x83, 0xEC, 0x20]
}
