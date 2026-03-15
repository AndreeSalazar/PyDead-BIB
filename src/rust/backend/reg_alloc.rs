// ============================================================
// PyDead-BIB Register Allocator — Heredado de ADead-BIB v8.0
// ============================================================
// Linear Scan Register Allocation for x86-64
// 13 general-purpose registers: RAX-R15 (minus RSP, RBP)
// 16 XMM/YMM registers for float/SIMD
// Sin spill en programas simples — fast path
// ============================================================

use crate::middle::ir::IRType;
use crate::backend::optimizer::OptimizedProgram;

// ── x86-64 Register definitions ──────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum X86Reg {
    RAX, RCX, RDX, RBX,
    RSI, RDI,
    R8, R9, R10, R11, R12, R13, R14, R15,
    // Float/SIMD
    XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7,
    XMM8, XMM9, XMM10, XMM11, XMM12, XMM13, XMM14, XMM15,
    // YMM (256-bit SIMD)
    YMM0, YMM1, YMM2, YMM3, YMM4, YMM5, YMM6, YMM7,
}

impl X86Reg {
    pub fn encoding(&self) -> u8 {
        match self {
            X86Reg::RAX => 0, X86Reg::RCX => 1, X86Reg::RDX => 2, X86Reg::RBX => 3,
            X86Reg::RSI => 6, X86Reg::RDI => 7,
            X86Reg::R8 => 8, X86Reg::R9 => 9, X86Reg::R10 => 10, X86Reg::R11 => 11,
            X86Reg::R12 => 12, X86Reg::R13 => 13, X86Reg::R14 => 14, X86Reg::R15 => 15,
            X86Reg::XMM0 | X86Reg::YMM0 => 0,
            X86Reg::XMM1 | X86Reg::YMM1 => 1,
            X86Reg::XMM2 | X86Reg::YMM2 => 2,
            X86Reg::XMM3 | X86Reg::YMM3 => 3,
            X86Reg::XMM4 | X86Reg::YMM4 => 4,
            X86Reg::XMM5 | X86Reg::YMM5 => 5,
            X86Reg::XMM6 | X86Reg::YMM6 => 6,
            X86Reg::XMM7 | X86Reg::YMM7 => 7,
            X86Reg::XMM8 => 8, X86Reg::XMM9 => 9,
            X86Reg::XMM10 => 10, X86Reg::XMM11 => 11,
            X86Reg::XMM12 => 12, X86Reg::XMM13 => 13,
            X86Reg::XMM14 => 14, X86Reg::XMM15 => 15,
        }
    }

    pub fn needs_rex(&self) -> bool {
        self.encoding() >= 8
    }
}

// ── Allocation result ─────────────────────────────────────────
pub struct AllocatedProgram {
    pub functions: Vec<AllocatedFunction>,
    pub globals: Vec<crate::frontend::python::py_to_ir::IRGlobal>,
    pub string_data: Vec<(String, String)>,
    pub stats: AllocStats,
}

pub struct AllocatedFunction {
    pub name: String,
    pub params: Vec<(String, IRType)>,
    pub return_type: IRType,
    pub body: Vec<crate::middle::ir::IRInstruction>,
    pub reg_map: Vec<(String, X86Reg)>,
    pub param_moves: Vec<(X86Reg, X86Reg)>, // (abi_reg, callee_saved_reg)
    pub stack_size: usize,
    pub spill_count: usize,
}

#[derive(Debug, Default)]
pub struct AllocStats {
    pub total_vars: usize,
    pub registers_used: usize,
    pub spills: usize,
}

// ── Windows x64 ABI: RCX, RDX, R8, R9 for first 4 int args
// ── Linux x64 ABI: RDI, RSI, RDX, RCX, R8, R9
const WIN_INT_ARGS: &[X86Reg] = &[X86Reg::RCX, X86Reg::RDX, X86Reg::R8, X86Reg::R9];
const _LINUX_INT_ARGS: &[X86Reg] = &[X86Reg::RDI, X86Reg::RSI, X86Reg::RDX, X86Reg::RCX, X86Reg::R8, X86Reg::R9];
// Local variables use callee-saved registers so they survive across calls and BinOp/Compare codegen
const SCRATCH_REGS: &[X86Reg] = &[X86Reg::R12, X86Reg::R13, X86Reg::R14, X86Reg::R15, X86Reg::RSI, X86Reg::RDI];
const _CALLEE_SAVED: &[X86Reg] = &[X86Reg::RBX, X86Reg::R12, X86Reg::R13, X86Reg::R14, X86Reg::R15];

