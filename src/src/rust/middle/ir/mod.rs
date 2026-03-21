// ============================================================
// ADead-BIB IR (Intermediate Representation)
// ============================================================
// Inspired by LLVM IR - SSA form with typed values
//
// Key concepts:
// - Module: Top-level container (like LLVM Module)
// - Function: Contains basic blocks
// - BasicBlock: Sequence of instructions ending in terminator
// - Instruction: SSA operation producing a Value
// - Value: Typed result of an instruction
// ============================================================

pub mod basicblock;
mod builder;
mod function;
mod instruction;
mod module;
pub mod pdp11_heritage;
mod types;
mod value;

pub use basicblock::BasicBlock;
pub use builder::IRBuilder;
pub use function::Function;
pub use instruction::{BinaryOp, CastOp, CompareOp, Instruction, Opcode};
pub use module::{GlobalVariable, Module};
pub use types::Type;
pub use value::{Constant, Value, ValueId};
