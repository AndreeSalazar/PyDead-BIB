// ============================================================
// PyDead-BIB v5.0 - Main Library
// ============================================================
// PyDead = Python Dead | BIB = Binary Is Binary
//
// Philosophy:
// - Guido van Rossum: 'readability counts'
// - Dennis Ritchie:   'small is beautiful'
// - Grace Hopper:     'la máquina sirve al humano'
//
// COMPILADOR DUAL: Python + C99 → x86-64 Nativo
// SIN CPYTHON — NUNCA
// SIN GIL — ELIMINADO PARA SIEMPRE
// SIN GCC — SIN LLVM — SIN CLANG
// SIN LINKER EXTERNO — NUNCA
// UB DETECTION ANTES DEL OPTIMIZER
// 256-BIT NATIVO — YMM/AVX2 — SoA NATURAL
//
// Pipeline Python: Source → Preprocessor → ImportResolver → Lexer →
//                  Parser → TypeInferencer → IR (ADeadOp) → UB_Detector →
//                  Optimizer → RegAlloc → BitResolver → ISA → Output
//
// Pipeline C99:    Source → CPreprocessor → CLexer → CParser →
//                  CToIR → Program(IR) → UB_Detector →
//                  Optimizer → RegAlloc → ISA → Output
//
// Targets: windows | linux | fastos64 | fastos128 | fastos256 | all
//
// Hereda ADead-BIB v8.0:
//   IR ADeadOp         → reutilizado 100%
//   ISA Compiler       → reutilizado 100%
//   UB Detector        → extendido Python + C99
//   BG Binary Guardian → reutilizado 100%
//   PE/ELF/Po output   → reutilizado 100%
//   Register Allocator → reutilizado 100%
//   C99 Frontend       → integrado 100% desde ADead-BIB
// ============================================================

// ── Frontend: Python + C99 ──────────────────────────────────
pub mod frontend;

// ── Middle-end: IR + UB (heredado + extendido) ──────────────
pub mod middle;

// ── Backend: Optimizer → RegAlloc → ISA → BG → Output ───────
pub mod backend;
