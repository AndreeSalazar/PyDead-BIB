use super::types::*;
use super::encoder::*;
use super::compiler::*;
use super::stubs::*;

use crate::middle::ir::*;
use crate::backend::reg_alloc::*;
use std::collections::HashMap;

pub fn compile_instruction(instr: &IRInstruction, func: &AllocatedFunction, enc: &mut Encoder, saved_regs: &[X86Reg], stack_size: usize) {
    match instr {
        IRInstruction::LoadConst(val) => {
            match val {
                IRConstValue::Int(n) => enc.mov_imm64(X86Reg::RAX, *n),
                IRConstValue::Float(f) => {
                    // Load f64 bits into RAX, then move to XMM0
                    enc.mov_imm64(X86Reg::RAX, f.to_bits() as i64);
                    enc.movq_xmm0_rax();
                }
                IRConstValue::Bool(b) => {
                    if *b { enc.mov_imm64(X86Reg::RAX, 1); }
                    else { enc.xor_rr(X86Reg::RAX); }
                }
                IRConstValue::None => enc.xor_rr(X86Reg::RAX),
            }
        }
        IRInstruction::BinOp { op, left, right } => {
            compile_instruction(left, func, enc, saved_regs, stack_size);
            enc.push(X86Reg::RAX);
            compile_instruction(right, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.pop(X86Reg::RAX);
            match op {
                IROp::Add => enc.add_rr(X86Reg::RAX, X86Reg::RCX),
                IROp::Sub => enc.sub_rr(X86Reg::RAX, X86Reg::RCX),
                IROp::Mul => enc.imul_rr(X86Reg::RAX, X86Reg::RCX),
                IROp::Div | IROp::FloorDiv => enc.idiv_r(X86Reg::RCX),
                IROp::Mod => {
                    enc.idiv_r(X86Reg::RCX);
                    enc.mov_rr(X86Reg::RAX, X86Reg::RDX);
                }
                IROp::Pow => {
                    // RAX=base, RCX=exponent → call __pyb_pow
                    enc.call_label("__pyb_pow");
                }
                IROp::Shl => { enc.rex_w(); enc.emit(&[0xD3, 0xE0]); }
                IROp::Shr => { enc.rex_w(); enc.emit(&[0xD3, 0xF8]); }
                IROp::And => { enc.rex_w(); enc.emit(&[0x21, 0xC8]); }
                IROp::Or  => { enc.rex_w(); enc.emit(&[0x09, 0xC8]); }
                IROp::Xor => { enc.rex_w(); enc.emit(&[0x31, 0xC8]); }
                _ => {}
            }
        }
        IRInstruction::Compare { op, left, right } => {
            compile_instruction(left, func, enc, saved_regs, stack_size);
            enc.push(X86Reg::RAX);
            compile_instruction(right, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.pop(X86Reg::RAX);
            enc.cmp_rr(X86Reg::RAX, X86Reg::RCX);
            let cc = match op {
                IRCmpOp::Eq => 0x94, IRCmpOp::Ne => 0x95,
                IRCmpOp::Lt => 0x9C, IRCmpOp::Le => 0x9E,
                IRCmpOp::Gt => 0x9F, IRCmpOp::Ge => 0x9D,
                _ => 0x94,
            };
            enc.emit(&[0x0F, cc, 0xC0]);
            enc.rex_w(); enc.emit(&[0x0F, 0xB6, 0xC0]);
        }
        IRInstruction::Label(name) => enc.label(name),
        IRInstruction::Jump(lbl) => enc.jmp(lbl),
        IRInstruction::BranchIfFalse(lbl) => {
            enc.rex_w(); enc.emit(&[0x85, 0xC0]); // TEST RAX, RAX
            enc.jcc(0x84, lbl); // JE
        }
        IRInstruction::Return => {
            emit_function_epilogue(saved_regs, stack_size, enc);
        }
        IRInstruction::ReturnVoid => {
            enc.xor_rr(X86Reg::RAX);
            emit_function_epilogue(saved_regs, stack_size, enc);
        }
        IRInstruction::Load(name) => {
            if let Some((_, reg)) = func.reg_map.iter().find(|(n, _)| n == name) {
                enc.mov_rr(X86Reg::RAX, *reg);
            }
        }
        IRInstruction::Store(name) => {
            if let Some((_, reg)) = func.reg_map.iter().find(|(n, _)| n == name) {
                enc.mov_rr(*reg, X86Reg::RAX);
            }
        }
        // v4.0 — Global State Tracker (FASE 1)
        IRInstruction::GlobalLoad(name) => {
            // Load global from .data: LEA RCX, [__global_NAME]; MOV RAX, [RCX]
            let label = format!("__global_{}", name);
            enc.ensure_data_label(&label, 0i64);
            enc.lea_rax_data(&label);
            // MOV RAX, [RAX] — load value from global address
            enc.code.extend_from_slice(&[0x48, 0x8B, 0x00]); // MOV RAX, [RAX]
        }
        IRInstruction::GlobalStore(name) => {
            // Store RAX to global in .data: save RAX, LEA RCX, [__global_NAME]; MOV [RCX], saved
            let label = format!("__global_{}", name);
            enc.ensure_data_label(&label, 0i64);
            // Push RAX (value to store), LEA RAX (addr), MOV RCX=addr, POP RAX (value), MOV [RCX], RAX
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX); // RCX = value
            enc.lea_rax_data(&label);              // RAX = &global
            // MOV [RAX], RCX
            enc.code.extend_from_slice(&[0x48, 0x89, 0x08]); // MOV [RAX], RCX
        }
        IRInstruction::Call { func: callee, args } => {
            // Special: __pyb_obj_new::InitFunc — constructor pattern
            if callee.starts_with("__pyb_obj_new::") {
                let init_func = &callee["__pyb_obj_new::".len()..];
                // args[0] = alloc_size, args[1..] = __init__ params after self
                // 1) Allocate: RCX = size
                if !args.is_empty() {
                    compile_instruction(&args[0], func, enc, saved_regs, stack_size);
                    enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
                }
                enc.sub_rsp(32);
                enc.call_label("__pyb_heap_alloc");
                enc.add_rsp(32);
                // RAX = new obj ptr, save in RBX
                enc.push(X86Reg::RAX); // save obj ptr on stack
                // 2) Call __init__(self=new_ptr, args...)
                // First load remaining args into temps on stack
                for (i, arg) in args[1..].iter().enumerate() {
                    compile_instruction(arg, func, enc, saved_regs, stack_size);
                    enc.push(X86Reg::RAX); // push each arg
                }
                // Now pop args into ABI regs in reverse
                let abi = [X86Reg::RCX, X86Reg::RDX, X86Reg::R8, X86Reg::R9];
                let extra_args = args.len() - 1; // excluding alloc_size
                // Pop extra args that don't fit in ABI regs (leave on stack for callee)
                for i in (0..extra_args).rev() {
                    if i + 1 < abi.len() {
                        enc.pop(abi[i + 1]); // +1 because slot 0 is self
                    } else {
                        // Extra args beyond 4 ABI regs stay on stack
                        enc.pop(X86Reg::RAX); // discard into RAX (simplified)
                    }
                }
                // self = saved obj ptr (on stack top)
                enc.pop(X86Reg::RCX); // self = new obj ptr
                enc.push(X86Reg::RCX); // re-save for return
                enc.sub_rsp(32);
                enc.call_label(init_func);
                enc.add_rsp(32);
                // 3) Return obj ptr
                enc.pop(X86Reg::RAX); // restore obj ptr
            } else if args.is_empty() && callee.starts_with("__pyb_") {
                // No-arg stub calls: RAX already has the value, move to RCX
                enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
                enc.sub_rsp(32);
                enc.call_label(callee);
                enc.add_rsp(32);
            } else {
                // Normal call: push args into Windows ABI regs
                // v4.3 FIX: Compile all args first, push to stack, then pop into ABI regs
                // This prevents clobbering when compiling subsequent args
                let abi = [X86Reg::RCX, X86Reg::RDX, X86Reg::R8, X86Reg::R9];
                let arg_count = args.len().min(4);
                
                // First pass: compile each arg and push result to stack
                for arg in args.iter().take(4) {
                    compile_instruction(arg, func, enc, saved_regs, stack_size);
                    enc.push(X86Reg::RAX);
                }
                
                // Second pass: pop args into ABI regs in reverse order
                for i in (0..arg_count).rev() {
                    enc.pop(abi[i]);
                }
                
                enc.sub_rsp(32);
                enc.call_label(callee);
                enc.add_rsp(32);
            }
        }
        IRInstruction::VarDecl { .. } => {}
        IRInstruction::LoadString(label) => {
            enc.lea_rax_data(label);
        }

        // ── Real print support ────────────────────────────
        // Note: caller is inside a function with push rbx + sub rsp,32
        // So RSP is already 16-byte aligned. We need sub rsp,40 to:
        //   - provide 32 bytes shadow space
        //   - keep alignment (40+8=48 for call → 48%16=0 ✓ ... actually
        //     we are already aligned, so sub 32 + call = 40 → 40%16=8 BAD
        //     We need sub 40 so: 40 + call's 8 = 48 → but wait, it's the
        //     callee's sub that matters. Here we just need shadow space.
        //     Actually: RSP is aligned before we enter this instruction.
        //     sub rsp,32 → still aligned. call pushes 8 → misaligned.
        //     That's NORMAL — callee expects entry with RSP%16==8.)
        // So sub rsp,32 is correct for shadow space before a call.
        IRInstruction::PrintStr(label) => {
            let str_len = enc.data_labels.iter()
                .find(|(n, _)| n == label)
                .map(|(_, off)| {
                    let start = *off as usize;
                    let end = enc.data[start..].iter().position(|&b| b == 0).unwrap_or(0);
                    end as i64
                })
                .unwrap_or(0);

            enc.sub_rsp(32);
            enc.lea_rax_data(label);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.mov_imm64(X86Reg::RDX, str_len);
            enc.call_label("__pyb_print_str");
            enc.add_rsp(32);
        }
        IRInstruction::PrintInt => {
            // RAX already has the integer
            enc.sub_rsp(32);
            enc.call_label("__pyb_itoa");
            enc.add_rsp(32);
        }
        IRInstruction::PrintNewline => {
            enc.sub_rsp(32);
            enc.call_label("__pyb_print_nl");
            enc.add_rsp(32);
        }
        IRInstruction::PrintFloat => {
            // Ensure XMM0 has the float value — RAX may have f64 bits from math calls
            enc.movq_xmm0_rax();
            enc.sub_rsp(32);
            enc.call_label("__pyb_ftoa");
            enc.add_rsp(32);
        }
        IRInstruction::PrintChar => {
            // RAX has the codepoint — write single byte to stack buffer and print
            // MOV [RSP-8], AL (use red zone or allocate)
            enc.sub_rsp(32);
            enc.emit(&[0x88, 0x04, 0x24]); // MOV [RSP], AL
            // LEA RCX, [RSP]
            enc.emit(&[0x48, 0x8D, 0x0C, 0x24]);
            enc.mov_imm64(X86Reg::RDX, 1);
            enc.call_label("__pyb_print_str");
            enc.add_rsp(32);
        }
        IRInstruction::ExitProcess => {
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.call_iat(IAT_EXIT_PROCESS);
        }

        IRInstruction::IterNext { target, end_label } => {
            // For range() loops: RAX has current counter, compare with end
            // The loop variable is the counter — load it, check if done
            // This is called at loop top: if counter >= end, jump to end_label
            // Load loop counter from register
            if let Some((_, reg)) = func.reg_map.iter().find(|(n, _)| n == target) {
                enc.mov_rr(X86Reg::RAX, *reg);
            }
            // RCX should hold the end value (set up by range() init)
            // Compare RAX with RCX (end)
            // For now, we rely on the range setup putting end in a specific register
            // This is handled by the ForRange IR pattern
        }
        IRInstruction::Break | IRInstruction::Continue => {
            // These should have been converted to Jump instructions by py_to_ir
            // If we get here, it's a no-op
        }
        // Exception handling — uses __pyb_error_state global
        IRInstruction::TryBegin(_handler_label) => {
            // Clear error state at try entry
            enc.lea_rax_data("__pyb_error_state");
            enc.emit(&[0x48, 0xC7, 0x00, 0x00, 0x00, 0x00, 0x00]); // MOV QWORD [RAX], 0
        }
        IRInstruction::TryEnd => {
            // Nothing — error state already cleared if no error
        }
        IRInstruction::ClearError => {
            enc.lea_rax_data("__pyb_error_state");
            enc.emit(&[0x48, 0xC7, 0x00, 0x00, 0x00, 0x00, 0x00]); // MOV QWORD [RAX], 0
        }
        IRInstruction::CheckError(label) => {
            // If error_state == 0, jump to label (no error / wrong type)
            enc.lea_rax_data("__pyb_error_state");
            enc.emit(&[0x48, 0x83, 0x38, 0x00]); // CMP QWORD [RAX], 0
            enc.jcc(0x84, label); // JE label (no error → skip handler)
        }
        IRInstruction::Raise { exc_type: _, message } => {
            // Set error state to 1
            enc.lea_rax_data("__pyb_error_state");
            enc.emit(&[0x48, 0xC7, 0x00, 0x01, 0x00, 0x00, 0x00]); // MOV QWORD [RAX], 1
            // If there's a message, evaluate it (for `as e` capture)
            if let Some(msg_instr) = message {
                compile_instruction(msg_instr, func, enc, saved_regs, stack_size);
            }
        }
        IRInstruction::FinallyBegin | IRInstruction::FinallyEnd => {
            // No special codegen — finally is just inline code
        }

        // v3.0 — Coroutine state machine
        IRInstruction::CoroutineCreate { func } => {
            // Allocate coroutine struct on heap: [state:8][result:8] = 16 bytes
            enc.sub_rsp(32);
            enc.mov_imm64(X86Reg::RCX, 16);
            enc.call_label("__pyb_heap_alloc");
            // Initialize state = 0, result = 0
            enc.emit(&[0x48, 0xC7, 0x00, 0x00, 0x00, 0x00, 0x00]); // MOV QWORD [RAX], 0 (state=0)
            enc.emit(&[0x48, 0xC7, 0x40, 0x08, 0x00, 0x00, 0x00, 0x00]); // MOV QWORD [RAX+8], 0 (result=0)
            enc.add_rsp(32);
        }
        IRInstruction::CoroutineResume => {
            // RAX = coroutine ptr, just call the function body (simplified)
            // In a full impl this would switch on state field
        }
        IRInstruction::CoroutineYield => {
            // Store current value into coroutine result field
            // Simplified: just return current RAX
        }

        // v3.0 — Generator protocol
        IRInstruction::GeneratorCreate { func } => {
            // Allocate generator struct: [state:8][current:8][end:8] = 24 bytes
            enc.sub_rsp(32);
            enc.mov_imm64(X86Reg::RCX, 24);
            enc.call_label("__pyb_heap_alloc");
            enc.emit(&[0x48, 0xC7, 0x00, 0x00, 0x00, 0x00, 0x00]); // state=0
            enc.emit(&[0x48, 0xC7, 0x40, 0x08, 0x00, 0x00, 0x00, 0x00]); // current=0
            enc.emit(&[0x48, 0xC7, 0x40, 0x10, 0x00, 0x00, 0x00, 0x00]); // end=0
            enc.add_rsp(32);
        }
        IRInstruction::GeneratorNext => {
            // RAX = generator ptr, load current value, increment state
            // MOV RCX, [RAX+8] (current value)
            enc.emit(&[0x48, 0x8B, 0x48, 0x08]);
            // INC QWORD [RAX+8] (advance current)
            enc.emit(&[0x48, 0xFF, 0x40, 0x08]);
            // MOV RAX, RCX (return current)
            enc.mov_rr(X86Reg::RAX, X86Reg::RCX);
        }
        IRInstruction::GeneratorSend(val) => {
            // Evaluate value, store into generator, then next()
            compile_instruction(val, func, enc, saved_regs, stack_size);
        }

        // v3.0 — Property descriptor
        IRInstruction::PropertyGet { obj, name } => {
            // Calls the getter method: ClassName__name(self)
            if let Some((_, reg)) = func.reg_map.iter().find(|(n, _)| n == obj) {
                enc.mov_rr(X86Reg::RCX, *reg);
            }
        }
        IRInstruction::PropertySet { obj, name } => {
            // Calls the setter method
            if let Some((_, reg)) = func.reg_map.iter().find(|(n, _)| n == obj) {
                enc.mov_rr(X86Reg::RCX, *reg);
            }
        }

        // v3.0 — LRU Cache
        IRInstruction::LruCacheCheck { func: fn_name, key } => {
            // Check hash table for cached result
            compile_instruction(key, func, enc, saved_regs, stack_size);
        }
        IRInstruction::LruCacheStore { func: fn_name, key, value } => {
            // Store result in hash table
            compile_instruction(key, func, enc, saved_regs, stack_size);
            compile_instruction(value, func, enc, saved_regs, stack_size);
        }

        // v3.0 — SIMD AVX2 (YMM 256-bit)
        IRInstruction::SimdLoad { label } => {
            // VMOVAPS YMM0, [RIP+disp32]
            // VEX.256.0F.WIG 28 /r
            enc.emit(&[0xC5, 0xFC, 0x28, 0x05]); // VMOVAPS ymm0, [rip+disp32]
            let fixup_pos = enc.pos();
            enc.emit_u32_le(0);
            enc.data_fixups.push((fixup_pos, label.clone()));
        }
        IRInstruction::SimdOp { op, src } => {
            match op.as_str() {
                "add" => {
                    // VADDPS YMM0, YMM0, YMM1 — C5 FC 58 C1
                    enc.emit(&[0xC5, 0xFC, 0x58, 0xC1]);
                }
                "mul" => {
                    // VMULPS YMM0, YMM0, YMM1 — C5 FC 59 C1
                    enc.emit(&[0xC5, 0xFC, 0x59, 0xC1]);
                }
                "sub" => {
                    // VSUBPS YMM0, YMM0, YMM1 — C5 FC 5C C1
                    enc.emit(&[0xC5, 0xFC, 0x5C, 0xC1]);
                }
                "div" => {
                    // VDIVPS YMM0, YMM0, YMM1 — C5 FC 5E C1
                    enc.emit(&[0xC5, 0xFC, 0x5E, 0xC1]);
                }
                _ => {}
            }
        }
        IRInstruction::SimdStore { label } => {
            // VMOVAPS [RIP+disp32], YMM0
            enc.emit(&[0xC5, 0xFC, 0x29, 0x05]); // VMOVAPS [rip+disp32], ymm0
            let fixup_pos = enc.pos();
            enc.emit_u32_le(0);
            enc.data_fixups.push((fixup_pos, label.clone()));
        }
        IRInstruction::SimdReduce { op } => {
            // Horizontal reduce YMM0 to scalar in XMM0
            // VEXTRACTF128 xmm1, ymm0, 1 — extract high 128
            enc.emit(&[0xC4, 0xE3, 0x7D, 0x19, 0xC1, 0x01]);
            match op.as_str() {
                "sum" => {
                    // VADDPS xmm0, xmm0, xmm1
                    enc.emit(&[0xC5, 0xF8, 0x58, 0xC1]);
                    // VHADDPS xmm0, xmm0, xmm0
                    enc.emit(&[0xC5, 0xFB, 0x7C, 0xC0]);
                    enc.emit(&[0xC5, 0xFB, 0x7C, 0xC0]);
                }
                "max" => {
                    // VMAXPS xmm0, xmm0, xmm1
                    enc.emit(&[0xC5, 0xF8, 0x5F, 0xC1]);
                }
                "min" => {
                    // VMINPS xmm0, xmm0, xmm1
                    enc.emit(&[0xC5, 0xF8, 0x5D, 0xC1]);
                }
                _ => {}
            }
        }
        IRInstruction::SimdSqrt => {
            // VSQRTPS YMM0, YMM0 — C5 FC 51 C0
            enc.emit(&[0xC5, 0xFC, 0x51, 0xC0]);
        }

        // v3.0 — C extension / DLL loading
        IRInstruction::DllLoad { path } => {
            // LoadLibraryA(path) — already in IAT
            // LEA RCX, [path_string]
            enc.lea_rax_data(path);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_iat(IAT_LOAD_LIBRARY);
            enc.add_rsp(32);
        }
        IRInstruction::DllGetProc { module: _, name } => {
            // GetProcAddress(hModule, name) — RAX has module handle from previous DllLoad
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.lea_rax_data(name);
            enc.mov_rr(X86Reg::RDX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_iat(IAT_GET_PROC_ADDRESS);
            enc.add_rsp(32);
        }
        IRInstruction::DllFree { module: _ } => {
            // FreeLibrary(hModule) — RAX has module handle
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_iat(IAT_FREE_LIBRARY);
            enc.add_rsp(32);
        }
        IRInstruction::DllCall { func_ptr: _, args } => {
            // Call function pointer in RAX with args
            for (i, arg) in args.iter().enumerate() {
                compile_instruction(arg, func, enc, saved_regs, stack_size);
                match i {
                    0 => enc.mov_rr(X86Reg::RCX, X86Reg::RAX),
                    1 => enc.mov_rr(X86Reg::RDX, X86Reg::RAX),
                    2 => enc.mov_rr(X86Reg::R8, X86Reg::RAX),
                    3 => enc.mov_rr(X86Reg::R9, X86Reg::RAX),
                    _ => {}
                }
            }
            enc.sub_rsp(32);
            // CALL RAX — FF D0
            enc.emit(&[0xFF, 0xD0]);
            enc.add_rsp(32);
        }

        // v4.1 — ctypes C ABI types
        IRInstruction::CStructAlloc { name: _, size } => {
            // HeapAlloc(size) → RAX = struct ptr
            enc.mov_imm64(X86Reg::RCX, *size as i64);
            enc.call_label("__pyb_heap_alloc");
        }
        IRInstruction::CStructSetField { offset, value } => {
            // Save struct ptr, evaluate value, then MOV [ptr+offset], value
            enc.push(X86Reg::RBX);
            enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // Save struct ptr
            compile_instruction(value, func, enc, saved_regs, stack_size);
            // MOV [RBX+offset], RAX
            if *offset < 128 {
                enc.emit(&[0x48, 0x89, 0x43, *offset as u8]); // MOV [RBX+disp8], RAX
            } else {
                enc.emit(&[0x48, 0x89, 0x83]); // MOV [RBX+disp32], RAX
                enc.emit_u32_le(*offset as u32);
            }
            enc.mov_rr(X86Reg::RAX, X86Reg::RBX); // Restore struct ptr to RAX
            enc.pop(X86Reg::RBX);
        }
        IRInstruction::CStructGetField { offset } => {
            // MOV RAX, [RCX+offset] — RCX = struct ptr
            if *offset < 128 {
                enc.emit(&[0x48, 0x8B, 0x41, *offset as u8]); // MOV RAX, [RCX+disp8]
            } else {
                enc.emit(&[0x48, 0x8B, 0x81]); // MOV RAX, [RCX+disp32]
                enc.emit_u32_le(*offset as u32);
            }
        }
        IRInstruction::CPointerAlloc { inner_size: _ } => {
            // HeapAlloc(8) → RAX = ptr to ptr
            enc.mov_imm64(X86Reg::RCX, 8);
            enc.call_label("__pyb_heap_alloc");
        }
        IRInstruction::CPointerDeref => {
            // MOV RAX, [RAX]
            enc.emit(&[0x48, 0x8B, 0x00]);
        }
        IRInstruction::CPointerSet { value } => {
            // Save ptr, evaluate value, MOV [ptr], value
            enc.push(X86Reg::RBX);
            enc.mov_rr(X86Reg::RBX, X86Reg::RAX);
            compile_instruction(value, func, enc, saved_regs, stack_size);
            enc.emit(&[0x48, 0x89, 0x03]); // MOV [RBX], RAX
            enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
            enc.pop(X86Reg::RBX);
        }
        IRInstruction::CByRef { var } => {
            // LEA RAX, [var] — load address of variable
            enc.lea_rax_data(var);
        }

        // v4.2 — ctypes extended types
        IRInstruction::CCharP { value } => {
            // c_char_p: evaluate string, it's already a pointer to null-terminated data
            compile_instruction(value, func, enc, saved_regs, stack_size);
            // RAX now contains pointer to string data (already null-terminated in .data)
        }
        IRInstruction::CVoidP { value } => {
            // c_void_p: just pass through the pointer value
            compile_instruction(value, func, enc, saved_regs, stack_size);
        }
        IRInstruction::CArrayAlloc { elem_size, count } => {
            // Allocate array: HeapAlloc(elem_size * count)
            let total_size = (*elem_size * *count) as i64;
            enc.mov_imm64(X86Reg::RCX, total_size);
            enc.call_label("__pyb_heap_alloc");
        }
        IRInstruction::CArraySet { elem_size, index, value } => {
            // array[index] = value
            enc.push(X86Reg::RBX);
            enc.push(X86Reg::R12);
            enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // Save array ptr
            compile_instruction(index, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::R12, X86Reg::RAX); // Save index
            compile_instruction(value, func, enc, saved_regs, stack_size);
            // Calculate offset: index * elem_size
            enc.emit(&[0x49, 0x6B, 0xC4, *elem_size as u8]); // IMUL RAX, R12, elem_size
            enc.emit(&[0x48, 0x01, 0xD8]); // ADD RAX, RBX
            enc.emit(&[0x48, 0x89, 0x00]); // MOV [RAX], value (from previous RAX)
            enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
            enc.pop(X86Reg::R12);
            enc.pop(X86Reg::RBX);
        }
        IRInstruction::CArrayGet { elem_size, index } => {
            // value = array[index]
            enc.push(X86Reg::RBX);
            enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // Save array ptr
            compile_instruction(index, func, enc, saved_regs, stack_size);
            // Calculate offset: index * elem_size
            enc.emit(&[0x48, 0x6B, 0xC0, *elem_size as u8]); // IMUL RAX, RAX, elem_size
            enc.emit(&[0x48, 0x01, 0xD8]); // ADD RAX, RBX
            enc.emit(&[0x48, 0x8B, 0x00]); // MOV RAX, [RAX]
            enc.pop(X86Reg::RBX);
        }

        // v4.2 — struct module
        IRInstruction::StructPack { format, values } => {
            // struct.pack: allocate buffer, pack values according to format
            // Calculate total size from format string
            let size = format.chars().filter(|c| *c != '<' && *c != '>' && *c != '=' && *c != '@' && *c != '!').map(|c| match c {
                'b' | 'B' | 'c' | '?' => 1,
                'h' | 'H' => 2,
                'i' | 'I' | 'l' | 'L' | 'f' => 4,
                'q' | 'Q' | 'd' | 'P' => 8,
                _ => 0,
            }).sum::<usize>();
            enc.mov_imm64(X86Reg::RCX, size as i64);
            enc.call_label("__pyb_heap_alloc");
            enc.push(X86Reg::RBX);
            enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // Save buffer ptr
            let mut offset = 0usize;
            for (i, c) in format.chars().filter(|c| *c != '<' && *c != '>' && *c != '=' && *c != '@' && *c != '!').enumerate() {
                if i < values.len() {
                    compile_instruction(&values[i], func, enc, saved_regs, stack_size);
                    // MOV [RBX+offset], RAX (or part of it)
                    if offset < 128 {
                        enc.emit(&[0x48, 0x89, 0x43, offset as u8]);
                    } else {
                        enc.emit(&[0x48, 0x89, 0x83]);
                        enc.emit_u32_le(offset as u32);
                    }
                }
                offset += match c {
                    'b' | 'B' | 'c' | '?' => 1,
                    'h' | 'H' => 2,
                    'i' | 'I' | 'l' | 'L' | 'f' => 4,
                    'q' | 'Q' | 'd' | 'P' => 8,
                    _ => 0,
                };
            }
            enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
            enc.pop(X86Reg::RBX);
        }
        IRInstruction::StructUnpack { format: _, data } => {
            // struct.unpack: return tuple of unpacked values
            // For now, just return the data pointer (simplified)
            compile_instruction(data, func, enc, saved_regs, stack_size);
        }

        // v4.0 — GPU Dispatch (FASE 4)
        // All GPU instructions compile to DLL calls via nvcuda.dll
        // The actual CUDA driver API is called through LoadLibraryA + GetProcAddress
        IRInstruction::GpuInit => {
            // cuInit(0): load nvcuda.dll, get cuInit, call with 0
            let nvcuda_label = "__gpu_nvcuda_path";
            enc.ensure_data_label(nvcuda_label, 0);
            // For now, emit as a call to our GPU runtime stub
            enc.mov_imm64(X86Reg::RCX, 0); // flags = 0
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_init");
            enc.add_rsp(32);
        }
        IRInstruction::GpuDeviceGet => {
            enc.mov_imm64(X86Reg::RCX, 0); // device ordinal = 0
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_device_get");
            enc.add_rsp(32);
        }
        IRInstruction::GpuCtxCreate => {
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_ctx_create");
            enc.add_rsp(32);
        }
        IRInstruction::GpuMalloc { size } => {
            compile_instruction(size, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_malloc");
            enc.add_rsp(32);
        }
        IRInstruction::GpuMemcpyHtoD { dst: _, src: _, size } => {
            compile_instruction(size, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_memcpy_htod");
            enc.add_rsp(32);
        }
        IRInstruction::GpuMemcpyDtoH { dst: _, src: _, size } => {
            compile_instruction(size, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_memcpy_dtoh");
            enc.add_rsp(32);
        }
        IRInstruction::GpuLaunch { kernel, args } => {
            // Load kernel name, then call launch stub
            for (i, arg) in args.iter().enumerate() {
                compile_instruction(arg, func, enc, saved_regs, stack_size);
                let abi = [X86Reg::RCX, X86Reg::RDX, X86Reg::R8, X86Reg::R9];
                if i < abi.len() { enc.mov_rr(abi[i], X86Reg::RAX); }
            }
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_launch");
            enc.add_rsp(32);
        }
        IRInstruction::GpuFree { ptr: _ } => {
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_free");
            enc.add_rsp(32);
        }
        IRInstruction::GpuCtxDestroy => {
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_ctx_destroy");
            enc.add_rsp(32);
        }
        IRInstruction::GpuAvxToCuda { avx_label, gpu_ptr: _, count } => {
            // Load AVX2 data address, then transfer to GPU
            enc.lea_rax_data(avx_label);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            compile_instruction(count, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RDX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_gpu_avx_to_cuda");
            enc.add_rsp(32);
        }

        // v4.0 — Vulkan/SPIR-V Dispatch
        // All Vulkan instructions route through vulkan-1.dll runtime stubs
        IRInstruction::VkInit => {
            enc.mov_imm64(X86Reg::RCX, 0);
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_init");
            enc.add_rsp(32);
        }
        IRInstruction::VkDeviceGet => {
            enc.mov_imm64(X86Reg::RCX, 0);
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_device_get");
            enc.add_rsp(32);
        }
        IRInstruction::VkDeviceCreate => {
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_device_create");
            enc.add_rsp(32);
        }
        IRInstruction::VkBufferCreate { size } => {
            compile_instruction(size, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_buffer_create");
            enc.add_rsp(32);
        }
        IRInstruction::VkBufferWrite { dst: _, src: _, size } => {
            compile_instruction(size, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_buffer_write");
            enc.add_rsp(32);
        }
        IRInstruction::VkBufferRead { dst: _, src: _, size } => {
            compile_instruction(size, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_buffer_read");
            enc.add_rsp(32);
        }
        IRInstruction::VkShaderLoad { spirv_path: _ } => {
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_shader_load");
            enc.add_rsp(32);
        }
        IRInstruction::VkDispatch { shader: _, x, y, z } => {
            compile_instruction(x, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            compile_instruction(y, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::RDX, X86Reg::RAX);
            compile_instruction(z, func, enc, saved_regs, stack_size);
            enc.mov_rr(X86Reg::R8, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_dispatch");
            enc.add_rsp(32);
        }
        IRInstruction::VkBufferFree { ptr: _ } => {
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_buffer_free");
            enc.add_rsp(32);
        }
        IRInstruction::VkDestroy => {
            enc.sub_rsp(32);
            enc.call_label("__pyb_vk_destroy");
            enc.add_rsp(32);
        }

        _ => {}
    }
}
