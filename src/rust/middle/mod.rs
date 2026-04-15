// ============================================================
// PyDead-BIB Middle-End v1.0
// ============================================================
// Heredado de ADead-BIB v8.0 + extensiones Python
//
// Pipeline: Python AST → IR → UB Detection → Optimization → Backend
// ============================================================

pub mod ir_old;
pub use ir_old as ir;
pub mod ub_detector;
