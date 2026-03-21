// ============================================================
// Dead Code Elimination Pass
// ============================================================
// Removes instructions whose results are never used
// Inspired by LLVM's DCE pass
// ============================================================

use crate::middle::ir::{Function, Instruction, Opcode};
use crate::middle::passes::pass_manager::{Pass, PassKind};
use std::collections::HashSet;

/// Dead Code Elimination Pass
pub struct DeadCodeElimPass;

impl Pass for DeadCodeElimPass {
    fn name(&self) -> &'static str {
        "dce"
    }

    fn kind(&self) -> PassKind {
        PassKind::Function
    }

    fn run_on_function(&self, func: &mut Function) -> bool {
        let mut changed = false;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10;

        // Iterate until no more changes (fixed point)
        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                break;
            }

            let mut dead_instructions: Vec<(usize, usize)> = Vec::new();

            // Collect all used values
            let used_values = collect_used_values(func);

            // Find dead instructions
            for (block_idx, block) in func.blocks.iter().enumerate() {
                for (inst_idx, inst) in block.instructions.iter().enumerate() {
                    if is_dead_instruction(inst, &used_values) {
                        dead_instructions.push((block_idx, inst_idx));
                    }
                }
            }

            if dead_instructions.is_empty() {
                break;
            }

            // Remove dead instructions (in reverse order to preserve indices)
            for (block_idx, inst_idx) in dead_instructions.into_iter().rev() {
                func.blocks[block_idx].instructions.remove(inst_idx);
                changed = true;
            }
        }

        changed
    }
}

/// Collect all values that are used by any instruction
fn collect_used_values(func: &Function) -> HashSet<u32> {
    let mut used = HashSet::new();

    for block in &func.blocks {
        for inst in &block.instructions {
            // Collect value IDs from operands
            for operand in &inst.operands {
                if let crate::middle::ir::Value::Instruction(id) = operand {
                    used.insert(id.0);
                }
            }

            // Collect from indices (GEP)
            for idx in &inst.indices {
                if let crate::middle::ir::Value::Instruction(id) = idx {
                    used.insert(id.0);
                }
            }
        }
    }

    used
}

/// Check if an instruction is dead (result not used and no side effects)
fn is_dead_instruction(inst: &Instruction, used_values: &HashSet<u32>) -> bool {
    // Instructions with side effects are never dead
    if inst.has_side_effects() {
        return false;
    }

    // Instructions without results are not dead (they're side-effect-only)
    let result_id = match &inst.result {
        Some(id) => id.0,
        None => return false,
    };

    // If result is used, instruction is not dead
    if used_values.contains(&result_id) {
        return false;
    }

    // Alloca is never dead (might be used for address-taken variables)
    if matches!(inst.opcode, Opcode::Alloca) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::ir::{Constant, Type, Value, ValueId};

    #[test]
    fn test_dce_removes_unused() {
        let mut func = Function::new("test", Type::I32);
        let entry = func.create_block(Some("entry"));

        if let Some(block) = func.get_block_mut(entry) {
            // Unused instruction
            block.push(Instruction::binary(
                crate::middle::ir::BinaryOp::Add,
                Type::I32,
                Value::Constant(Constant::i32(1)),
                Value::Constant(Constant::i32(2)),
                ValueId(0),
            ));

            // Return (used)
            block.push(Instruction::ret(Some(Value::Constant(Constant::i32(0)))));
        }

        let pass = DeadCodeElimPass;
        let changed = pass.run_on_function(&mut func);

        assert!(changed);
        assert_eq!(func.blocks[0].instructions.len(), 1); // Only ret remains
    }

    #[test]
    fn test_dce_keeps_used() {
        let mut func = Function::new("test", Type::I32);
        let entry = func.create_block(Some("entry"));

        if let Some(block) = func.get_block_mut(entry) {
            // Used instruction
            block.push(Instruction::binary(
                crate::middle::ir::BinaryOp::Add,
                Type::I32,
                Value::Constant(Constant::i32(1)),
                Value::Constant(Constant::i32(2)),
                ValueId(0),
            ));

            // Return uses the result
            block.push(Instruction::ret(Some(Value::Instruction(ValueId(0)))));
        }

        let pass = DeadCodeElimPass;
        let changed = pass.run_on_function(&mut func);

        assert!(!changed);
        assert_eq!(func.blocks[0].instructions.len(), 2);
    }
}
