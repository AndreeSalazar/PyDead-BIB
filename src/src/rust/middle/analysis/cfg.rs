// ============================================================
// Control Flow Graph Analysis
// ============================================================

use crate::middle::ir::basicblock::BasicBlockId;
use crate::middle::ir::Function;
use std::collections::{HashMap, HashSet};

/// CFG Analysis - Computes control flow graph properties
pub struct CFGAnalysis {
    /// Predecessors for each block
    pub predecessors: HashMap<BasicBlockId, Vec<BasicBlockId>>,
    /// Successors for each block
    pub successors: HashMap<BasicBlockId, Vec<BasicBlockId>>,
    /// Entry block
    pub entry: Option<BasicBlockId>,
    /// Exit blocks (blocks with ret/unreachable)
    pub exits: Vec<BasicBlockId>,
}

impl CFGAnalysis {
    pub fn new() -> Self {
        CFGAnalysis {
            predecessors: HashMap::new(),
            successors: HashMap::new(),
            entry: None,
            exits: Vec::new(),
        }
    }

    /// Analyze a function's CFG
    pub fn analyze(&mut self, func: &Function) {
        self.predecessors.clear();
        self.successors.clear();
        self.exits.clear();

        if func.blocks.is_empty() {
            self.entry = None;
            return;
        }

        self.entry = Some(func.blocks[0].id);

        for block in &func.blocks {
            self.predecessors
                .insert(block.id, block.predecessors.clone());
            self.successors.insert(block.id, block.successors.clone());

            if let Some(term) = block.terminator() {
                if term.is_terminator() && block.successors.is_empty() {
                    self.exits.push(block.id);
                }
            }
        }
    }

    /// Get all reachable blocks from entry
    pub fn reachable_blocks(&self) -> HashSet<BasicBlockId> {
        let mut reachable = HashSet::new();
        let mut worklist = Vec::new();

        if let Some(entry) = self.entry {
            worklist.push(entry);
        }

        while let Some(block) = worklist.pop() {
            if reachable.insert(block) {
                if let Some(succs) = self.successors.get(&block) {
                    worklist.extend(succs.iter().cloned());
                }
            }
        }

        reachable
    }
}

impl Default for CFGAnalysis {
    fn default() -> Self {
        Self::new()
    }
}
