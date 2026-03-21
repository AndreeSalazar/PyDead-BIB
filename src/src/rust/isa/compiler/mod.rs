// ============================================================
// ADead-BIB ISA Compiler — Modular Structure
// ============================================================
// Pipeline: AST → ADeadIR (Vec<ADeadOp>) → Encoder → bytes
//
// Sin ASM. Sin NASM. Sin LLVM. Solo ISA puro.
// Inspirado en FASM — encoding compacto y eficiente.
//
// Autor: Eddi Andreé Salazar Matos
// ============================================================

mod arrays;
mod compile;
mod control_flow;
mod core;
mod expressions;
mod functions;
mod helpers;
mod statements;

pub use core::*;
