// ============================================================
// ADead-BIB — LLVM Heritage: Attributes, Intrinsics, Calling Conv
// ============================================================
// This module captures what ADead-BIB learns from LLVM:
//
//   • Function/parameter attributes as first-class IR values
//   • Named intrinsic functions (llvm.memcpy, llvm.bswap, …)
//   • LLVM-style calling conventions
//
// References:
//   https://llvm.org/docs/LangRef.html#function-attributes
//   https://llvm.org/docs/LangRef.html#intrinsic-functions
//   https://llvm.org/docs/LangRef.html#calling-conventions
// ============================================================

/// Function and parameter attributes inherited from LLVM IR.
///
/// These map directly to corresponding LLVM attributes and control
/// backend code-generation decisions (inlining, alias analysis, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LlvmAttribute {
    // ── Optimization hints ──────────────────────────────────
    /// Always inline this function call site.
    AlwaysInline,
    /// Never inline this function.
    NoInline,
    /// Optimize for size (prefer smaller code over speed).
    OptSize,
    /// Disable all optimisations on this function (`optnone`).
    OptNone,
    /// The function is cold (rarely called).
    Cold,
    /// The function is hot (frequently called).
    Hot,
    /// The function can be merged with identical functions.
    MergeableFunction,

    // ── Control flow / safety ───────────────────────────────
    /// Function never returns (e.g. `exit`, `abort`).
    NoReturn,
    /// Function never throws an exception.
    NoUnwind,
    /// No undefined behaviour due to memory operations (`willreturn`).
    WillReturn,
    /// Function may only be entered from a call instruction.
    Naked,

    // ── Memory semantics ────────────────────────────────────
    /// Pointer argument does not alias any other pointer.
    NoAlias,
    /// Pointer captures are not observed outside the function.
    NoCapture,
    /// Argument is read only inside the function.
    ReadOnly,
    /// Argument is write only inside the function.
    WriteOnly,
    /// Function only reads memory (no side effects on memory).
    ReadNone,
    /// Argument is returned (returned attribute).
    Returned,
    /// Argument is non-null.
    NonNull,
    /// Argument has a compile-time-known alignment, `align N`.
    Align(u64),
    /// Function returns a freshly-allocated pointer.
    NoAliasFn, // `noalias` on return value

    // ── Stack / calling ─────────────────────────────────────
    /// No red zone (used in kernel / signal handlers).
    NoRedZone,
    /// Use soft-float calling convention.
    SoftFloat,
    /// The function's stack frame is protected by a canary.
    StackProtect,
    StackProtectReq,
    StackProtectStrong,

    // ── Sanitisers / debug ──────────────────────────────────
    /// Sanitize address.
    SanitizeAddress,
    /// Sanitize memory.
    SanitizeMemory,
    /// Sanitize thread.
    SanitizeThread,
    /// Sanitize hardware address.
    SanitizeHWAddress,

    // ── Visibility ──────────────────────────────────────────
    /// Symbol is default-visibility.
    VisibilityDefault,
    /// Symbol is hidden — not exported.
    VisibilityHidden,
    /// Symbol is protected.
    VisibilityProtected,

    // ── Speculative execution ───────────────────────────────
    /// Speculative load hardening (Spectre mitigation).
    SpeculativeLoadHardening,
}

impl LlvmAttribute {
    /// Convert textual LLVM attribute name to enum variant.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "alwaysinline" => Some(Self::AlwaysInline),
            "noinline" => Some(Self::NoInline),
            "optsize" => Some(Self::OptSize),
            "optnone" => Some(Self::OptNone),
            "cold" => Some(Self::Cold),
            "hot" => Some(Self::Hot),
            "noreturn" => Some(Self::NoReturn),
            "nounwind" => Some(Self::NoUnwind),
            "willreturn" => Some(Self::WillReturn),
            "naked" => Some(Self::Naked),
            "noalias" => Some(Self::NoAlias),
            "nocapture" => Some(Self::NoCapture),
            "readonly" => Some(Self::ReadOnly),
            "writeonly" => Some(Self::WriteOnly),
            "readnone" => Some(Self::ReadNone),
            "returned" => Some(Self::Returned),
            "nonnull" => Some(Self::NonNull),
            "noredzone" => Some(Self::NoRedZone),
            "softfloat" => Some(Self::SoftFloat),
            "ssp" => Some(Self::StackProtect),
            "sspreq" => Some(Self::StackProtectReq),
            "sspstrong" => Some(Self::StackProtectStrong),
            "sanitize_address" => Some(Self::SanitizeAddress),
            "sanitize_memory" => Some(Self::SanitizeMemory),
            "sanitize_thread" => Some(Self::SanitizeThread),
            _ => None,
        }
    }

    /// Whether this attribute implies the function never returns.
    pub fn implies_no_return(&self) -> bool {
        matches!(self, Self::NoReturn | Self::Naked)
    }

    /// Whether this attribute disables inlining.
    pub fn blocks_inlining(&self) -> bool {
        matches!(self, Self::NoInline | Self::OptNone | Self::Naked)
    }
}

