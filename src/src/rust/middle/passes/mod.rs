// ============================================================
// ADead-BIB Optimization Passes
// ============================================================
// LLVM-style pass infrastructure
// Passes transform IR to optimize code
// ============================================================

pub mod pass_manager;
pub mod transform;

pub use pass_manager::{OptLevel, Pass, PassKind, PassManager};
pub use transform::*;
