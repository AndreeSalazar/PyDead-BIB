// ============================================================
// Simplify CFG Pass
// ============================================================
// Simplifies control flow graph
// Inspired by LLVM's SimplifyCFG
// ============================================================

use crate::middle::ir::Function;
use crate::middle::passes::pass_manager::{Pass, PassKind};

/// Simplify CFG Pass
pub struct SimplifyCFGPass;

impl Pass for SimplifyCFGPass {
    fn name(&self) -> &'static str {
        "simplifycfg"
    }
    fn kind(&self) -> PassKind {
        PassKind::Function
    }
    fn run_on_function(&self, _func: &mut Function) -> bool {
        false
    }
}
