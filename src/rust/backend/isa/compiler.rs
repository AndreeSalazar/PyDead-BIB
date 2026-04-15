use super::types::*;
use super::encoder::*;

use super::stubs::*;
use super::instructions::*;
use crate::middle::ir::*;
use crate::backend::reg_alloc::*;
use std::collections::HashMap;

pub fn compile(program: &AllocatedProgram, target: Target) -> CompiledProgram {
    let mut enc = Encoder::new();

    // Add string data + newline
    for (label, content) in &program.string_data {
        enc.add_data_string(label, content);
    }
    // Always add newline string
    enc.add_data_string("__newline", "\r\n");
    // Float constants for ftoa
    enc.add_data_f64("__f64_1e6", 1_000_000.0);
    enc.add_data_f64("__f64_10", 10.0);
    enc.add_data_f64("__f64_0", 0.0);
    // Sign mask for abs(float): clear bit 63
    enc.add_data_f64("__f64_abs_mask", f64::from_bits(0x7FFF_FFFF_FFFF_FFFF));
    // FYL2X constant: 1/log2(e) = log(2)
    enc.add_data_f64("__f64_log2e_inv", std::f64::consts::LN_2);

    // ── Emit runtime stubs first ──────────────────────────
    emit_runtime_stubs(&mut enc, target);

    // ── Compile user functions ────────────────────────────
    let mut compiled_funcs = Vec::new();
    for func in &program.functions {
        let offset = enc.pos();
        compile_function(func, &mut enc, target);
        let size = enc.pos() - offset;
        compiled_funcs.push(CompiledFunction {
            name: func.name.clone(), offset, size,
        });
        enc.stats.functions_compiled += 1;
    }

    // ── Generate _start entry point ───────────────────────
    let entry_offset = enc.pos();
    enc.label("_start");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(40);

    // Call __main__ (top-level code) if exists, else call main
    // __main__ includes any explicit main() calls from the script
    if program.functions.iter().any(|f| f.name == "__main__") {
        enc.call_label("__main__");
    } else if program.functions.iter().any(|f| f.name == "main") {
        enc.call_label("main");
    }

    // Exit
    match target {
        Target::Windows => {
            // Clean up stack and return — Windows kernel32!BaseProcessStart
            // handles process termination when the entry point returns
            enc.add_rsp(40);
            enc.pop(X86Reg::RBX);
            enc.xor_rr(X86Reg::RAX); // exit code 0
            enc.ret();
        }
        Target::Linux => {
            enc.mov_rr(X86Reg::RDI, X86Reg::RAX);
            enc.mov_imm64(X86Reg::RAX, 60);
            enc.emit(&[0x0F, 0x05]); // syscall
        }
        _ => {
            enc.add_rsp(40);
            enc.pop(X86Reg::RBX);
            enc.ret();
        }
    }

    enc.resolve_label_fixups();

    let total = enc.code.len();
    enc.stats.total_bytes = total;

    CompiledProgram {
        text: enc.code,
        data: enc.data,
        data_labels: enc.data_labels,
        functions: compiled_funcs,
        entry_point: entry_offset,
        target,
        iat_fixups: enc.iat_fixups,
        data_fixups: enc.data_fixups,
        stats: enc.stats,
    }
}

// ── Runtime stubs ─────────────────────────────────────────────
pub fn get_used_callee_saved(func: &AllocatedFunction) -> Vec<X86Reg> {
    let mut saved = vec![X86Reg::RBX]; // Always save RBX
    for &reg in CALLEE_SAVED_ORDER {
        if reg == X86Reg::RBX { continue; }
        if func.reg_map.iter().any(|(_, r)| *r == reg) {
            saved.push(reg);
        }
    }
    saved
}

pub fn emit_function_epilogue(saved_regs: &[X86Reg], stack_size: usize, enc: &mut Encoder) {
    if stack_size > 0 && stack_size <= 127 {
        enc.add_rsp(stack_size as u8);
    }
    // Pop in reverse order
    for reg in saved_regs.iter().rev() {
        enc.pop(*reg);
    }
    enc.ret();
}

// ── Compile a user function ───────────────────────────────────
pub fn compile_function(func: &AllocatedFunction, enc: &mut Encoder, _target: Target) {
    enc.label(&func.name);

    let saved_regs = get_used_callee_saved(func);

    // Prologue: push callee-saved registers
    for &reg in &saved_regs {
        enc.push(reg);
    }

    // Compute actual stack size: must be 16-byte aligned including pushes
    // Each push is 8 bytes. With ret addr (8), total = 8 + saved*8 + stack_size
    // Needs to be 16-aligned before any CALL
    let push_bytes = saved_regs.len() * 8 + 8; // +8 for ret addr
    let mut stack = func.stack_size;
    if (push_bytes + stack) % 16 != 0 {
        stack += 8;
    }

    if stack > 0 && stack <= 127 {
        enc.sub_rsp(stack as u8);
    }

    // Move params from ABI registers to callee-saved registers
    for &(src, dst) in &func.param_moves {
        enc.mov_rr(dst, src);
    }

    for instr in &func.body {
        compile_instruction(instr, func, enc, &saved_regs, stack);
    }

    if !func.body.iter().any(|i| matches!(i, IRInstruction::Return | IRInstruction::ReturnVoid)) {
        emit_function_epilogue(&saved_regs, stack, enc);
    }
}

