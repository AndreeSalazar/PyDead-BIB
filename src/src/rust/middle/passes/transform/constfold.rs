// ============================================================
// Constant Folding Pass
// ============================================================
// Evaluates constant expressions at compile time
// Inspired by LLVM's ConstantFolding
// ============================================================

use crate::middle::ir::{BinaryOp, Constant, Function, Instruction, Opcode, Type, Value};
use crate::middle::passes::pass_manager::{Pass, PassKind};

/// Constant Folding Pass
pub struct ConstantFoldPass;

impl Pass for ConstantFoldPass {
    fn name(&self) -> &'static str {
        "constfold"
    }

    fn kind(&self) -> PassKind {
        PassKind::Function
    }

    fn run_on_function(&self, func: &mut Function) -> bool {
        let mut changed = false;

        for block in &mut func.blocks {
            for inst in &mut block.instructions {
                if let Some(folded) = try_fold_instruction(inst) {
                    // Replace operands with folded constant
                    inst.operands = vec![Value::Constant(folded)];
                    changed = true;
                }
            }
        }

        changed
    }
}

/// Try to fold an instruction to a constant
fn try_fold_instruction(inst: &Instruction) -> Option<Constant> {
    match &inst.opcode {
        Opcode::Binary(op) => {
            if inst.operands.len() != 2 {
                return None;
            }

            let lhs = inst.operands[0].as_constant()?;
            let rhs = inst.operands[1].as_constant()?;

            fold_binary(*op, lhs, rhs, &inst.ty)
        }
        _ => None,
    }
}

/// Fold a binary operation
fn fold_binary(op: BinaryOp, lhs: &Constant, rhs: &Constant, ty: &Type) -> Option<Constant> {
    // Integer folding
    if let (Constant::Int { value: l, .. }, Constant::Int { value: r, .. }) = (lhs, rhs) {
        let result = match op {
            BinaryOp::Add => l.checked_add(*r)?,
            BinaryOp::Sub => l.checked_sub(*r)?,
            BinaryOp::Mul => l.checked_mul(*r)?,
            BinaryOp::SDiv => {
                if *r == 0 {
                    return None;
                }
                l.checked_div(*r)?
            }
            BinaryOp::UDiv => {
                if *r == 0 {
                    return None;
                }
                ((*l as u64) / (*r as u64)) as i64
            }
            BinaryOp::SRem => {
                if *r == 0 {
                    return None;
                }
                l.checked_rem(*r)?
            }
            BinaryOp::URem => {
                if *r == 0 {
                    return None;
                }
                ((*l as u64) % (*r as u64)) as i64
            }
            BinaryOp::And => *l & *r,
            BinaryOp::Or => *l | *r,
            BinaryOp::Xor => *l ^ *r,
            BinaryOp::Shl => l.checked_shl(*r as u32)?,
            BinaryOp::LShr => ((*l as u64) >> (*r as u32)) as i64,
            BinaryOp::AShr => l.checked_shr(*r as u32)?,
            _ => return None,
        };

        return Some(Constant::Int {
            value: result,
            ty: ty.clone(),
        });
    }

    // Float folding
    if let (Constant::Float { value: l, .. }, Constant::Float { value: r, .. }) = (lhs, rhs) {
        let result = match op {
            BinaryOp::FAdd => *l + *r,
            BinaryOp::FSub => *l - *r,
            BinaryOp::FMul => *l * *r,
            BinaryOp::FDiv => {
                if *r == 0.0 {
                    return None;
                }
                *l / *r
            }
            BinaryOp::FRem => {
                if *r == 0.0 {
                    return None;
                }
                *l % *r
            }
            _ => return None,
        };

        return Some(Constant::Float {
            value: result,
            ty: ty.clone(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::ir::ValueId;

    #[test]
    fn test_fold_add() {
        let mut func = Function::new("test", Type::I32);
        let entry = func.create_block(Some("entry"));

        if let Some(block) = func.get_block_mut(entry) {
            block.push(Instruction::binary(
                BinaryOp::Add,
                Type::I32,
                Value::Constant(Constant::i32(10)),
                Value::Constant(Constant::i32(20)),
                ValueId(0),
            ));
            block.push(Instruction::ret(Some(Value::Instruction(ValueId(0)))));
        }

        let pass = ConstantFoldPass;
        let changed = pass.run_on_function(&mut func);

        assert!(changed);
    }

    #[test]
    fn test_fold_mul() {
        let result = fold_binary(
            BinaryOp::Mul,
            &Constant::i32(6),
            &Constant::i32(7),
            &Type::I32,
        );

        assert!(result.is_some());
        if let Some(Constant::Int { value, .. }) = result {
            assert_eq!(value, 42);
        }
    }

    #[test]
    fn test_fold_div_by_zero() {
        let result = fold_binary(
            BinaryOp::SDiv,
            &Constant::i32(10),
            &Constant::i32(0),
            &Type::I32,
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_fold_float() {
        let result = fold_binary(
            BinaryOp::FAdd,
            &Constant::f64(1.5),
            &Constant::f64(2.5),
            &Type::F64,
        );

        assert!(result.is_some());
        if let Some(Constant::Float { value, .. }) = result {
            assert!((value - 4.0).abs() < 0.001);
        }
    }
}