// ── LLVM Intrinsics ─────────────────────────────────────────────────────────

/// Named intrinsic functions corresponding to LLVM built-in intrinsics.
///
/// ADead-BIB recognises these names and emits optimised x86-64 byte sequences
/// instead of actual function calls.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LlvmIntrinsic {
    // Memory
    MemCpy,  // llvm.memcpy
    MemMove, // llvm.memmove
    MemSet,  // llvm.memset
    MemCmp,  // llvm.memcmp (not standard LLVM but common)

    // Bit manipulation
    Bswap16,      // llvm.bswap.i16
    Bswap32,      // llvm.bswap.i32
    Bswap64,      // llvm.bswap.i64
    Ctlz32,       // llvm.ctlz.i32  (count leading zeros)
    Ctlz64,       // llvm.ctlz.i64
    Cttz32,       // llvm.cttz.i32  (count trailing zeros)
    Cttz64,       // llvm.cttz.i64
    Popcount32,   // llvm.ctpop.i32
    Popcount64,   // llvm.ctpop.i64
    BitReverse32, // llvm.bitreverse.i32
    BitReverse64, // llvm.bitreverse.i64

    // Math
    Sqrt32, // llvm.sqrt.f32
    Sqrt64, // llvm.sqrt.f64
    Fma32,  // llvm.fma.f32
    Fma64,  // llvm.fma.f64
    Abs32,  // llvm.abs.i32
    Abs64,  // llvm.abs.i64
    SMin32,
    SMax32,
    UMin32,
    UMax32,
    SMin64,
    SMax64,
    UMin64,
    UMax64,
    MinNum64,
    MaxNum64, // llvm.minnum / llvm.maxnum (NaN-aware)
    Floor64,
    Ceil64,
    Round64,
    Trunc64,
    Fabs64,

    // Overflow-checked arithmetic
    SAddOverflow,
    UAddOverflow,
    SSubOverflow,
    USubOverflow,
    SMulOverflow,
    UMulOverflow,

    // Prefetch
    Prefetch, // llvm.prefetch

    // Control flow
    Trap,                  // llvm.trap
    Debugtrap,             // llvm.debugtrap
    Unreachable,           // llvm.unreachable (UB-marker)
    Assume,                // llvm.assume
    Expect,                // llvm.expect (branch prediction hint)
    ExpectWithProbability, // llvm.expect.with.probability

    // Stack
    FrameAddress,  // llvm.frameaddress
    ReturnAddress, // llvm.returnaddress
    Stacksave,     // llvm.stacksave
    Stackrestore,  // llvm.stackrestore

    // Atomics
    AtomicFenceSeqCst,
    AtomicFenceAcqRel,

    // SIMD / vector
    VectorReduceAdd,
    VectorReduceAnd,
    VectorReduceOr,
    VectorReduceXor,
    VectorReduceSMin,
    VectorReduceSMax,
}

