// ============================================================
// ADead-BIB Middle-End v4.0
// ============================================================
// Inspired by LLVM IR - The heart of the compiler
//
// Pipeline: AST → IR → Optimization Passes → Backend
// ============================================================

pub mod analysis;
pub mod ir;
pub mod lowering;
pub mod passes;
pub mod ub_detector;

pub use ir::{BasicBlock, Function, Instruction, Module, Type as IRType, Value};
pub use lowering::lower_to_ir;
pub use passes::PassManager;
pub use ub_detector::UBDetector;