// ── Main allocator ────────────────────────────────────────────
pub fn allocate(program: &OptimizedProgram) -> AllocatedProgram {
    let mut stats = AllocStats::default();
    let mut functions = Vec::new();

    for func in &program.functions {
        let alloc_func = allocate_function(func, &mut stats);
        functions.push(alloc_func);
    }

    AllocatedProgram {
        functions,
        globals: program.globals.clone(),
        string_data: program.string_data.clone(),
        stats,
    }
}

fn allocate_function(
    func: &crate::backend::optimizer::OptimizedFunction,
    stats: &mut AllocStats,
) -> AllocatedFunction {
    let mut reg_map: Vec<(String, X86Reg)> = Vec::new();
    let mut next_reg = 0usize;
    let mut spill_count = 0usize;

    // Allocate params to callee-saved scratch registers so they survive calls
    // We'll record the ABI→callee-saved mapping for the prologue
    let mut param_moves: Vec<(X86Reg, X86Reg)> = Vec::new(); // (abi_reg, callee_saved_reg)
    for (i, (name, ir_type)) in func.params.iter().enumerate() {
        if ir_type.is_float() {
            let xmm = match i {
                0 => X86Reg::XMM0,
                1 => X86Reg::XMM1,
                2 => X86Reg::XMM2,
                3 => X86Reg::XMM3,
                _ => { spill_count += 1; X86Reg::XMM0 }
            };
            reg_map.push((name.clone(), xmm));
        } else if next_reg < SCRATCH_REGS.len() && i < WIN_INT_ARGS.len() {
            let dest = SCRATCH_REGS[next_reg];
            param_moves.push((WIN_INT_ARGS[i], dest));
            reg_map.push((name.clone(), dest));
            next_reg += 1;
        } else if i < WIN_INT_ARGS.len() {
            reg_map.push((name.clone(), WIN_INT_ARGS[i]));
        } else {
            spill_count += 1;
            reg_map.push((name.clone(), X86Reg::RAX)); // spilled
        }
        stats.total_vars += 1;
    }

    // Allocate locals from scratch registers
    for instr in &func.body {
        if let crate::middle::ir::IRInstruction::VarDecl { name, ir_type } = instr {
            if !reg_map.iter().any(|(n, _)| n == name) {
                if ir_type.is_float() {
                    let xmm_idx = reg_map.iter().filter(|(_, r)| matches!(r,
                        X86Reg::XMM0 | X86Reg::XMM1 | X86Reg::XMM2 | X86Reg::XMM3 |
                        X86Reg::XMM4 | X86Reg::XMM5 | X86Reg::XMM6 | X86Reg::XMM7
                    )).count();
                    let xmm = match xmm_idx {
                        0 => X86Reg::XMM0, 1 => X86Reg::XMM1, 2 => X86Reg::XMM2,
                        3 => X86Reg::XMM3, 4 => X86Reg::XMM4, 5 => X86Reg::XMM5,
                        6 => X86Reg::XMM6, 7 => X86Reg::XMM7,
                        _ => { spill_count += 1; X86Reg::XMM0 }
                    };
                    reg_map.push((name.clone(), xmm));
                } else if next_reg < SCRATCH_REGS.len() {
                    reg_map.push((name.clone(), SCRATCH_REGS[next_reg]));
                    next_reg += 1;
                } else {
                    spill_count += 1;
                    reg_map.push((name.clone(), X86Reg::RAX));
                }
                stats.total_vars += 1;
            }
        }
    }

    stats.registers_used += reg_map.len();
    stats.spills += spill_count;

    // Stack: 8 bytes per spill + 32 bytes shadow space (Windows ABI)
    let stack_size = 32 + (spill_count * 8);
    // Align to 16 bytes
    let stack_size = (stack_size + 15) & !15;

    AllocatedFunction {
        name: func.name.clone(),
        params: func.params.clone(),
        return_type: func.return_type.clone(),
        body: func.body.clone(),
        reg_map,
        param_moves,
        stack_size,
        spill_count,
    }
}
