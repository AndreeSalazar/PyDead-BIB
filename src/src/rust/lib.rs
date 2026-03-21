// ============================================================
// ADead-BIB v8.0 - Main Library
// ============================================================
// ADead = ASM Dead | BIB = Binary Is Binary
//
// Philosophy:
// - Grace Hopper: 'la maquina sirve al humano'
// - Dennis Ritchie: 'small is beautiful'
// - Ken Thompson: 'trust only code you created'
// - Linus Torvalds: 'talk is cheap, show me the code'
//
// SIN LINKER EXTERNO — NUNCA
// UB DETECTION ANTES DEL OPTIMIZER
// 256-BIT NATIVO — YMM/AVX2 — SoA NATURAL
//
// Pipeline: Source → Preprocessor → Parser → IR → UB_Detector →
//           BitResolver → SoA → Optimizer → RegAlloc → ISA → Output
//
// Targets: boot16 | boot32 | fastos64 | fastos128 | fastos256 |
//          windows | linux | all
// ============================================================

// ── Core modules ─────────────────────────────────────────────
pub mod backend;
pub mod bg;
pub mod builder;
pub mod cache;
pub mod cli;
pub mod frontend;
pub mod isa;
pub mod optimizer;
pub mod output;
pub mod preprocessor;
pub mod runtime;
pub mod stdlib;

// Middle-end (LLVM-style IR and passes)
pub mod middle;

// Toolchain compatibility (GCC/LLVM/MSVC flag emulation)
pub mod toolchain;

// ── Backend re-exports ───────────────────────────────────────
pub use backend::cpu::flat_binary::FlatBinaryGenerator;
pub use backend::elf;
pub use backend::pe;

// ── Security module ──────────────────────────────────────────
pub use bg::{BinaryGuardian, SecurityLevel, SecurityPolicy, Verdict};

// ── Frontend re-exports ──────────────────────────────────────
pub use frontend::ast;
pub use frontend::c;
pub use frontend::cpp;
pub use frontend::type_checker;

// ── ISA layer re-exports ─────────────────────────────────────
pub use isa::codegen;
pub use isa::isa_compiler::IsaCompiler;

// ── ISA v8.0: 256-bit pipeline ───────────────────────────────
pub use isa::bit_resolver::{BitResolver, BitTarget};
pub use isa::soa_optimizer::SoaOptimizer;
pub use isa::vex_emitter::VexEmitter;
pub use isa::ymm_allocator::YmmAllocator;

// ── Runtime re-exports ───────────────────────────────────────
pub use runtime::{CPUFeatures, ComputeBackend};

// ── Middle-end re-exports ────────────────────────────────────
pub use middle::ir::{Function as IRFunction, Module as IRModule, Type as IRType};
pub use middle::lowering::lower_to_ir;
pub use middle::passes::{OptLevel, PassManager};
pub use middle::ub_detector::{UBDetector, UBKind, UBReport};

// ── Preprocessor re-exports (Sin CMake, Sin Linker — NUNCA) ─
pub use preprocessor::{HeaderResolver, MacroExpander, SymbolDedup};

// ── Standard Library re-exports (Sin libc externa) ──────────
pub use stdlib::HeaderMain;

// ── Cache re-exports (fastos.bib system v2) ─────────────────
pub use cache::ADeadCache;

// ── Output re-exports (Sin linker externo) ──────────────────
pub use output::OutputFormat;

// ── Toolchain Heritage re-exports ───────────────────────────
// LLVM: attributes, intrinsics, calling conventions
pub use toolchain::llvm_attrs::{LlvmAttribute, LlvmCallingConv, LlvmIntrinsic};
// GCC: __attribute__(()), __builtin_*
pub use toolchain::gcc_builtins::{GccAttribute, GccBuiltin};
// GCC: flag compatibility (-O2, -Wall, -std=c99, etc.)
pub use toolchain::gcc_compat::{parse_gcc_flag, GccFlagResult, GccOptLevel};
// Clang: flag compatibility (-fsanitize, --target, etc.)
pub use toolchain::clang_compat::{parse_clang_flag, ClangFlagResult};
// MSVC: __declspec(), calling conventions, extensions
pub use toolchain::msvc_compat::{MsvcCallingConv, MsvcDeclspec, MsvcExtension, MsvcPragma};
// Unified calling convention table
pub use toolchain::calling_conventions::{
    detect_convention, shadow_space, CallFrame, CallingConvention,
};
// C++ name mangling
pub use toolchain::cpp_name_mangler::{ManglerContext, ManglingStyle, NameMangler};