impl LlvmIntrinsic {
    /// Try to parse an `llvm.*` intrinsic name.
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim_start_matches("llvm.");
        match name {
            "memcpy" | "memcpy.p0.p0.i64" => Some(Self::MemCpy),
            "memmove" | "memmove.p0.p0.i64" => Some(Self::MemMove),
            "memset" | "memset.p0.i64" => Some(Self::MemSet),
            "bswap.i16" => Some(Self::Bswap16),
            "bswap.i32" => Some(Self::Bswap32),
            "bswap.i64" => Some(Self::Bswap64),
            "ctlz.i32" => Some(Self::Ctlz32),
            "ctlz.i64" => Some(Self::Ctlz64),
            "cttz.i32" => Some(Self::Cttz32),
            "cttz.i64" => Some(Self::Cttz64),
            "ctpop.i32" => Some(Self::Popcount32),
            "ctpop.i64" => Some(Self::Popcount64),
            "sqrt.f32" => Some(Self::Sqrt32),
            "sqrt.f64" => Some(Self::Sqrt64),
            "fma.f32" => Some(Self::Fma32),
            "fma.f64" => Some(Self::Fma64),
            "abs.i32" => Some(Self::Abs32),
            "abs.i64" => Some(Self::Abs64),
            "trap" => Some(Self::Trap),
            "debugtrap" => Some(Self::Debugtrap),
            "assume" => Some(Self::Assume),
            "expect" => Some(Self::Expect),
            "expect.with.probability" => Some(Self::ExpectWithProbability),
            "frameaddress" => Some(Self::FrameAddress),
            "returnaddress" => Some(Self::ReturnAddress),
            "stacksave" => Some(Self::Stacksave),
            "stackrestore" => Some(Self::Stackrestore),
            "prefetch" => Some(Self::Prefetch),
            _ => None,
        }
    }

    /// Return the x86-64 REP-MOV encoding size for memory intrinsics, if applicable.
    pub fn is_memory_intrinsic(&self) -> bool {
        matches!(self, Self::MemCpy | Self::MemMove | Self::MemSet)
    }
}

// ── LLVM Calling Conventions ────────────────────────────────────────────────

/// Calling conventions as defined in LLVM IR.
///
/// ADead-BIB maps each to the appropriate x86-64 parameter registers and
/// stack layout in the backend calling-convention tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LlvmCallingConv {
    /// Standard C calling convention (platform default).
    C,
    /// Fast calling convention (intra-module only).
    Fast,
    /// Cold calling convention (rarely-called paths).
    Cold,
    /// Microsoft Win64 ABI (RCX, RDX, R8, R9).
    Win64,
    /// System V AMD64 ABI (RDI, RSI, RDX, RCX, R8, R9).
    X86_64SysV,
    /// x86 __stdcall (right-to-left, callee cleans stack).
    X86StdCall,
    /// x86 __fastcall (ECX/EDX for first two params).
    X86FastCall,
    /// x86 __thiscall (ECX = this).
    X86ThisCall,
    /// Intel vectorcall (XMM0-5 for vectors).
    X86VectorCall,
    /// GHC calling convention.
    GHC,
    /// Preserve most registers (used by GC runtimes).
    PreserveMost,
    /// Preserve all registers.
    PreserveAll,
    /// Swifttail / Swift calling conventions.
    Swift,
    /// ARM AAPCS.
    AAPCS,
    /// ARM AAPCS-VFP.
    AAPCSVfp,
}

impl LlvmCallingConv {
    /// Parse from numeric or symbolic LLVM CC identifier.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "0" | "ccc" | "C" => Some(Self::C),
            "8" | "fastcc" | "Fast" => Some(Self::Fast),
            "9" | "coldcc" | "Cold" => Some(Self::Cold),
            "64" | "win64cc" | "Win64" => Some(Self::Win64),
            "78" | "x86_64_sysvcc" => Some(Self::X86_64SysV),
            "x86_stdcallcc" => Some(Self::X86StdCall),
            "x86_fastcallcc" => Some(Self::X86FastCall),
            "x86_thiscallcc" => Some(Self::X86ThisCall),
            "x86_vectorcallcc" => Some(Self::X86VectorCall),
            _ => None,
        }
    }

    /// Returns true if this CC uses the Win64 parameter-passing rules.
    pub fn is_win64(&self) -> bool {
        matches!(self, Self::Win64)
    }

    /// Returns true if this CC uses the System V AMD64 rules.
    pub fn is_sysv(&self) -> bool {
        matches!(self, Self::C | Self::Fast | Self::Cold | Self::X86_64SysV)
    }
}
