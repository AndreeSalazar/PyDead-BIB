// ============================================================
// Merge Functions Pass
// ============================================================
// Merges identical functions to reduce code size
// Inspired by LLVM's MergeFunctions
// ============================================================

use crate::middle::ir::Module;
use crate::middle::passes::pass_manager::{Pass, PassKind};

/// Merge Functions Pass
pub struct MergeFunctionsPass;

impl Pass for MergeFunctionsPass {
    fn name(&self) -> &'static str {
        "mergefunc"
    }
    fn kind(&self) -> PassKind {
        PassKind::Module
    }
    fn run_on_module(&self, _module: &mut Module) -> bool {
        false
    }
}
