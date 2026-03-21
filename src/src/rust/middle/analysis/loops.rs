// ============================================================
// Loop Analysis
// ============================================================

use crate::middle::ir::basicblock::BasicBlockId;
use std::collections::HashSet;

/// Loop information
#[derive(Debug, Clone)]
pub struct Loop {
    /// Loop header block
    pub header: BasicBlockId,
    /// All blocks in the loop
    pub blocks: HashSet<BasicBlockId>,
    /// Back edges (latch → header)
    pub back_edges: Vec<(BasicBlockId, BasicBlockId)>,
    /// Exit blocks
    pub exits: Vec<BasicBlockId>,
    /// Nested loops
    pub subloops: Vec<Loop>,
}

/// Loop Analysis - Detects and analyzes loops
pub struct LoopAnalysis {
    /// All detected loops
    pub loops: Vec<Loop>,
}

impl LoopAnalysis {
    pub fn new() -> Self {
        LoopAnalysis { loops: Vec::new() }
    }

    /// Get the innermost loop containing a block
    pub fn get_loop_for(&self, block: BasicBlockId) -> Option<&Loop> {
        self.loops.iter().find(|l| l.blocks.contains(&block))
    }

    /// Check if a block is in any loop
    pub fn is_in_loop(&self, block: BasicBlockId) -> bool {
        self.loops.iter().any(|l| l.blocks.contains(&block))
    }
}

impl Default for LoopAnalysis {
    fn default() -> Self {
        Self::new()
    }
}
