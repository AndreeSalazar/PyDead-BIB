// ============================================================
// PyDead-BIB ISA Compiler v1.3 — Full Runtime
// ============================================================
// IR → x86-64 machine code bytes
// Direct encoding — sin assembler externo — sin NASM
// Windows: GetStdHandle + WriteFile via IAT
// Linux: write syscall direct
// Runtime stubs: print_str, itoa, ftoa, print_nl
// Supports: int, float, str, bool, if/else, for, while, funcs
// ============================================================

use crate::middle::ir::{IRConstValue, IRCmpOp, IRInstruction, IROp};
use crate::backend::reg_alloc::{AllocatedFunction, AllocatedProgram, X86Reg};

// ── Compilation target ────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Windows,
    Linux,
    FastOS64,
    FastOS128,
    FastOS256,
}

impl Target {
    pub fn from_str(s: &str) -> Self {
        match s {
            "windows" | "win64" | "pe" => Target::Windows,
            "linux" | "elf" => Target::Linux,
            "fastos64" => Target::FastOS64,
            "fastos128" => Target::FastOS128,
            "fastos256" => Target::FastOS256,
            _ => Target::Windows,
        }
    }
}

// ── IAT slot indices (order must match output.rs import table) ──
pub const IAT_GET_STD_HANDLE: usize = 0;
pub const IAT_WRITE_FILE: usize = 1;
pub const IAT_EXIT_PROCESS: usize = 2;
pub const IAT_SLOT_COUNT: usize = 3;

// ── Compiled code section ─────────────────────────────────────
pub struct CompiledProgram {
    pub text: Vec<u8>,
    pub data: Vec<u8>,
    pub data_labels: Vec<(String, u32)>,
    pub functions: Vec<CompiledFunction>,
    pub entry_point: u32,
    pub target: Target,
    pub iat_fixups: Vec<(u32, usize)>,  // (offset_in_text, iat_slot_index)
    pub data_fixups: Vec<(u32, String)>, // (offset_in_text, data_label) for LEA
    pub stats: ISAStats,
}

pub struct CompiledFunction {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Default)]
pub struct ISAStats {
    pub total_bytes: usize,
    pub functions_compiled: usize,
    pub instructions_emitted: usize,
}

// ── x86-64 Encoder ────────────────────────────────────────────
struct Encoder {
    code: Vec<u8>,
    data: Vec<u8>,
    data_labels: Vec<(String, u32)>,
    label_offsets: Vec<(String, u32)>,
    fixups: Vec<(usize, String)>,
    iat_fixups: Vec<(u32, usize)>,
    data_fixups: Vec<(u32, String)>,
    stats: ISAStats,
}

impl Encoder {
    fn new() -> Self {
        Self {
            code: Vec::new(),
            data: Vec::new(),
            data_labels: Vec::new(),
            label_offsets: Vec::new(),
            fixups: Vec::new(),
            iat_fixups: Vec::new(),
            data_fixups: Vec::new(),
            stats: ISAStats::default(),
        }
    }

    fn pos(&self) -> u32 { self.code.len() as u32 }

    fn emit(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
        self.stats.instructions_emitted += 1;
    }

    fn emit_u8(&mut self, b: u8) { self.code.push(b); }

    fn emit_u32_le(&mut self, v: u32) { self.code.extend_from_slice(&v.to_le_bytes()); }

    fn emit_i32_le(&mut self, v: i32) { self.code.extend_from_slice(&v.to_le_bytes()); }

    fn emit_u64_le(&mut self, v: u64) { self.code.extend_from_slice(&v.to_le_bytes()); }

    // REX.W prefix (no extended regs)
    fn rex_w(&mut self) { self.emit_u8(0x48); }

