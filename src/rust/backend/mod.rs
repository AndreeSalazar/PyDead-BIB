// ============================================================
// PyDead-BIB Backend v1.1 — Heredado de ADead-BIB v8.0
// ============================================================
// Pipeline: IR → Optimizer → RegAlloc → ISA → BG → Output
// Sin linker externo — NUNCA
// Sin GCC — Sin LLVM — Sin Clang
// ============================================================

pub mod optimizer;
pub mod reg_alloc;
pub mod isa;
pub mod bg;
pub mod output;
pub mod jit;
