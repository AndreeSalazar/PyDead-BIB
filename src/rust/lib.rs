// ============================================================
// PyDead-BIB v1.0 - Main Library
// ============================================================
// PyDead = Python Dead | BIB = Binary Is Binary
//
// Philosophy:
// - Guido van Rossum: 'readability counts'
// - Dennis Ritchie: 'small is beautiful'
// - Grace Hopper: 'la maquina sirve al humano'
//
// SIN CPYTHON — NUNCA
// SIN GIL — ELIMINADO PARA SIEMPRE
// SIN LINKER EXTERNO — NUNCA
// UB DETECTION ANTES DEL OPTIMIZER
// 256-BIT NATIVO — YMM/AVX2 — SoA NATURAL
//
// Pipeline: Python Source → Preprocessor → ImportResolver → Lexer →
//           Parser → TypeInferencer → IR (ADeadOp) → UB_Detector →
//           Optimizer → RegAlloc → BitResolver → ISA → Output
//
// Targets: windows | linux | fastos64 | fastos128 | fastos256 | all
//
// Hereda ADead-BIB v8.0:
//   IR ADeadOp        → reutilizado 100%
//   ISA Compiler      → reutilizado 100%
//   UB Detector       → extendido Python
//   BG Binary Guardian→ reutilizado 100%
//   PE/ELF/Po output  → reutilizado 100%
//   Register Allocator→ reutilizado 100%
// ============================================================

// ── Python Frontend (★ NUEVO — 15% del código) ──────────────
pub mod frontend;

// ── Middle-end: IR + UB (heredado + extendido) ──────────────
pub mod middle;