    // REX.W with R and B bits for extended registers
    // For instructions like MOV r/m64, r64:  REX.R = src>=8, REX.B = dst>=8
    // For instructions like MOV r64, r/m64:  REX.R = dst>=8, REX.B = src>=8
    fn rex_wrb(&mut self, reg: X86Reg, rm: X86Reg) {
        let mut rex: u8 = 0x48; // REX.W
        if reg.encoding() >= 8 { rex |= 0x04; } // REX.R
        if rm.encoding() >= 8 { rex |= 0x01; }  // REX.B
        self.emit_u8(rex);
    }

    fn rex_wb(&mut self, rm: X86Reg) {
        let mut rex: u8 = 0x48;
        if rm.encoding() >= 8 { rex |= 0x01; }
        self.emit_u8(rex);
    }

    // MOV reg, imm64
    fn mov_imm64(&mut self, reg: X86Reg, val: i64) {
        let r = reg.encoding();
        if r >= 8 { self.emit_u8(0x49); } else { self.emit_u8(0x48); }
        self.emit_u8(0xB8 + (r & 7));
        self.emit_u64_le(val as u64);
        self.stats.instructions_emitted += 1;
    }

    // MOV r/m64, r64  (opcode 0x89: src=reg field, dst=r/m field)
    fn mov_rr(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_wrb(src, dst);
        self.emit(&[0x89, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    fn add_rr(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_wrb(src, dst);
        self.emit(&[0x01, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    fn sub_rr(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_wrb(src, dst);
        self.emit(&[0x29, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    fn imul_rr(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_wrb(dst, src); // 0F AF: reg=dst, r/m=src
        self.emit(&[0x0F, 0xAF, 0xC0 | ((dst.encoding() & 7) << 3) | (src.encoding() & 7)]);
    }

    fn idiv_r(&mut self, src: X86Reg) {
        self.rex_w(); self.emit_u8(0x99); // CQO
        self.rex_wb(src); self.emit(&[0xF7, 0xF8 | (src.encoding() & 7)]);
    }

    fn cmp_rr(&mut self, a: X86Reg, b: X86Reg) {
        self.rex_wrb(b, a); // CMP r/m64, r64: reg=b, r/m=a
        self.emit(&[0x39, 0xC0 | ((b.encoding() & 7) << 3) | (a.encoding() & 7)]);
    }

    fn xor_rr(&mut self, reg: X86Reg) {
        self.rex_wrb(reg, reg);
        let r = reg.encoding() & 7;
        self.emit(&[0x31, 0xC0 | (r << 3) | r]);
    }

    fn push(&mut self, reg: X86Reg) {
        if reg.needs_rex() { self.emit_u8(0x41); }
        self.emit_u8(0x50 + (reg.encoding() & 7));
    }

    fn pop(&mut self, reg: X86Reg) {
        if reg.needs_rex() { self.emit_u8(0x41); }
        self.emit_u8(0x58 + (reg.encoding() & 7));
    }

    fn sub_rsp(&mut self, val: u8) { self.rex_w(); self.emit(&[0x83, 0xEC, val]); }
    fn add_rsp(&mut self, val: u8) { self.rex_w(); self.emit(&[0x83, 0xC4, val]); }
    fn ret(&mut self) { self.emit_u8(0xC3); }

    fn label(&mut self, name: &str) {
        self.label_offsets.push((name.to_string(), self.pos()));
    }

    fn jmp(&mut self, lbl: &str) {
        self.emit_u8(0xE9);
        self.fixups.push((self.code.len(), lbl.to_string()));
        self.emit_u32_le(0);
    }

    fn jcc(&mut self, cc: u8, lbl: &str) {
        self.emit(&[0x0F, cc]);
        self.fixups.push((self.code.len(), lbl.to_string()));
        self.emit_u32_le(0);
    }

    fn call_label(&mut self, lbl: &str) {
        self.emit_u8(0xE8);
        self.fixups.push((self.code.len(), lbl.to_string()));
        self.emit_u32_le(0);
    }

    // CALL [RIP+disp32] — indirect call through IAT
    fn call_iat(&mut self, slot: usize) {
        // FF 15 xx xx xx xx = CALL [RIP+disp32]
        self.emit(&[0xFF, 0x15]);
        let fixup_pos = self.pos();
        self.emit_u32_le(0); // placeholder — output.rs patches this
        self.iat_fixups.push((fixup_pos, slot));
    }

    // LEA RAX, [RIP+disp32] — load data address
    fn lea_rax_data(&mut self, data_label: &str) {
        // 48 8D 05 xx xx xx xx = LEA RAX, [RIP+disp32]
        self.emit(&[0x48, 0x8D, 0x05]);
        let fixup_pos = self.pos();
        self.emit_u32_le(0);
        self.data_fixups.push((fixup_pos, data_label.to_string()));
    }

    fn add_data_string(&mut self, label: &str, s: &str) {
        let offset = self.data.len() as u32;
        self.data_labels.push((label.to_string(), offset));
        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0);
    }

    fn add_data_f64(&mut self, label: &str, val: f64) {
        // Align to 8 bytes
        while self.data.len() % 8 != 0 { self.data.push(0); }
        let offset = self.data.len() as u32;
        self.data_labels.push((label.to_string(), offset));
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    // ── SSE2 float instructions ─────────────────────────────
    // MOVSD XMM, [RIP+disp32]  — load f64 from data
    fn movsd_xmm_data(&mut self, xmm: u8, data_label: &str) {
        // F2 0F 10 /r = MOVSD xmm1, xmm2/m64
        // ModRM: 00 reg 101 = RIP+disp32
        self.emit(&[0xF2, 0x0F, 0x10, 0x05 | (xmm << 3)]);
        let fixup_pos = self.pos();
        self.emit_u32_le(0);
        self.data_fixups.push((fixup_pos, data_label.to_string()));
    }

    // MOVSD xmm1, xmm2
    fn movsd_xmm_xmm(&mut self, dst: u8, src: u8) {
        // F2 0F 10 /r (reg-reg: mod=11)
        self.emit(&[0xF2, 0x0F, 0x10, 0xC0 | (dst << 3) | src]);
    }

    // ADDSD xmm1, xmm2
    fn addsd(&mut self, dst: u8, src: u8) {
        self.emit(&[0xF2, 0x0F, 0x58, 0xC0 | (dst << 3) | src]);
    }
    // SUBSD xmm1, xmm2
    fn subsd(&mut self, dst: u8, src: u8) {
        self.emit(&[0xF2, 0x0F, 0x5C, 0xC0 | (dst << 3) | src]);
    }
    // MULSD xmm1, xmm2
    fn mulsd(&mut self, dst: u8, src: u8) {
        self.emit(&[0xF2, 0x0F, 0x59, 0xC0 | (dst << 3) | src]);
    }
    // DIVSD xmm1, xmm2
    fn divsd(&mut self, dst: u8, src: u8) {
        self.emit(&[0xF2, 0x0F, 0x5E, 0xC0 | (dst << 3) | src]);
    }
    // CVTTSD2SI reg64, xmm  (truncate f64 → i64)
    fn cvttsd2si(&mut self, dst: X86Reg, xmm: u8) {
        // F2 REX.W 0F 2C /r
        let mut rex: u8 = 0x48;
        if dst.encoding() >= 8 { rex |= 0x04; }
        self.emit(&[0xF2, rex, 0x0F, 0x2C, 0xC0 | ((dst.encoding() & 7) << 3) | xmm]);
    }
    // CVTSI2SD xmm, reg64  (i64 → f64)
    fn cvtsi2sd(&mut self, xmm: u8, src: X86Reg) {
        // F2 REX.W 0F 2A /r
        let mut rex: u8 = 0x48;
        if src.encoding() >= 8 { rex |= 0x01; }
        self.emit(&[0xF2, rex, 0x0F, 0x2A, 0xC0 | (xmm << 3) | (src.encoding() & 7)]);
    }
    // MOVQ RAX, XMM0 (move 64 bits from xmm to gpr)
    fn movq_rax_xmm0(&mut self) {
        // 66 48 0F 7E C0 = MOVQ RAX, XMM0
        self.emit(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]);
    }
    // MOVQ XMM0, RAX
    fn movq_xmm0_rax(&mut self) {
        // 66 48 0F 6E C0 = MOVQ XMM0, RAX
        self.emit(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]);
    }
    // XORPD xmm, xmm (zero a float register)
    fn xorpd(&mut self, xmm: u8) {
        self.emit(&[0x66, 0x0F, 0x57, 0xC0 | (xmm << 3) | xmm]);
    }
    // UCOMISD xmm1, xmm2
    fn ucomisd(&mut self, a: u8, b: u8) {
        self.emit(&[0x66, 0x0F, 0x2E, 0xC0 | (a << 3) | b]);
    }
    // MULSD xmm, [RIP+disp32]
    fn mulsd_data(&mut self, xmm: u8, label: &str) {
        self.emit(&[0xF2, 0x0F, 0x59, 0x05 | (xmm << 3)]);
        let fixup_pos = self.pos();
        self.emit_u32_le(0);
        self.data_fixups.push((fixup_pos, label.to_string()));
    }

    fn resolve_label_fixups(&mut self) {
        for (fixup_off, target_lbl) in &self.fixups {
            if let Some((_, target_off)) = self.label_offsets.iter().find(|(n, _)| n == target_lbl) {
                let rel32 = (*target_off as i32) - (*fixup_off as i32 + 4);
                let bytes = rel32.to_le_bytes();
                self.code[*fixup_off] = bytes[0];
                self.code[*fixup_off + 1] = bytes[1];
                self.code[*fixup_off + 2] = bytes[2];
                self.code[*fixup_off + 3] = bytes[3];
            }
        }
    }
}

// ── Main ISA compiler ─────────────────────────────────────────
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
fn emit_runtime_stubs(enc: &mut Encoder, target: Target) {
    // __pyb_print_str: RCX=ptr, RDX=len → WriteFile(stdout, ptr, len)
    enc.label("__pyb_print_str");
    match target {
        Target::Windows => {
            // Stack layout after entry:
            //   [ret addr] ← RSP on entry (misaligned by 8)
            // We need: 3 saved regs (24) + local space
            // 24 + 8(ret) = 32 → need sub 48 → total 80 → 80%16=0 ✓
            // Local layout at RSP after sub:
            //   [rsp+0x00..0x1F] = shadow space for subcalls (32 bytes)
            //   [rsp+0x20..0x27] = 5th param slot / written var (8 bytes)
            //   [rsp+0x28..0x2F] = padding (8 bytes)
            enc.push(X86Reg::RBX);
            enc.push(X86Reg::RSI);
            enc.push(X86Reg::RDI);
            enc.sub_rsp(48);

            // Save ptr and len in non-volatile regs
            enc.mov_rr(X86Reg::RSI, X86Reg::RCX); // ptr
            enc.mov_rr(X86Reg::RDI, X86Reg::RDX); // len

            // GetStdHandle(STD_OUTPUT_HANDLE = -11)
            enc.mov_imm64(X86Reg::RCX, -11i64);
            enc.call_iat(IAT_GET_STD_HANDLE);
            enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // save handle

            // WriteFile(handle, buf, len, &written, NULL)
            // RCX = handle
            enc.mov_rr(X86Reg::RCX, X86Reg::RBX);
            // RDX = buffer pointer
            enc.mov_rr(X86Reg::RDX, X86Reg::RSI);
            // R8 = number of bytes to write
            enc.mov_rr(X86Reg::R8, X86Reg::RDI);
            // R9 = &written → lea r9, [rsp+0x28]
            enc.emit(&[0x4C, 0x8D, 0x4C, 0x24, 0x28]);
            // 5th param: lpOverlapped = NULL → mov qword [rsp+0x20], 0
            enc.emit(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00]);
            enc.call_iat(IAT_WRITE_FILE);

            enc.add_rsp(48);
            enc.pop(X86Reg::RDI);
            enc.pop(X86Reg::RSI);
            enc.pop(X86Reg::RBX);
            enc.ret();
        }
        Target::Linux => {
            // write(1, ptr, len)
            // On entry: RCX=ptr, RDX=len
            // Linux syscall ABI: RAX=syscall#, RDI=fd, RSI=buf, RDX=count
            enc.mov_rr(X86Reg::RSI, X86Reg::RCX); // buf
            // RDX already has len
            enc.mov_imm64(X86Reg::RAX, 1); // SYS_write
            enc.mov_imm64(X86Reg::RDI, 1); // stdout fd
            enc.emit(&[0x0F, 0x05]); // syscall
            enc.ret();
        }
        _ => { enc.ret(); }
    }

    // __pyb_print_nl: print "\r\n"
    enc.label("__pyb_print_nl");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(32);
    enc.lea_rax_data("__newline");
    enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
    enc.mov_imm64(X86Reg::RDX, 2); // "\r\n" = 2 bytes
    enc.call_label("__pyb_print_str");
    enc.add_rsp(32);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_itoa: RAX=int64 → prints decimal to stdout
    // Uses stack buffer, divides by 10 repeatedly
    enc.label("__pyb_itoa");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(80); // 32 shadow + 32 buf + 16 align

    // Save number
    enc.mov_rr(X86Reg::RSI, X86Reg::RAX);

    // Handle negative: if RAX < 0, print '-' and negate
    // TEST RSI, RSI
    enc.rex_w(); enc.emit(&[0x85, 0xF6]);
    // JNS skip_neg
    let skip_neg = format!("__itoa_pos_{}", enc.pos());
    enc.jcc(0x89, &skip_neg); // JNS
    // Print '-'
    enc.mov_imm64(X86Reg::RAX, 0x2D); // '-'
    // mov [rsp+32], al
    enc.emit(&[0x88, 0x44, 0x24, 0x20]);
    // lea rcx, [rsp+32]
    enc.emit(&[0x48, 0x8D, 0x4C, 0x24, 0x20]);
    enc.mov_imm64(X86Reg::RDX, 1);
    enc.call_label("__pyb_print_str");
    // NEG RSI
    enc.rex_w(); enc.emit(&[0xF7, 0xDE]);
    enc.label(&skip_neg);

    // Convert digits: use rsp+32..rsp+63 as buffer (right to left)
    // RDI = end of buffer pointer
    // lea rdi, [rsp+63]
    enc.emit(&[0x48, 0x8D, 0x7C, 0x24, 0x3F]);
    enc.xor_rr(X86Reg::RBX); // digit count = 0
    enc.mov_rr(X86Reg::RAX, X86Reg::RSI);

    // Handle zero special case
    enc.rex_w(); enc.emit(&[0x85, 0xC0]); // TEST RAX, RAX
    let not_zero = format!("__itoa_nz_{}", enc.pos());
    enc.jcc(0x85, &not_zero); // JNE
    // It's zero: store '0'
    enc.emit(&[0xC6, 0x07, 0x30]); // mov byte [rdi], '0'
    enc.mov_imm64(X86Reg::RBX, 1);
    let done_digits = format!("__itoa_done_{}", enc.pos());
    enc.jmp(&done_digits);

    enc.label(&not_zero);
    // Loop: divide by 10
    let loop_label = format!("__itoa_loop_{}", enc.pos());
    enc.label(&loop_label);
    enc.rex_w(); enc.emit(&[0x85, 0xC0]); // TEST RAX, RAX
    let end_loop = format!("__itoa_end_{}", enc.pos());
    enc.jcc(0x84, &end_loop); // JE

    // XOR RDX, RDX; MOV RCX, 10; DIV RCX → RAX=quotient, RDX=remainder
    enc.xor_rr(X86Reg::RDX);
    enc.mov_imm64(X86Reg::RCX, 10);
    enc.rex_w(); enc.emit(&[0xF7, 0xF1]); // DIV RCX (unsigned)

    // digit = RDX + '0'
    enc.rex_w(); enc.emit(&[0x83, 0xC2, 0x30]); // ADD RDX, 0x30
    // mov [rdi], dl
    enc.emit(&[0x88, 0x17]);
    // dec rdi
    enc.rex_w(); enc.emit(&[0xFF, 0xCF]);
    // inc rbx
    enc.rex_w(); enc.emit(&[0xFF, 0xC3]);
    enc.jmp(&loop_label);

    enc.label(&end_loop);
    // rdi+1 points to first digit, rbx = count
    // inc rdi (point to first digit)
    enc.rex_w(); enc.emit(&[0xFF, 0xC7]);

    enc.label(&done_digits);
    // Print: rcx=rdi, rdx=rbx
    enc.mov_rr(X86Reg::RCX, X86Reg::RDI);
    enc.mov_rr(X86Reg::RDX, X86Reg::RBX);
    enc.call_label("__pyb_print_str");

    enc.add_rsp(80);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_ftoa: XMM0=f64 → print float to stdout
    // Strategy: convert to string in Rust-like fashion using integer math
    // 1. Handle sign  2. Print integer part  3. Print '.'  4. Print decimal part
    enc.label("__pyb_ftoa");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(96); // shadow(32) + buf(32) + locals(32)

    // Check sign: UCOMISD XMM0, 0.0
    enc.xorpd(1); // XMM1 = 0.0
    enc.ucomisd(0, 1);
    let ftoa_pos = format!("__ftoa_pos_{}", enc.pos());
    enc.jcc(0x83, &ftoa_pos); // JAE (not below = positive or zero)
    // Negative: print '-'
    enc.emit(&[0xC6, 0x44, 0x24, 0x20, 0x2D]); // mov byte [rsp+32], '-'
    enc.emit(&[0x48, 0x8D, 0x4C, 0x24, 0x20]); // lea rcx, [rsp+32]
    enc.mov_imm64(X86Reg::RDX, 1);
    enc.call_label("__pyb_print_str");
    // Negate: XMM0 = -XMM0 (XOR with sign bit)
    // Actually simpler: SUBSD XMM1, XMM0 then MOVSD XMM0, XMM1 (XMM1 was 0)
    enc.xorpd(1);
    enc.subsd(1, 0);
    enc.movsd_xmm_xmm(0, 1);
    enc.label(&ftoa_pos);

    // Save XMM0 to stack [rsp+48] (we'll need it after printing integer part)
    // MOVSD [rsp+0x30], XMM0
    enc.emit(&[0xF2, 0x0F, 0x11, 0x44, 0x24, 0x30]);

    // Integer part: CVTTSD2SI RAX, XMM0
    enc.cvttsd2si(X86Reg::RAX, 0);
    // Save integer part in RSI
    enc.mov_rr(X86Reg::RSI, X86Reg::RAX);
    // Print integer part via __pyb_itoa
    enc.call_label("__pyb_itoa");

    // Print '.'
    enc.emit(&[0xC6, 0x44, 0x24, 0x20, 0x2E]); // mov byte [rsp+32], '.'
    enc.emit(&[0x48, 0x8D, 0x4C, 0x24, 0x20]); // lea rcx, [rsp+32]
    enc.mov_imm64(X86Reg::RDX, 1);
    enc.call_label("__pyb_print_str");

    // Fractional part: XMM0 = saved_value - integer_part
    // Reload saved float
    enc.emit(&[0xF2, 0x0F, 0x10, 0x44, 0x24, 0x30]); // MOVSD XMM0, [rsp+0x30]
    // Convert integer part back to float in XMM1
    enc.cvtsi2sd(1, X86Reg::RSI);
    // XMM0 = XMM0 - XMM1 (fractional part)
    enc.subsd(0, 1);
    // Multiply by 1,000,000 to get 6 decimal digits
    enc.mulsd_data(0, "__f64_1e6");
    // CVTTSD2SI RAX, XMM0
    enc.cvttsd2si(X86Reg::RAX, 0);
    // Make sure it's positive (in case of tiny floating point errors)
    // TEST RAX, RAX; JNS skip; NEG RAX; skip:
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    let ftoa_frac_pos = format!("__ftoa_fp_{}", enc.pos());
    enc.jcc(0x89, &ftoa_frac_pos); // JNS
    enc.rex_w(); enc.emit(&[0xF7, 0xD8]); // NEG RAX
    enc.label(&ftoa_frac_pos);

    // Save frac value in RDI
    enc.mov_rr(X86Reg::RDI, X86Reg::RAX);

    // Convert fractional digits to buffer, right-to-left, exactly 6 digits
    // We'll write 6 digits into buf at [rsp+32..rsp+37], then trim trailing zeros
    // lea rbx, [rsp+38] (one past end, DEC before each write)
    enc.emit(&[0x48, 0x8D, 0x5C, 0x24, 0x26]); // lea rbx, [rsp+38]
    enc.mov_imm64(X86Reg::RSI, 6); // digit count
    enc.mov_rr(X86Reg::RAX, X86Reg::RDI);

    // Loop: extract 6 digits
    let frac_loop = format!("__ftoa_fl_{}", enc.pos());
    enc.label(&frac_loop);
    enc.rex_w(); enc.emit(&[0x83, 0xFE, 0x00]); // CMP RSI, 0
    let frac_done = format!("__ftoa_fd_{}", enc.pos());
    enc.jcc(0x84, &frac_done); // JE done
    enc.xor_rr(X86Reg::RDX);
    enc.mov_imm64(X86Reg::RCX, 10);
    enc.rex_w(); enc.emit(&[0xF7, 0xF1]); // DIV RCX
    enc.rex_w(); enc.emit(&[0x83, 0xC2, 0x30]); // ADD RDX, '0'
    // DEC RBX
    enc.rex_w(); enc.emit(&[0xFF, 0xCB]);
    // MOV [RBX], DL
    enc.emit(&[0x88, 0x13]);
    // DEC RSI
    enc.rex_w(); enc.emit(&[0xFF, 0xCE]);
    enc.jmp(&frac_loop);
    enc.label(&frac_done);

    // Now buf at [rsp+32..rsp+37] has 6 digits. Trim trailing '0's.
    // RBX points to start (rsp+32 after loop), end = rsp+37
    // lea rdi, [rsp+38] = one past last digit
    enc.emit(&[0x48, 0x8D, 0x7C, 0x24, 0x26]); // lea rdi, [rsp+38]
    // Trim loop: while (rdi-1) > rbx && *(rdi-1) == '0', rdi--
    let trim_loop = format!("__ftoa_tl_{}", enc.pos());
    enc.label(&trim_loop);
    // DEC RDI (tentative)
    enc.rex_w(); enc.emit(&[0xFF, 0xCF]);
    // CMP RDI, RBX
    enc.cmp_rr(X86Reg::RDI, X86Reg::RBX);
    let trim_done = format!("__ftoa_td_{}", enc.pos());
    enc.jcc(0x86, &trim_done); // JBE (rdi <= rbx, keep at least 1 digit)
    // CMP byte [RDI], '0'
    enc.emit(&[0x80, 0x3F, 0x30]);
    enc.jcc(0x84, &trim_loop); // JE → continue trimming
    // Not '0' → undo the DEC (we want rdi to point past last non-zero)
    enc.rex_w(); enc.emit(&[0xFF, 0xC7]); // INC RDI
    enc.label(&trim_done);

    // Print: ptr=RBX, len=RDI-RBX
    enc.mov_rr(X86Reg::RCX, X86Reg::RBX);
    enc.mov_rr(X86Reg::RDX, X86Reg::RDI);
    enc.sub_rr(X86Reg::RDX, X86Reg::RBX);
    // Ensure at least 1 digit (for x.0 case)
    enc.rex_w(); enc.emit(&[0x83, 0xFA, 0x01]); // CMP RDX, 1
    let print_frac = format!("__ftoa_pf_{}", enc.pos());
    enc.jcc(0x8D, &print_frac); // JGE
    enc.mov_imm64(X86Reg::RDX, 1);
    enc.label(&print_frac);
    enc.call_label("__pyb_print_str");

    enc.add_rsp(96);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_pow: RAX=base, RCX=exponent → RAX=result (integer power)
    enc.label("__pyb_pow");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RCX);
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // base
    enc.mov_imm64(X86Reg::RAX, 1);        // result = 1
    let pow_loop = format!("__pow_l_{}", enc.pos());
    enc.label(&pow_loop);
    enc.rex_w(); enc.emit(&[0x85, 0xC9]); // TEST RCX, RCX
    let pow_done = format!("__pow_d_{}", enc.pos());
    enc.jcc(0x84, &pow_done); // JE → done
    enc.imul_rr(X86Reg::RAX, X86Reg::RBX); // result *= base
    enc.rex_w(); enc.emit(&[0xFF, 0xC9]); // DEC RCX
    enc.jmp(&pow_loop);
    enc.label(&pow_done);
    enc.pop(X86Reg::RCX);
    enc.pop(X86Reg::RBX);
    enc.ret();
}

// Callee-saved registers that we may use for local variables
const CALLEE_SAVED_ORDER: &[X86Reg] = &[
    X86Reg::RBX, X86Reg::R12, X86Reg::R13, X86Reg::R14, X86Reg::R15,
    X86Reg::RSI, X86Reg::RDI,
];

fn get_used_callee_saved(func: &AllocatedFunction) -> Vec<X86Reg> {
    let mut saved = vec![X86Reg::RBX]; // Always save RBX
    for &reg in CALLEE_SAVED_ORDER {
        if reg == X86Reg::RBX { continue; }
        if func.reg_map.iter().any(|(_, r)| *r == reg) {
            saved.push(reg);
        }
    }
    saved
}

fn emit_function_epilogue(saved_regs: &[X86Reg], stack_size: usize, enc: &mut Encoder) {
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
fn compile_function(func: &AllocatedFunction, enc: &mut Encoder, _target: Target) {
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

    for instr in &func.body {
        compile_instruction(instr, func, enc, &saved_regs, stack);
    }

    if !func.body.iter().any(|i| matches!(i, IRInstruction::Return | IRInstruction::ReturnVoid)) {
        emit_function_epilogue(&saved_regs, stack, enc);
    }
}

fn compile_instruction(instr: &IRInstruction, func: &AllocatedFunction, enc: &mut Encoder, saved_regs: &[X86Reg], stack_size: usize) {
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
        IRInstruction::Call { func: callee, args } => {
            // Push args into Windows ABI regs
            let abi = [X86Reg::RCX, X86Reg::RDX, X86Reg::R8, X86Reg::R9];
            for (i, arg) in args.iter().enumerate().take(4) {
                compile_instruction(arg, func, enc, saved_regs, stack_size);
                if i < abi.len() { enc.mov_rr(abi[i], X86Reg::RAX); }
            }
            enc.call_label(callee);
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
            // XMM0 already has the float (from LoadConst::Float or variable load)
            // If value was in RAX (from variable), convert to XMM0
            enc.sub_rsp(32);
            enc.call_label("__pyb_ftoa");
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
        _ => {}
    }
}
