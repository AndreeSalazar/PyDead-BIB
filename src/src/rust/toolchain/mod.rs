// ============================================================
// ADead-BIB Toolchain Heritage Module v1.0
// ============================================================
// Herencia explícita de los tres grandes toolchains:
//
//   LLVM   → IR attributes, intrinsics, calling conventions
//   GCC    → __builtin_*, __attribute__(()), GNU extensions
//   MSVC   → __declspec(), pragmas, Windows calling conventions
//
// También incluye:
//   calling_conventions — Tabla ABI unificada Win64/SysV
//   cpp_name_mangler    — Itanium (GCC) + MSVC name mangling
//
// Pipeline utilization:
//   Frontend: parsear __attribute__, __declspec, attrs C++17/20
//   Middle:   LlvmAttribute en IR instructions
//   Backend:  CallingConvention → ABI-correct codegen
// ============================================================

pub mod calling_conventions;
pub mod clang_compat;
pub mod cpp_name_mangler;
pub mod gcc_builtins;
pub mod gcc_compat;
pub mod llvm_attrs;
pub mod msvc_compat;

// ── Re-exports ──────────────────────────────────────────────

// LLVM heritage
pub use llvm_attrs::{LlvmAttribute, LlvmCallingConv, LlvmIntrinsic};

// GCC heritage
pub use gcc_builtins::{GccAttribute, GccBuiltin};

// MSVC heritage
pub use msvc_compat::{MsvcCallingConv, MsvcDeclspec, MsvcExtension, MsvcPragma};

// Unified calling conventions
pub use calling_conventions::{detect_convention, shadow_space, CallFrame, CallingConvention};

// C++ name mangler
pub use cpp_name_mangler::{ManglingStyle, NameMangler};
