// ============================================================
// Dominator Tree Analysis
// ============================================================

use crate::middle::ir::basicblock::BasicBlockId;
use std::collections::HashMap;

/// Dominator Tree - Computes dominance relationships
pub struct DominatorTree {
    /// Immediate dominator for each block
    pub idom: HashMap<BasicBlockId, BasicBlockId>,
    /// Dominance frontier for each block
    pub frontier: HashMap<BasicBlockId, Vec<BasicBlockId>>,
}

impl DominatorTree {
    pub fn new() -> Self {
        DominatorTree {
            idom: HashMap::new(),
            frontier: HashMap::new(),
        }
    }

    /// Check if A dominates B
    pub fn dominates(&self, a: BasicBlockId, b: BasicBlockId) -> bool {
        if a == b {
            return true;
        }

        let mut current = b;
        while let Some(&dom) = self.idom.get(&current) {
            if dom == a {
                return true;
            }
            if dom == current {
                break; // Entry block
            }
            current = dom;
        }

        false
    }
}

impl Default for DominatorTree {
    fn default() -> Self {
        Self::new()
    }
}
