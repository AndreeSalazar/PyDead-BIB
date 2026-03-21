// ============================================================
// Loop Invariant Code Motion Pass
// ============================================================
// Moves loop-invariant code out of loops
// Inspired by LLVM's LICM
// ============================================================

use crate::middle::ir::Function;
use crate::middle::passes::pass_manager::{Pass, PassKind};

/// Loop Invariant Code Motion Pass
pub struct LICMPass;

impl Pass for LICMPass {
    fn name(&self) -> &'static str {
        "licm"
    }
    fn kind(&self) -> PassKind {
        PassKind::Function
    }
    fn run_on_function(&self, _func: &mut Function) -> bool {
        false
    }
}
