// ============================================================
// ADead-BIB IR Basic Block
// ============================================================
// A basic block is a sequence of instructions with:
// - Single entry point (label at the beginning)
// - Single exit point (terminator at the end)
// - No branches in the middle
// ============================================================

use super::Instruction;
use std::fmt;

/// Basic Block ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BasicBlockId(pub u32);

impl BasicBlockId {
    pub fn new(id: u32) -> Self {
        BasicBlockId(id)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for BasicBlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

/// Basic Block - A sequence of instructions ending in a terminator
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Unique identifier
    pub id: BasicBlockId,

    /// Optional name (for debugging)
    pub name: Option<String>,

    /// Instructions in this block
    pub instructions: Vec<Instruction>,

    /// Predecessor blocks (for CFG)
    pub predecessors: Vec<BasicBlockId>,

    /// Successor blocks (for CFG)
    pub successors: Vec<BasicBlockId>,
}

impl BasicBlock {
    pub fn new(id: u32) -> Self {
        BasicBlock {
            id: BasicBlockId(id),
            name: None,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Add an instruction to the block
    pub fn push(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    /// Insert instruction at position
    pub fn insert(&mut self, index: usize, inst: Instruction) {
        self.instructions.insert(index, inst);
    }

    /// Get the terminator instruction (last instruction)
    pub fn terminator(&self) -> Option<&Instruction> {
        self.instructions.last().filter(|i| i.is_terminator())
    }

    /// Get mutable terminator
    pub fn terminator_mut(&mut self) -> Option<&mut Instruction> {
        self.instructions.last_mut().filter(|i| i.is_terminator())
    }

    /// Check if block has a terminator
    pub fn has_terminator(&self) -> bool {
        self.terminator().is_some()
    }

    /// Check if block is empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Get number of instructions
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Iterate over instructions
    pub fn iter(&self) -> impl Iterator<Item = &Instruction> {
        self.instructions.iter()
    }

    /// Iterate mutably over instructions
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Instruction> {
        self.instructions.iter_mut()
    }

    /// Get instruction at index
    pub fn get(&self, index: usize) -> Option<&Instruction> {
        self.instructions.get(index)
    }

    /// Get mutable instruction at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Instruction> {
        self.instructions.get_mut(index)
    }

    /// Add a predecessor
    pub fn add_predecessor(&mut self, pred: BasicBlockId) {
        if !self.predecessors.contains(&pred) {
            self.predecessors.push(pred);
        }
    }

    /// Add a successor
    pub fn add_successor(&mut self, succ: BasicBlockId) {
        if !self.successors.contains(&succ) {
            self.successors.push(succ);
        }
    }

    /// Remove a predecessor
    pub fn remove_predecessor(&mut self, pred: BasicBlockId) {
        self.predecessors.retain(|p| *p != pred);
    }

    /// Remove a successor
    pub fn remove_successor(&mut self, succ: BasicBlockId) {
        self.successors.retain(|s| *s != succ);
    }

    /// Get the display name
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("bb{}", self.id.0))
    }
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Block label
        writeln!(f, "{}:", self.display_name())?;

        // Instructions
        for inst in &self.instructions {
            writeln!(f, "  {}", inst)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Constant, Type, Value, ValueId};
    use super::*;

    #[test]
    fn test_basic_block_creation() {
        let bb = BasicBlock::new(0).with_name("entry");
        assert_eq!(bb.display_name(), "entry");
        assert!(bb.is_empty());
    }

    #[test]
    fn test_basic_block_instructions() {
        let mut bb = BasicBlock::new(0);

        // Add alloca
        bb.push(Instruction::alloca(Type::I32, ValueId(0)));
        assert_eq!(bb.len(), 1);
        assert!(!bb.has_terminator());

        // Add return
        bb.push(Instruction::ret(Some(Value::Constant(Constant::i32(0)))));
        assert!(bb.has_terminator());
    }

    #[test]
    fn test_predecessors_successors() {
        let mut bb = BasicBlock::new(0);
        bb.add_predecessor(BasicBlockId(1));
        bb.add_successor(BasicBlockId(2));

        assert_eq!(bb.predecessors.len(), 1);
        assert_eq!(bb.successors.len(), 1);

        // No duplicates
        bb.add_predecessor(BasicBlockId(1));
        assert_eq!(bb.predecessors.len(), 1);
    }
}
