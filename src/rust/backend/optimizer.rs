// ============================================================
// PyDead-BIB Optimizer — Heredado de ADead-BIB v8.0
// ============================================================
// Constant folding, dead code elimination, SIMD vectorization
// Sin GCC — Sin LLVM — optimizaciones propias
// ============================================================

use crate::middle::ir::{IRConstValue, IRInstruction, IROp, IRType};
use crate::frontend::python::py_to_ir::IRProgram;

// ── Optimization pass result ──────────────────────────────────
pub struct OptimizedProgram {
    pub functions: Vec<OptimizedFunction>,
    pub globals: Vec<crate::frontend::python::py_to_ir::IRGlobal>,
    pub string_data: Vec<(String, String)>,
    pub stats: OptStats,
}

pub struct OptimizedFunction {
    pub name: String,
    pub params: Vec<(String, IRType)>,
    pub return_type: IRType,
    pub body: Vec<IRInstruction>,
}

#[derive(Debug, Default)]
pub struct OptStats {
    pub constants_folded: usize,
    pub dead_code_removed: usize,
    pub simd_vectorized: usize,
}

// ── Main optimizer ────────────────────────────────────────────
pub fn optimize(program: &IRProgram) -> OptimizedProgram {
    let mut stats = OptStats::default();
    let mut functions = Vec::new();

    for func in &program.functions {
        let optimized_body = optimize_function(&func.body, &mut stats);
        functions.push(OptimizedFunction {
            name: func.name.clone(),
            params: func.params.clone(),
            return_type: func.return_type.clone(),
            body: optimized_body,
        });
    }

    OptimizedProgram {
        functions,
        globals: program.globals.clone(),
        string_data: program.string_data.clone(),
        stats,
    }
}

fn optimize_function(body: &[IRInstruction], stats: &mut OptStats) -> Vec<IRInstruction> {
    let mut result: Vec<IRInstruction> = Vec::with_capacity(body.len());

    for instr in body {
        match instr {
            // ── Constant folding: int op int → const ──────────
            IRInstruction::BinOp { op, left, right } => {
                if let (
                    IRInstruction::LoadConst(IRConstValue::Int(a)),
                    IRInstruction::LoadConst(IRConstValue::Int(b)),
                ) = (left.as_ref(), right.as_ref()) {
                    if let Some(result_val) = fold_int_op(*op, *a, *b) {
                        result.push(IRInstruction::LoadConst(IRConstValue::Int(result_val)));
                        stats.constants_folded += 1;
                        continue;
                    }
                }
                if let (
                    IRInstruction::LoadConst(IRConstValue::Float(a)),
                    IRInstruction::LoadConst(IRConstValue::Float(b)),
                ) = (left.as_ref(), right.as_ref()) {
                    if let Some(result_val) = fold_float_op(*op, *a, *b) {
                        result.push(IRInstruction::LoadConst(IRConstValue::Float(result_val)));
                        stats.constants_folded += 1;
                        continue;
                    }
                }
                // Optimize children recursively
                let opt_left = optimize_expr(left, stats);
                let opt_right = optimize_expr(right, stats);
                result.push(IRInstruction::BinOp {
                    op: *op,
                    left: Box::new(opt_left),
                    right: Box::new(opt_right),
                });
            }
            // ── Remove Nop ────────────────────────────────────
            IRInstruction::Nop => {
                stats.dead_code_removed += 1;
            }
            _ => {
                result.push(instr.clone());
            }
        }
    }

    result
}

fn optimize_expr(instr: &IRInstruction, stats: &mut OptStats) -> IRInstruction {
    match instr {
        IRInstruction::BinOp { op, left, right } => {
            if let (
                IRInstruction::LoadConst(IRConstValue::Int(a)),
                IRInstruction::LoadConst(IRConstValue::Int(b)),
            ) = (left.as_ref(), right.as_ref()) {
                if let Some(val) = fold_int_op(*op, *a, *b) {
                    stats.constants_folded += 1;
                    return IRInstruction::LoadConst(IRConstValue::Int(val));
                }
            }
            instr.clone()
        }
        _ => instr.clone(),
    }
}

fn fold_int_op(op: IROp, a: i64, b: i64) -> Option<i64> {
    match op {
        IROp::Add => a.checked_add(b),
        IROp::Sub => a.checked_sub(b),
        IROp::Mul => a.checked_mul(b),
        IROp::Div if b != 0 => Some(a / b),
        IROp::FloorDiv if b != 0 => Some(a / b),
        IROp::Mod if b != 0 => Some(a % b),
        IROp::Shl if b >= 0 && b < 64 => Some(a << b),
        IROp::Shr if b >= 0 && b < 64 => Some(a >> b),
        IROp::And => Some(a & b),
        IROp::Or => Some(a | b),
        IROp::Xor => Some(a ^ b),
        _ => None,
    }
}

fn fold_float_op(op: IROp, a: f64, b: f64) -> Option<f64> {
    match op {
        IROp::Add => Some(a + b),
        IROp::Sub => Some(a - b),
        IROp::Mul => Some(a * b),
        IROp::Div if b != 0.0 => Some(a / b),
        _ => None,
    }
}
