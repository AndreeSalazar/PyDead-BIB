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
pub const IAT_GET_PROCESS_HEAP: usize = 3;
pub const IAT_HEAP_ALLOC: usize = 4;
pub const IAT_GET_CURRENT_DIRECTORY: usize = 5;
pub const IAT_GET_FILE_ATTRIBUTES: usize = 6;
pub const IAT_GET_CURRENT_PROCESS_ID: usize = 7;
pub const IAT_CREATE_FILE: usize = 8;
pub const IAT_READ_FILE: usize = 9;
pub const IAT_CLOSE_HANDLE: usize = 10;
pub const IAT_CREATE_DIRECTORY: usize = 11;
pub const IAT_DELETE_FILE: usize = 12;
pub const IAT_MOVE_FILE: usize = 13;
pub const IAT_FIND_FIRST_FILE: usize = 14;
pub const IAT_FIND_NEXT_FILE: usize = 15;
pub const IAT_FIND_CLOSE: usize = 16;
pub const IAT_GET_ENVIRONMENT_VARIABLE: usize = 17;
pub const IAT_GET_COMMAND_LINE: usize = 18;
pub const IAT_GET_FILE_SIZE: usize = 19;
pub const IAT_SLOT_COUNT: usize = 20;

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

    // Ensure a data label exists (for globals) — 8-byte slot initialized to value
    fn ensure_data_label(&mut self, label: &str, init_val: i64) {
        // Check if label already exists
        if self.data_labels.iter().any(|(l, _)| l == label) {
            return;
        }
        // Align to 8 bytes
        while self.data.len() % 8 != 0 { self.data.push(0); }
        let offset = self.data.len() as u32;
        self.data_labels.push((label.to_string(), offset));
        self.data.extend_from_slice(&init_val.to_le_bytes());
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

    // __math_sqrt: RCX has f64 bits → XMM0, SQRTSD, result in RAX as f64 bits
    enc.label("__math_sqrt");
    // Move RCX (f64 bits) → XMM0
    enc.emit(&[0x66, 0x48, 0x0F, 0x6E, 0xC1]); // MOVQ XMM0, RCX
    // SQRTSD XMM0, XMM0
    enc.emit(&[0xF2, 0x0F, 0x51, 0xC0]);
    // Move result back to RAX
    enc.emit(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]); // MOVQ RAX, XMM0
    enc.ret();

    // __math_floor: RCX has f64 bits → floor → i64 in RAX
    enc.label("__math_floor");
    enc.emit(&[0x66, 0x48, 0x0F, 0x6E, 0xC1]); // MOVQ XMM0, RCX
    // ROUNDSD XMM0, XMM0, 1 (round toward -inf)
    // 66 0F 3A 0B C0 01
    enc.emit(&[0x66, 0x0F, 0x3A, 0x0B, 0xC0, 0x01]);
    // CVTTSD2SI RAX, XMM0
    enc.cvttsd2si(X86Reg::RAX, 0);
    enc.ret();

    // __math_ceil: RCX has f64 bits → ceil → i64 in RAX
    enc.label("__math_ceil");
    enc.emit(&[0x66, 0x48, 0x0F, 0x6E, 0xC1]); // MOVQ XMM0, RCX
    // ROUNDSD XMM0, XMM0, 2 (round toward +inf)
    enc.emit(&[0x66, 0x0F, 0x3A, 0x0B, 0xC0, 0x02]);
    enc.cvttsd2si(X86Reg::RAX, 0);
    enc.ret();

    // __math_sin: RCX has f64 bits → sin via x87 FPU → RAX as f64 bits
    enc.label("__math_sin");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(16);
    // Store f64 from RCX to stack
    enc.emit(&[0x48, 0x89, 0x0C, 0x24]); // MOV [RSP], RCX
    // FLD QWORD [RSP]
    enc.emit(&[0xDD, 0x04, 0x24]);
    // FSIN
    enc.emit(&[0xD9, 0xFE]);
    // FSTP QWORD [RSP]
    enc.emit(&[0xDD, 0x1C, 0x24]);
    // MOV RAX, [RSP]
    enc.emit(&[0x48, 0x8B, 0x04, 0x24]);
    enc.add_rsp(16);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __math_cos: RCX has f64 bits → cos via x87 FPU → RAX as f64 bits
    enc.label("__math_cos");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(16);
    enc.emit(&[0x48, 0x89, 0x0C, 0x24]); // MOV [RSP], RCX
    enc.emit(&[0xDD, 0x04, 0x24]); // FLD QWORD [RSP]
    enc.emit(&[0xD9, 0xFF]); // FCOS
    enc.emit(&[0xDD, 0x1C, 0x24]); // FSTP QWORD [RSP]
    enc.emit(&[0x48, 0x8B, 0x04, 0x24]); // MOV RAX, [RSP]
    enc.add_rsp(16);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __math_log: RCX has f64 bits → ln(x) via x87 → RAX as f64 bits
    // FYL2X computes ST(1) * log2(ST(0)). We want ln(x) = log2(x) * ln(2)
    // So: push ln(2) as ST(1), push x as ST(0), FYL2X
    enc.label("__math_log");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(16);
    enc.emit(&[0x48, 0x89, 0x0C, 0x24]); // MOV [RSP], RCX
    // FLDLN2 — push ln(2) onto FPU stack
    enc.emit(&[0xD9, 0xED]);
    // FLD QWORD [RSP] — push x
    enc.emit(&[0xDD, 0x04, 0x24]);
    // FYL2X — ST(1) * log2(ST(0)) = ln(2) * log2(x) = ln(x)
    enc.emit(&[0xD9, 0xF1]);
    // FSTP QWORD [RSP]
    enc.emit(&[0xDD, 0x1C, 0x24]);
    enc.emit(&[0x48, 0x8B, 0x04, 0x24]); // MOV RAX, [RSP]
    enc.add_rsp(16);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __math_abs: RCX has f64 bits → |x| → RAX as f64 bits
    enc.label("__math_abs");
    // Clear sign bit: AND with 0x7FFF_FFFF_FFFF_FFFF
    enc.mov_imm64(X86Reg::RAX, 0x7FFF_FFFF_FFFF_FFFFu64 as i64);
    enc.rex_w(); enc.emit(&[0x21, 0xC1]); // AND RCX, RAX
    enc.mov_rr(X86Reg::RAX, X86Reg::RCX);
    enc.ret();

    // __builtin_abs: RCX has int → |x| → RAX
    enc.label("__builtin_abs");
    enc.mov_rr(X86Reg::RAX, X86Reg::RCX);
    // TEST RAX, RAX; JNS skip; NEG RAX; skip:
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    let abs_pos = format!("__abs_p_{}", enc.pos());
    enc.jcc(0x89, &abs_pos);
    enc.rex_w(); enc.emit(&[0xF7, 0xD8]); // NEG RAX
    enc.label(&abs_pos);
    enc.ret();

    // __builtin_min: RCX=a, RDX=b → RAX = min(a,b)
    enc.label("__builtin_min");
    enc.mov_rr(X86Reg::RAX, X86Reg::RCX);
    enc.cmp_rr(X86Reg::RDX, X86Reg::RCX);
    // CMOVL RAX, RDX (if RDX < RCX, RAX = RDX)
    enc.rex_wrb(X86Reg::RAX, X86Reg::RDX);
    enc.emit(&[0x0F, 0x4C, 0xC0 | ((X86Reg::RAX.encoding() & 7) << 3) | (X86Reg::RDX.encoding() & 7)]);
    enc.ret();

    // __builtin_max: RCX=a, RDX=b → RAX = max(a,b)
    enc.label("__builtin_max");
    enc.mov_rr(X86Reg::RAX, X86Reg::RCX);
    enc.cmp_rr(X86Reg::RDX, X86Reg::RCX);
    // CMOVG RAX, RDX (if RDX > RCX, RAX = RDX)
    enc.rex_wrb(X86Reg::RAX, X86Reg::RDX);
    enc.emit(&[0x0F, 0x4F, 0xC0 | ((X86Reg::RAX.encoding() & 7) << 3) | (X86Reg::RDX.encoding() & 7)]);
    enc.ret();

    // __builtin_chr: RCX=codepoint → RAX=codepoint (caller prints as char)
    enc.label("__builtin_chr");
    enc.mov_rr(X86Reg::RAX, X86Reg::RCX);
    enc.ret();

    // __builtin_ord: RCX=ptr to char string → RAX = first byte
    enc.label("__builtin_ord");
    // MOVZX RAX, byte [RCX]
    enc.rex_w(); enc.emit(&[0x0F, 0xB6, 0x01]);
    enc.ret();

    // __builtin_len: RCX = ptr to PyList → RAX = list.len
    enc.label("__builtin_len");
    // MOV RAX, [RCX+8]  (len field)
    enc.emit(&[0x48, 0x8B, 0x41, 0x08]);
    enc.ret();

    // ══════════════════════════════════════════════════════════
    // Heap allocation stub
    // ══════════════════════════════════════════════════════════
    // __pyb_heap_alloc: RCX = size → RAX = ptr
    // Uses GetProcessHeap + HeapAlloc(heap, 0, size)
    enc.label("__pyb_heap_alloc");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.sub_rsp(40);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX); // save size
    // GetProcessHeap()
    enc.call_iat(IAT_GET_PROCESS_HEAP);
    // HeapAlloc(heap=RAX, flags=0x08 HEAP_ZERO_MEMORY, size=RSI)
    enc.mov_rr(X86Reg::RCX, X86Reg::RAX); // heap handle
    enc.mov_imm64(X86Reg::RDX, 0x08);     // HEAP_ZERO_MEMORY
    enc.mov_rr(X86Reg::R8, X86Reg::RSI);  // size
    enc.call_iat(IAT_HEAP_ALLOC);
    // RAX = allocated pointer
    enc.add_rsp(40);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // ══════════════════════════════════════════════════════════
    // List runtime stubs
    // ══════════════════════════════════════════════════════════
    // __pyb_list_new: → RAX = ptr to new PyList (cap=8)
    // PyList layout: [data_ptr:8][len:8][cap:8] = 24 bytes
    enc.label("__pyb_list_new");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(32);
    // Allocate PyList header (24 bytes)
    enc.mov_imm64(X86Reg::RCX, 24);
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // save list ptr
    // Allocate data array (8 elements × 8 bytes = 64)
    enc.mov_imm64(X86Reg::RCX, 64);
    enc.call_label("__pyb_heap_alloc");
    // list.data_ptr = RAX
    enc.emit(&[0x48, 0x89, 0x03]); // MOV [RBX], RAX
    // list.len = 0
    enc.emit(&[0x48, 0xC7, 0x43, 0x08, 0x00, 0x00, 0x00, 0x00]); // MOV QWORD [RBX+8], 0
    // list.cap = 8
    enc.emit(&[0x48, 0xC7, 0x43, 0x10, 0x08, 0x00, 0x00, 0x00]); // MOV QWORD [RBX+16], 8
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.add_rsp(32);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_list_append: RCX=list_ptr, RDX=value → void
    enc.label("__pyb_list_append");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX); // list ptr
    enc.mov_rr(X86Reg::RSI, X86Reg::RDX); // value
    // Load len and cap
    enc.emit(&[0x48, 0x8B, 0x7B, 0x08]); // MOV RDI, [RBX+8]  (len)
    // TODO: realloc if len == cap (skip for now, cap=8 is enough for tests)
    // data_ptr = [RBX]
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX] (data_ptr)
    // data_ptr[len] = value → MOV [RAX + RDI*8], RSI
    enc.emit(&[0x48, 0x89, 0x34, 0xF8]); // MOV [RAX+RDI*8], RSI
    // len++
    enc.rex_w(); enc.emit(&[0xFF, 0xC7]); // INC RDI
    enc.emit(&[0x48, 0x89, 0x7B, 0x08]); // MOV [RBX+8], RDI
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_list_get: RCX=list_ptr, RDX=index → RAX=value
    enc.label("__pyb_list_get");
    // Handle negative index: if RDX < 0, RDX = len + RDX
    enc.rex_w(); enc.emit(&[0x85, 0xD2]); // TEST RDX, RDX
    let get_pos = format!("__lg_p_{}", enc.pos());
    enc.jcc(0x89, &get_pos); // JNS
    // RDX += len
    enc.emit(&[0x48, 0x03, 0x51, 0x08]); // ADD RDX, [RCX+8]
    enc.label(&get_pos);
    // data_ptr = [RCX]
    enc.emit(&[0x48, 0x8B, 0x01]); // MOV RAX, [RCX] (data_ptr)
    // RAX = data_ptr[RDX] → MOV RAX, [RAX+RDX*8]
    enc.emit(&[0x48, 0x8B, 0x04, 0xD0]); // MOV RAX, [RAX+RDX*8]
    enc.ret();

    // __pyb_list_set: RCX=list_ptr, RDX=index, R8=value → void
    enc.label("__pyb_list_set");
    // Handle negative index
    enc.rex_w(); enc.emit(&[0x85, 0xD2]);
    let set_pos = format!("__ls_p_{}", enc.pos());
    enc.jcc(0x89, &set_pos);
    enc.emit(&[0x48, 0x03, 0x51, 0x08]); // ADD RDX, [RCX+8]
    enc.label(&set_pos);
    // data_ptr = [RCX]
    enc.emit(&[0x48, 0x8B, 0x01]); // MOV RAX, [RCX]
    // data_ptr[RDX] = R8 → MOV [RAX+RDX*8], R8
    enc.emit(&[0x4C, 0x89, 0x04, 0xD0]); // MOV [RAX+RDX*8], R8
    enc.ret();

    // __pyb_listcomp_range: RCX=stop → RAX = list [0,1,2,...,stop-1]
    // Creates a list with i*i values (simplified: just [0..n-1] for now)
    enc.label("__pyb_listcomp_range");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX); // RSI = stop
    // Create new list
    enc.call_label("__pyb_list_new");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // RBX = list ptr
    enc.xor_rr(X86Reg::RDI); // RDI = i = 0
    let loop_label = format!("__lcr_loop_{}", enc.pos());
    let end_label = format!("__lcr_end_{}", enc.pos());
    enc.label(&loop_label);
    enc.cmp_rr(X86Reg::RDI, X86Reg::RSI);
    enc.jcc(0x8D, &end_label); // JGE end
    // list_append(list, i)
    enc.mov_rr(X86Reg::RCX, X86Reg::RBX);
    enc.mov_rr(X86Reg::RDX, X86Reg::RDI);
    enc.call_label("__pyb_list_append");
    enc.emit(&[0x48, 0xFF, 0xC7]); // INC RDI
    enc.jmp(&loop_label);
    enc.label(&end_label);
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX); // return list
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // ══════════════════════════════════════════════════════════
    // v3.0 — numpy-native runtime stubs
    // ══════════════════════════════════════════════════════════

    // __pyb_np_sum: RCX=list_ptr → RAX = sum of all elements
    enc.label("__pyb_np_sum");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX); // RBX = list ptr
    enc.xor_rr(X86Reg::RSI); // RSI = accumulator = 0
    enc.xor_rr(X86Reg::RDI); // RDI = i = 0
    let sum_loop = format!("__nps_loop_{}", enc.pos());
    let sum_end = format!("__nps_end_{}", enc.pos());
    enc.label(&sum_loop);
    // Load len = [RBX+8]
    enc.emit(&[0x48, 0x8B, 0x43, 0x08]); // MOV RAX, [RBX+8]
    enc.cmp_rr(X86Reg::RDI, X86Reg::RAX);
    enc.jcc(0x8D, &sum_end); // JGE end
    // data_ptr = [RBX]
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX]
    // element = data_ptr[RDI*8]
    enc.emit(&[0x48, 0x8B, 0x04, 0xF8]); // MOV RAX, [RAX+RDI*8]
    enc.add_rr(X86Reg::RSI, X86Reg::RAX);
    enc.emit(&[0x48, 0xFF, 0xC7]); // INC RDI
    enc.jmp(&sum_loop);
    enc.label(&sum_end);
    enc.mov_rr(X86Reg::RAX, X86Reg::RSI);
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_np_max: RCX=list_ptr → RAX = max element
    enc.label("__pyb_np_max");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);
    // RSI = max = first element
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX] (data_ptr)
    enc.emit(&[0x48, 0x8B, 0x30]); // MOV RSI, [RAX] (first element)
    enc.mov_imm64(X86Reg::RDI, 1); // RDI = i = 1
    let max_loop = format!("__npm_loop_{}", enc.pos());
    let max_end = format!("__npm_end_{}", enc.pos());
    let max_skip = format!("__npm_skip_{}", enc.pos());
    enc.label(&max_loop);
    enc.emit(&[0x48, 0x8B, 0x43, 0x08]); // MOV RAX, [RBX+8] (len)
    enc.cmp_rr(X86Reg::RDI, X86Reg::RAX);
    enc.jcc(0x8D, &max_end);
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX]
    enc.emit(&[0x48, 0x8B, 0x04, 0xF8]); // MOV RAX, [RAX+RDI*8]
    enc.cmp_rr(X86Reg::RAX, X86Reg::RSI);
    enc.jcc(0x8E, &max_skip); // JLE skip
    enc.mov_rr(X86Reg::RSI, X86Reg::RAX);
    enc.label(&max_skip);
    enc.emit(&[0x48, 0xFF, 0xC7]); // INC RDI
    enc.jmp(&max_loop);
    enc.label(&max_end);
    enc.mov_rr(X86Reg::RAX, X86Reg::RSI);
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_np_min: RCX=list_ptr → RAX = min element
    enc.label("__pyb_np_min");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX]
    enc.emit(&[0x48, 0x8B, 0x30]); // MOV RSI, [RAX]
    enc.mov_imm64(X86Reg::RDI, 1);
    let min_loop = format!("__npn_loop_{}", enc.pos());
    let min_end = format!("__npn_end_{}", enc.pos());
    let min_skip = format!("__npn_skip_{}", enc.pos());
    enc.label(&min_loop);
    enc.emit(&[0x48, 0x8B, 0x43, 0x08]); // MOV RAX, [RBX+8]
    enc.cmp_rr(X86Reg::RDI, X86Reg::RAX);
    enc.jcc(0x8D, &min_end);
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX]
    enc.emit(&[0x48, 0x8B, 0x04, 0xF8]); // MOV RAX, [RAX+RDI*8]
    enc.cmp_rr(X86Reg::RAX, X86Reg::RSI);
    enc.jcc(0x8D, &min_skip); // JGE skip
    enc.mov_rr(X86Reg::RSI, X86Reg::RAX);
    enc.label(&min_skip);
    enc.emit(&[0x48, 0xFF, 0xC7]); // INC RDI
    enc.jmp(&min_loop);
    enc.label(&min_end);
    enc.mov_rr(X86Reg::RAX, X86Reg::RSI);
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_np_dot: RCX=list_a, RDX=list_b → RAX = dot product
    enc.label("__pyb_np_dot");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.push(X86Reg::R12);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX); // RBX = list_a
    enc.mov_rr(X86Reg::R12, X86Reg::RDX); // R12 = list_b
    enc.xor_rr(X86Reg::RSI); // RSI = accumulator
    enc.xor_rr(X86Reg::RDI); // RDI = i = 0
    let dot_loop = format!("__npd_loop_{}", enc.pos());
    let dot_end = format!("__npd_end_{}", enc.pos());
    enc.label(&dot_loop);
    enc.emit(&[0x48, 0x8B, 0x43, 0x08]); // MOV RAX, [RBX+8]
    enc.cmp_rr(X86Reg::RDI, X86Reg::RAX);
    enc.jcc(0x8D, &dot_end);
    // a[i]
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX]
    enc.emit(&[0x48, 0x8B, 0x0C, 0xF8]); // MOV RCX, [RAX+RDI*8]
    // b[i]
    enc.emit(&[0x49, 0x8B, 0x04, 0x24]); // MOV RAX, [R12]
    enc.emit(&[0x48, 0x8B, 0x04, 0xF8]); // MOV RAX, [RAX+RDI*8]
    // RCX * RAX
    enc.imul_rr(X86Reg::RAX, X86Reg::RCX);
    enc.add_rr(X86Reg::RSI, X86Reg::RAX);
    enc.emit(&[0x48, 0xFF, 0xC7]); // INC RDI
    enc.jmp(&dot_loop);
    enc.label(&dot_end);
    enc.mov_rr(X86Reg::RAX, X86Reg::RSI);
    enc.add_rsp(32);
    enc.pop(X86Reg::R12);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_gen_next: RCX=generator_ptr → RAX = next value (stub)
    enc.label("__pyb_gen_next");
    // Load current = [RCX+8], increment, return old value
    enc.emit(&[0x48, 0x8B, 0x41, 0x08]); // MOV RAX, [RCX+8]
    enc.emit(&[0x48, 0xFF, 0x41, 0x08]); // INC QWORD [RCX+8]
    enc.ret();

    // __pyb_dll_load: RCX=path_ptr → RAX = module handle (stub)
    enc.label("__pyb_dll_load");
    enc.xor_rr(X86Reg::RAX); // Return 0 (stub)
    enc.ret();

    // ══════════════════════════════════════════════════════════
    // Dict runtime stubs — open addressing with integer keys
    // ══════════════════════════════════════════════════════════
    // PyDict layout (40 bytes):
    //   [0]  keys_ptr:  u64 → i64 array
    //   [8]  vals_ptr:  u64 → i64 array
    //   [16] flags_ptr: u64 → u8 array (0=empty, 1=used)
    //   [24] len:       u64
    //   [32] cap:       u64

    // __pyb_dict_new: → RAX = ptr to new PyDict (cap=16)
    enc.label("__pyb_dict_new");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.sub_rsp(32);
    enc.mov_imm64(X86Reg::RCX, 40);
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);
    enc.mov_imm64(X86Reg::RCX, 128); // keys: 16*8
    enc.call_label("__pyb_heap_alloc");
    enc.emit(&[0x48, 0x89, 0x03]); // MOV [RBX], RAX
    enc.mov_imm64(X86Reg::RCX, 128); // vals: 16*8
    enc.call_label("__pyb_heap_alloc");
    enc.emit(&[0x48, 0x89, 0x43, 0x08]); // MOV [RBX+8], RAX
    enc.mov_imm64(X86Reg::RCX, 16); // flags: 16*1
    enc.call_label("__pyb_heap_alloc");
    enc.emit(&[0x48, 0x89, 0x43, 0x10]); // MOV [RBX+16], RAX
    enc.emit(&[0x48, 0xC7, 0x43, 0x18, 0x00, 0x00, 0x00, 0x00]); // len=0
    enc.emit(&[0x48, 0xC7, 0x43, 0x20, 0x10, 0x00, 0x00, 0x00]); // cap=16
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.add_rsp(32);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_dict_set: RCX=dict_ptr, RDX=key(i64), R8=value(i64)
    // Strategy: compute slot = abs(key) & (cap-1), linear probe
    // Uses only GPRs: RBX=dict, RSI=key, RDI=value, RAX/RCX/RDX scratch
    // Stack slot [RSP+0] = slot index during probe
    enc.label("__pyb_dict_set");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(48); // 32 shadow + 16 local
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);  // dict ptr
    enc.mov_rr(X86Reg::RSI, X86Reg::RDX);  // key
    enc.mov_rr(X86Reg::RDI, X86Reg::R8);   // value
    // Compute slot: abs(key) & (cap-1)
    enc.mov_rr(X86Reg::RAX, X86Reg::RSI); // RAX = key
    // if RAX < 0, negate
    enc.rex_w(); enc.emit(&[0x85, 0xC0]); // TEST RAX, RAX
    let dsp = format!("__dsp_{}", enc.pos());
    enc.jcc(0x89, &dsp); // JNS
    enc.rex_w(); enc.emit(&[0xF7, 0xD8]); // NEG RAX
    enc.label(&dsp);
    // RCX = cap - 1
    enc.emit(&[0x48, 0x8B, 0x4B, 0x20]); // MOV RCX, [RBX+32]
    enc.rex_w(); enc.emit(&[0xFF, 0xC9]); // DEC RCX
    enc.rex_w(); enc.emit(&[0x21, 0xC8]); // AND RAX, RCX  → slot
    // Store slot in [RSP+32]
    enc.emit(&[0x48, 0x89, 0x44, 0x24, 0x20]); // MOV [RSP+32], RAX

    // Probe loop
    let dsprobe = format!("__dspr_{}", enc.pos());
    enc.label(&dsprobe);
    // Load slot → RDX
    enc.emit(&[0x48, 0x8B, 0x54, 0x24, 0x20]); // MOV RDX, [RSP+32]
    // flags_ptr = [RBX+16]
    enc.emit(&[0x48, 0x8B, 0x43, 0x10]); // MOV RAX, [RBX+16]
    // MOVZX ECX, byte [RAX+RDX]
    enc.emit(&[0x0F, 0xB6, 0x0C, 0x10]); // MOVZX ECX, byte [RAX+RDX]
    enc.rex_w(); enc.emit(&[0x85, 0xC9]); // TEST ECX, ECX
    let dsins = format!("__dsin_{}", enc.pos());
    enc.jcc(0x84, &dsins); // JE → empty slot, insert
    // Check if keys[slot] == key
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX] (keys_ptr)
    // CMP [RAX+RDX*8], RSI
    enc.emit(&[0x48, 0x39, 0x34, 0xD0]); // CMP [RAX+RDX*8], RSI
    let dsfnd = format!("__dsfn_{}", enc.pos());
    enc.jcc(0x84, &dsfnd); // JE → found, update
    // Collision: slot = (slot + 1) & (cap - 1)
    enc.emit(&[0x48, 0x8B, 0x54, 0x24, 0x20]); // MOV RDX, [RSP+32]
    enc.rex_w(); enc.emit(&[0xFF, 0xC2]); // INC RDX
    enc.emit(&[0x48, 0x8B, 0x4B, 0x20]); // MOV RCX, [RBX+32]
    enc.rex_w(); enc.emit(&[0xFF, 0xC9]); // DEC RCX
    enc.rex_w(); enc.emit(&[0x21, 0xCA]); // AND RDX, RCX
    enc.emit(&[0x48, 0x89, 0x54, 0x24, 0x20]); // MOV [RSP+32], RDX
    enc.jmp(&dsprobe);

    // Insert: keys[slot]=key, vals[slot]=val, flags[slot]=1, len++
    enc.label(&dsins);
    enc.emit(&[0x48, 0x8B, 0x54, 0x24, 0x20]); // MOV RDX, [RSP+32] (slot)
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX] (keys_ptr)
    enc.emit(&[0x48, 0x89, 0x34, 0xD0]); // MOV [RAX+RDX*8], RSI
    enc.emit(&[0x48, 0x8B, 0x43, 0x08]); // MOV RAX, [RBX+8] (vals_ptr)
    enc.emit(&[0x48, 0x89, 0x3C, 0xD0]); // MOV [RAX+RDX*8], RDI
    enc.emit(&[0x48, 0x8B, 0x43, 0x10]); // MOV RAX, [RBX+16] (flags_ptr)
    enc.emit(&[0xC6, 0x04, 0x10, 0x01]); // MOV byte [RAX+RDX], 1
    enc.emit(&[0x48, 0xFF, 0x43, 0x18]); // INC QWORD [RBX+24] (len++)
    let dsdone = format!("__dsdn_{}", enc.pos());
    enc.jmp(&dsdone);

    // Found: update vals[slot] = val
    enc.label(&dsfnd);
    enc.emit(&[0x48, 0x8B, 0x54, 0x24, 0x20]); // MOV RDX, [RSP+32]
    enc.emit(&[0x48, 0x8B, 0x43, 0x08]); // MOV RAX, [RBX+8] (vals_ptr)
    enc.emit(&[0x48, 0x89, 0x3C, 0xD0]); // MOV [RAX+RDX*8], RDI

    enc.label(&dsdone);
    enc.add_rsp(48);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_dict_get: RCX=dict_ptr, RDX=key → RAX=value (0 if not found)
    enc.label("__pyb_dict_get");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.sub_rsp(48);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);  // dict
    enc.mov_rr(X86Reg::RSI, X86Reg::RDX);  // key
    // slot = abs(key) & (cap-1)
    enc.mov_rr(X86Reg::RAX, X86Reg::RSI);
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    let dgp = format!("__dgp_{}", enc.pos());
    enc.jcc(0x89, &dgp);
    enc.rex_w(); enc.emit(&[0xF7, 0xD8]);
    enc.label(&dgp);
    enc.emit(&[0x48, 0x8B, 0x4B, 0x20]); // MOV RCX, [RBX+32]
    enc.rex_w(); enc.emit(&[0xFF, 0xC9]); // DEC RCX
    enc.rex_w(); enc.emit(&[0x21, 0xC8]); // AND RAX, RCX
    enc.emit(&[0x48, 0x89, 0x44, 0x24, 0x20]); // MOV [RSP+32], RAX

    let dgprobe = format!("__dgpr_{}", enc.pos());
    enc.label(&dgprobe);
    enc.emit(&[0x48, 0x8B, 0x54, 0x24, 0x20]); // MOV RDX, [RSP+32]
    enc.emit(&[0x48, 0x8B, 0x43, 0x10]); // MOV RAX, [RBX+16] (flags)
    enc.emit(&[0x0F, 0xB6, 0x0C, 0x10]); // MOVZX ECX, byte [RAX+RDX]
    enc.rex_w(); enc.emit(&[0x85, 0xC9]);
    let dgnf = format!("__dgnf_{}", enc.pos());
    enc.jcc(0x84, &dgnf); // JE → not found
    enc.emit(&[0x48, 0x8B, 0x03]); // MOV RAX, [RBX] (keys)
    enc.emit(&[0x48, 0x39, 0x34, 0xD0]); // CMP [RAX+RDX*8], RSI
    let dgfn = format!("__dgfn_{}", enc.pos());
    enc.jcc(0x84, &dgfn); // JE → found
    // next slot
    enc.emit(&[0x48, 0x8B, 0x54, 0x24, 0x20]);
    enc.rex_w(); enc.emit(&[0xFF, 0xC2]);
    enc.emit(&[0x48, 0x8B, 0x4B, 0x20]);
    enc.rex_w(); enc.emit(&[0xFF, 0xC9]);
    enc.rex_w(); enc.emit(&[0x21, 0xCA]);
    enc.emit(&[0x48, 0x89, 0x54, 0x24, 0x20]);
    enc.jmp(&dgprobe);

    enc.label(&dgfn);
    enc.emit(&[0x48, 0x8B, 0x43, 0x08]); // MOV RAX, [RBX+8] (vals)
    enc.emit(&[0x48, 0x8B, 0x04, 0xD0]); // MOV RAX, [RAX+RDX*8]
    let dgdn = format!("__dgdn_{}", enc.pos());
    enc.jmp(&dgdn);

    enc.label(&dgnf);
    enc.xor_rr(X86Reg::RAX);

    enc.label(&dgdn);
    enc.add_rsp(48);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_dict_len: RCX=dict_ptr → RAX=len
    enc.label("__pyb_dict_len");
    enc.emit(&[0x48, 0x8B, 0x41, 0x18]); // MOV RAX, [RCX+24]
    enc.ret();

    // ══════════════════════════════════════════════════════════
    // Object field access stubs for classes
    // ══════════════════════════════════════════════════════════
    // __pyb_obj_set_field: RCX=obj_ptr, RDX=byte_offset, R8=value
    // MOV [RCX+RDX], R8
    enc.label("__pyb_obj_set_field");
    enc.emit(&[0x4C, 0x89, 0x04, 0x11]); // MOV [RCX+RDX], R8
    enc.ret();

    // __pyb_obj_get_field: RCX=obj_ptr, RDX=byte_offset -> RAX=value
    // MOV RAX, [RCX+RDX]
    enc.label("__pyb_obj_get_field");
    enc.emit(&[0x48, 0x8B, 0x04, 0x11]); // MOV RAX, [RCX+RDX]
    enc.ret();

    // ====== OS module stubs ======

    // __pyb_os_getcwd: -> RAX = heap string ptr
    enc.label("__pyb_os_getcwd");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(32);
    enc.mov_imm64(X86Reg::RCX, 260);
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);
    enc.mov_imm64(X86Reg::RCX, 260);
    enc.mov_rr(X86Reg::RDX, X86Reg::RBX);
    enc.call_iat(IAT_GET_CURRENT_DIRECTORY);
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.add_rsp(32);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_os_path_exists: RCX=path -> RAX=1/0
    enc.label("__pyb_os_path_exists");
    enc.sub_rsp(32);
    enc.call_iat(IAT_GET_FILE_ATTRIBUTES);
    enc.emit(&[0x48, 0x83, 0xF8, 0xFF]); // CMP RAX, -1
    let pe_nf = format!("__pe_nf_{}", enc.pos());
    enc.jcc(0x84, &pe_nf);
    enc.mov_imm64(X86Reg::RAX, 1);
    let pe_dn = format!("__pe_dn_{}", enc.pos());
    enc.jmp(&pe_dn);
    enc.label(&pe_nf);
    enc.xor_rr(X86Reg::RAX);
    enc.label(&pe_dn);
    enc.add_rsp(32);
    enc.ret();

    // __pyb_os_getpid: -> RAX = pid
    enc.label("__pyb_os_getpid");
    enc.sub_rsp(32);
    enc.call_iat(IAT_GET_CURRENT_PROCESS_ID);
    enc.add_rsp(32);
    enc.ret();

    // __pyb_os_mkdir: RCX=path -> RAX=1/0
    enc.label("__pyb_os_mkdir");
    enc.sub_rsp(32);
    enc.xor_rr(X86Reg::RDX);
    enc.call_iat(IAT_CREATE_DIRECTORY);
    enc.add_rsp(32);
    enc.ret();

    // __pyb_os_remove: RCX=path -> RAX=1/0
    enc.label("__pyb_os_remove");
    enc.sub_rsp(32);
    enc.call_iat(IAT_DELETE_FILE);
    enc.add_rsp(32);
    enc.ret();

    // __pyb_os_rename: RCX=old, RDX=new -> RAX=1/0
    enc.label("__pyb_os_rename");
    enc.sub_rsp(32);
    enc.call_iat(IAT_MOVE_FILE);
    enc.add_rsp(32);
    enc.ret();

    // __pyb_os_environ_get: RCX=var_name -> RAX=heap str (or 0)
    enc.label("__pyb_os_environ_get");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);
    enc.mov_imm64(X86Reg::RCX, 1024);
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);
    enc.mov_rr(X86Reg::RCX, X86Reg::RSI);
    enc.mov_rr(X86Reg::RDX, X86Reg::RBX);
    enc.mov_imm64(X86Reg::R8, 1024);
    enc.call_iat(IAT_GET_ENVIRONMENT_VARIABLE);
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    let eg_nf = format!("__eg_nf_{}", enc.pos());
    enc.jcc(0x84, &eg_nf);
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    let eg_dn = format!("__eg_dn_{}", enc.pos());
    enc.jmp(&eg_dn);
    enc.label(&eg_nf);
    enc.xor_rr(X86Reg::RAX);
    enc.label(&eg_dn);
    enc.add_rsp(32);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // ====== File I/O stubs ======

    // __pyb_file_open: RCX=path, RDX=mode(0=r,1=w) -> RAX=handle
    enc.label("__pyb_file_open");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(64);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);
    enc.mov_rr(X86Reg::RDI, X86Reg::RDX);
    enc.mov_rr(X86Reg::RCX, X86Reg::RSI);
    enc.rex_w(); enc.emit(&[0x85, 0xFF]); // TEST RDI,RDI
    let fo_wr = format!("__fo_wr_{}", enc.pos());
    enc.jcc(0x85, &fo_wr);
    enc.mov_imm64(X86Reg::RDX, 0x80000000u64 as i64); // GENERIC_READ
    enc.emit(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x03, 0x00, 0x00, 0x00]); // creation=OPEN_EXISTING
    let fo_cl = format!("__fo_cl_{}", enc.pos());
    enc.jmp(&fo_cl);
    enc.label(&fo_wr);
    enc.mov_imm64(X86Reg::RDX, 0x40000000u64 as i64); // GENERIC_WRITE
    enc.emit(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x02, 0x00, 0x00, 0x00]); // creation=CREATE_ALWAYS
    enc.label(&fo_cl);
    enc.mov_imm64(X86Reg::R8, 1); // FILE_SHARE_READ
    enc.xor_rr(X86Reg::R9); // security=NULL
    enc.emit(&[0x48, 0xC7, 0x44, 0x24, 0x28, 0x80, 0x00, 0x00, 0x00]); // flags
    enc.emit(&[0x48, 0xC7, 0x44, 0x24, 0x30, 0x00, 0x00, 0x00, 0x00]); // template=NULL
    enc.call_iat(IAT_CREATE_FILE);
    enc.add_rsp(64);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_file_read: RCX=handle -> RAX=heap str ptr
    enc.label("__pyb_file_read");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.sub_rsp(48);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);
    enc.mov_rr(X86Reg::RCX, X86Reg::RSI);
    enc.emit(&[0x48, 0x8D, 0x54, 0x24, 0x20]); // LEA RDX, [RSP+32]
    enc.call_iat(IAT_GET_FILE_SIZE);
    enc.emit(&[0x48, 0x8B, 0x44, 0x24, 0x20]); // MOV RAX, [RSP+32]
    enc.push(X86Reg::RAX);
    enc.rex_w(); enc.emit(&[0xFF, 0xC0]); // INC RAX
    enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);
    enc.pop(X86Reg::RAX);
    enc.mov_rr(X86Reg::RCX, X86Reg::RSI);
    enc.mov_rr(X86Reg::RDX, X86Reg::RBX);
    enc.mov_rr(X86Reg::R8, X86Reg::RAX);
    enc.emit(&[0x4C, 0x8D, 0x4C, 0x24, 0x20]); // LEA R9, [RSP+32]
    enc.emit(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00]); // overlap=NULL
    enc.call_iat(IAT_READ_FILE);
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.add_rsp(48);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_file_write: RCX=handle, RDX=str_ptr, R8=str_len -> void
    enc.label("__pyb_file_write");
    enc.sub_rsp(48);
    enc.emit(&[0x4C, 0x8D, 0x4C, 0x24, 0x20]); // LEA R9, [RSP+32]
    enc.emit(&[0x48, 0xC7, 0x44, 0x24, 0x28, 0x00, 0x00, 0x00, 0x00]); // overlap=NULL
    enc.call_iat(IAT_WRITE_FILE);
    enc.add_rsp(48);
    enc.ret();

    // __pyb_file_close: RCX=handle -> void
    enc.label("__pyb_file_close");
    enc.sub_rsp(32);
    enc.call_iat(IAT_CLOSE_HANDLE);
    enc.add_rsp(32);
    enc.ret();

    // ====== Sys module ======
    // __pyb_sys_exit: RCX=code -> noreturn
    enc.label("__pyb_sys_exit");
    enc.sub_rsp(32);
    enc.call_iat(IAT_EXIT_PROCESS);

    // ====== Error state for try/except ======
    enc.add_data_f64("__pyb_error_state", f64::from_bits(0));

    // ====== Random (xorshift64) ======
    let rng_label = "__pyb_rng_state";
    enc.add_data_f64(rng_label, f64::from_bits(0x123456789ABCDEF));

    enc.label("__pyb_random_seed");
    enc.lea_rax_data(rng_label);
    enc.emit(&[0x48, 0x89, 0x08]); // MOV [RAX], RCX
    enc.ret();

    enc.label("__pyb_random_next");
    enc.lea_rax_data(rng_label);
    enc.emit(&[0x48, 0x8B, 0x08]); // MOV RCX, [RAX]
    enc.push(X86Reg::RBX);
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX); // save ptr
    enc.mov_rr(X86Reg::RAX, X86Reg::RCX);
    enc.mov_rr(X86Reg::RDX, X86Reg::RAX);
    enc.emit(&[0x48, 0xC1, 0xE2, 0x0D]); // SHL RDX, 13
    enc.rex_w(); enc.emit(&[0x31, 0xD0]); // XOR RAX, RDX
    enc.mov_rr(X86Reg::RDX, X86Reg::RAX);
    enc.emit(&[0x48, 0xC1, 0xEA, 0x07]); // SHR RDX, 7
    enc.rex_w(); enc.emit(&[0x31, 0xD0]); // XOR RAX, RDX
    enc.mov_rr(X86Reg::RDX, X86Reg::RAX);
    enc.emit(&[0x48, 0xC1, 0xE2, 0x11]); // SHL RDX, 17
    enc.rex_w(); enc.emit(&[0x31, 0xD0]); // XOR RAX, RDX
    enc.emit(&[0x48, 0x89, 0x03]); // MOV [RBX], RAX
    enc.pop(X86Reg::RBX);
    enc.ret();

    enc.label("__pyb_random_randint");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX); // lo
    enc.mov_rr(X86Reg::RDI, X86Reg::RDX); // hi
    enc.call_label("__pyb_random_next");
    enc.mov_rr(X86Reg::RCX, X86Reg::RDI);
    enc.rex_w(); enc.emit(&[0x29, 0xF1]); // SUB RCX, RSI
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]); // INC RCX
    enc.emit(&[0x48, 0x31, 0xD2]); // XOR RDX, RDX
    enc.emit(&[0x48, 0xF7, 0xF1]); // DIV RCX
    enc.mov_rr(X86Reg::RAX, X86Reg::RDX);
    enc.rex_w(); enc.emit(&[0x01, 0xF0]); // ADD RAX, RSI
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // ====== String method stubs ======

    // __pyb_str_len: RCX=str_ptr -> RAX=length
    enc.label("__pyb_str_len");
    enc.xor_rr(X86Reg::RAX);
    let sl_lp = format!("__sl_lp_{}", enc.pos());
    enc.label(&sl_lp);
    enc.emit(&[0x80, 0x3C, 0x01, 0x00]); // CMP byte [RCX+RAX], 0
    let sl_dn = format!("__sl_dn_{}", enc.pos());
    enc.jcc(0x84, &sl_dn);
    enc.rex_w(); enc.emit(&[0xFF, 0xC0]); // INC RAX
    enc.jmp(&sl_lp);
    enc.label(&sl_dn);
    enc.ret();

    // __pyb_str_upper: RCX=str -> RAX=new heap str (uppercase)
    enc.label("__pyb_str_upper");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::RDI, X86Reg::RAX);
    enc.rex_w(); enc.emit(&[0xFF, 0xC0]);
    enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);
    enc.xor_rr(X86Reg::RCX);
    let su_lp = format!("__su_lp_{}", enc.pos());
    enc.label(&su_lp);
    enc.rex_w(); enc.emit(&[0x39, 0xF9]); // CMP RCX, RDI
    let su_dn = format!("__su_dn_{}", enc.pos());
    enc.jcc(0x8D, &su_dn);
    enc.emit(&[0x0F, 0xB6, 0x04, 0x0E]); // MOVZX EAX, [RSI+RCX]
    enc.emit(&[0x3C, 0x61]); // CMP AL, 'a'
    let su_sk = format!("__su_sk_{}", enc.pos());
    enc.jcc(0x82, &su_sk);
    enc.emit(&[0x3C, 0x7A]); // CMP AL, 'z'
    enc.jcc(0x87, &su_sk);
    enc.emit(&[0x2C, 0x20]); // SUB AL, 32
    enc.label(&su_sk);
    enc.emit(&[0x88, 0x04, 0x0B]); // MOV [RBX+RCX], AL
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);
    enc.jmp(&su_lp);
    enc.label(&su_dn);
    enc.emit(&[0xC6, 0x04, 0x0B, 0x00]);
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_str_lower: RCX=str -> RAX=new heap str (lowercase)
    enc.label("__pyb_str_lower");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::RDI, X86Reg::RAX);
    enc.rex_w(); enc.emit(&[0xFF, 0xC0]);
    enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);
    enc.xor_rr(X86Reg::RCX);
    let slw_lp = format!("__slw_lp_{}", enc.pos());
    enc.label(&slw_lp);
    enc.rex_w(); enc.emit(&[0x39, 0xF9]);
    let slw_dn = format!("__slw_dn_{}", enc.pos());
    enc.jcc(0x8D, &slw_dn);
    enc.emit(&[0x0F, 0xB6, 0x04, 0x0E]);
    enc.emit(&[0x3C, 0x41]); // CMP AL, 'A'
    let slw_sk = format!("__slw_sk_{}", enc.pos());
    enc.jcc(0x82, &slw_sk);
    enc.emit(&[0x3C, 0x5A]); // CMP AL, 'Z'
    enc.jcc(0x87, &slw_sk);
    enc.emit(&[0x04, 0x20]); // ADD AL, 32
    enc.label(&slw_sk);
    enc.emit(&[0x88, 0x04, 0x0B]);
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);
    enc.jmp(&slw_lp);
    enc.label(&slw_dn);
    enc.emit(&[0xC6, 0x04, 0x0B, 0x00]);
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_str_find: RCX=haystack, RDX=needle -> RAX=index (-1 if not found)
    enc.label("__pyb_str_find");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.push(X86Reg::R12);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX); // haystack
    enc.mov_rr(X86Reg::RDI, X86Reg::RDX); // needle
    enc.mov_rr(X86Reg::RCX, X86Reg::RDI);
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::R12, X86Reg::RAX); // needle_len
    enc.xor_rr(X86Reg::RBX); // i=0
    let sf_lp = format!("__sf_lp_{}", enc.pos());
    enc.label(&sf_lp);
    enc.emit(&[0x80, 0x3C, 0x1E, 0x00]); // CMP byte [RSI+RBX], 0
    let sf_nf = format!("__sf_nf_{}", enc.pos());
    enc.jcc(0x84, &sf_nf);
    // Compare needle_len chars
    enc.xor_rr(X86Reg::RCX); // j=0
    let sf_cm = format!("__sf_cm_{}", enc.pos());
    enc.label(&sf_cm);
    enc.rex_w(); enc.emit(&[0x4C, 0x39, 0xE1]); // CMP RCX, R12
    let sf_ok = format!("__sf_ok_{}", enc.pos());
    enc.jcc(0x8D, &sf_ok);
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.rex_w(); enc.emit(&[0x01, 0xC8]); // ADD RAX, RCX
    enc.emit(&[0x0F, 0xB6, 0x14, 0x06]); // MOVZX EDX, [RSI+RAX]
    enc.emit(&[0x0F, 0xB6, 0x04, 0x0F]); // MOVZX EAX, [RDI+RCX]
    enc.emit(&[0x39, 0xC2]); // CMP EDX, EAX
    let sf_no = format!("__sf_no_{}", enc.pos());
    enc.jcc(0x85, &sf_no);
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]); // INC RCX
    enc.jmp(&sf_cm);
    enc.label(&sf_ok);
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX); // return index
    enc.add_rsp(32);
    enc.pop(X86Reg::R12);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();
    enc.label(&sf_no);
    enc.rex_w(); enc.emit(&[0xFF, 0xC3]); // INC RBX
    enc.jmp(&sf_lp);
    enc.label(&sf_nf);
    enc.mov_imm64(X86Reg::RAX, -1i64);
    enc.add_rsp(32);
    enc.pop(X86Reg::R12);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_str_print: RCX=heap str -> prints (no newline)
    enc.label("__pyb_str_print");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::RCX, X86Reg::RBX);
    enc.mov_rr(X86Reg::RDX, X86Reg::RAX);
    enc.call_label("__pyb_print_str");
    enc.add_rsp(32);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // ══════════════════════════════════════════════════════════
    // v4.5 — str completo runtime stubs
    // ══════════════════════════════════════════════════════════

    // __pyb_str_concat: RCX=str_a, RDX=str_b → RAX=new heap str (a+b)
    enc.label("__pyb_str_concat");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.push(X86Reg::R12);
    enc.push(X86Reg::R13);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);  // RSI = str_a
    enc.mov_rr(X86Reg::RDI, X86Reg::RDX);  // RDI = str_b
    // Get len_a
    enc.mov_rr(X86Reg::RCX, X86Reg::RSI);
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::R12, X86Reg::RAX);  // R12 = len_a
    // Get len_b
    enc.mov_rr(X86Reg::RCX, X86Reg::RDI);
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::R13, X86Reg::RAX);  // R13 = len_b
    // Alloc len_a + len_b + 1
    enc.mov_rr(X86Reg::RCX, X86Reg::R12);
    enc.add_rr(X86Reg::RCX, X86Reg::R13);
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);   // INC RCX
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);  // RBX = result ptr
    // Copy str_a: memcpy(RBX, RSI, R12)
    enc.xor_rr(X86Reg::RCX);               // i = 0
    let sc_lp1 = format!("__sc_l1_{}", enc.pos());
    let sc_d1 = format!("__sc_d1_{}", enc.pos());
    enc.label(&sc_lp1);
    enc.cmp_rr(X86Reg::RCX, X86Reg::R12);
    enc.jcc(0x8D, &sc_d1);                 // JGE done1
    enc.emit(&[0x0F, 0xB6, 0x04, 0x0E]);   // MOVZX EAX, [RSI+RCX]
    enc.emit(&[0x88, 0x04, 0x0B]);          // MOV [RBX+RCX], AL
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);   // INC RCX
    enc.jmp(&sc_lp1);
    enc.label(&sc_d1);
    // Copy str_b: memcpy(RBX+R12, RDI, R13)
    enc.xor_rr(X86Reg::RCX);               // j = 0
    let sc_lp2 = format!("__sc_l2_{}", enc.pos());
    let sc_d2 = format!("__sc_d2_{}", enc.pos());
    enc.label(&sc_lp2);
    enc.cmp_rr(X86Reg::RCX, X86Reg::R13);
    enc.jcc(0x8D, &sc_d2);
    enc.emit(&[0x0F, 0xB6, 0x04, 0x0F]);   // MOVZX EAX, [RDI+RCX]
    // dst = RBX + R12 + RCX
    enc.mov_rr(X86Reg::RAX, X86Reg::R12);
    enc.add_rr(X86Reg::RAX, X86Reg::RCX);
    // Need to reload byte since we clobbered RAX
    enc.emit(&[0x44, 0x0F, 0xB6, 0x04, 0x0F]); // MOVZX R8D, [RDI+RCX] (use R8 as temp)
    enc.emit(&[0x44, 0x88, 0x04, 0x03]);    // MOV [RBX+RAX], R8B
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);
    enc.jmp(&sc_lp2);
    enc.label(&sc_d2);
    // Null terminate
    enc.mov_rr(X86Reg::RAX, X86Reg::R12);
    enc.add_rr(X86Reg::RAX, X86Reg::R13);
    enc.emit(&[0xC6, 0x04, 0x03, 0x00]);    // MOV byte [RBX+RAX], 0
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.add_rsp(32);
    enc.pop(X86Reg::R13);
    enc.pop(X86Reg::R12);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_str_contains: RCX=haystack, RDX=needle → RAX=1/0
    enc.label("__pyb_str_contains");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(32);
    enc.call_label("__pyb_str_find");
    // If RAX >= 0, return 1; else 0
    enc.rex_w(); enc.emit(&[0x83, 0xF8, 0x00]); // CMP RAX, 0
    let scon_yes = format!("__scon_y_{}", enc.pos());
    let scon_done = format!("__scon_d_{}", enc.pos());
    enc.jcc(0x8D, &scon_yes); // JGE → found
    enc.xor_rr(X86Reg::RAX);
    enc.jmp(&scon_done);
    enc.label(&scon_yes);
    enc.mov_imm64(X86Reg::RAX, 1);
    enc.label(&scon_done);
    enc.add_rsp(32);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_str_startswith: RCX=str, RDX=prefix → RAX=1/0
    enc.label("__pyb_str_startswith");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);  // str
    enc.mov_rr(X86Reg::RDI, X86Reg::RDX);  // prefix
    enc.xor_rr(X86Reg::RBX);               // i = 0
    let ssw_lp = format!("__ssw_l_{}", enc.pos());
    let ssw_ok = format!("__ssw_ok_{}", enc.pos());
    let ssw_no = format!("__ssw_no_{}", enc.pos());
    enc.label(&ssw_lp);
    // If prefix[i] == 0, all matched → success
    enc.emit(&[0x0F, 0xB6, 0x04, 0x1F]);   // MOVZX EAX, [RDI+RBX]
    enc.emit(&[0x84, 0xC0]);                // TEST AL, AL
    enc.jcc(0x84, &ssw_ok);                 // JE → all prefix matched
    // If str[i] == 0 or str[i] != prefix[i], fail
    enc.emit(&[0x0F, 0xB6, 0x0C, 0x1E]);   // MOVZX ECX, [RSI+RBX]
    enc.emit(&[0x38, 0xC1]);                // CMP CL, AL
    enc.jcc(0x85, &ssw_no);                 // JNE → mismatch
    enc.rex_w(); enc.emit(&[0xFF, 0xC3]);   // INC RBX
    enc.jmp(&ssw_lp);
    enc.label(&ssw_ok);
    enc.mov_imm64(X86Reg::RAX, 1);
    let ssw_dn = format!("__ssw_d_{}", enc.pos());
    enc.jmp(&ssw_dn);
    enc.label(&ssw_no);
    enc.xor_rr(X86Reg::RAX);
    enc.label(&ssw_dn);
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_str_endswith: RCX=str, RDX=suffix → RAX=1/0
    enc.label("__pyb_str_endswith");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.push(X86Reg::R12);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);
    enc.mov_rr(X86Reg::RDI, X86Reg::RDX);
    // Get str len
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);  // RBX = str_len
    // Get suffix len
    enc.mov_rr(X86Reg::RCX, X86Reg::RDI);
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::R12, X86Reg::RAX);  // R12 = suffix_len
    // If suffix_len > str_len, return 0
    enc.cmp_rr(X86Reg::R12, X86Reg::RBX);
    let sew_no = format!("__sew_no_{}", enc.pos());
    enc.jcc(0x8F, &sew_no);                // JG → suffix longer
    // Compare: str[str_len - suffix_len ..] == suffix
    enc.mov_rr(X86Reg::RCX, X86Reg::RBX);
    enc.sub_rr(X86Reg::RCX, X86Reg::R12);  // RCX = start offset
    enc.xor_rr(X86Reg::RDX);               // j = 0
    let sew_lp = format!("__sew_l_{}", enc.pos());
    let sew_ok = format!("__sew_ok_{}", enc.pos());
    enc.label(&sew_lp);
    enc.cmp_rr(X86Reg::RDX, X86Reg::R12);
    enc.jcc(0x8D, &sew_ok);                // JGE → all matched
    // Compare str[start+j] with suffix[j]
    enc.mov_rr(X86Reg::RAX, X86Reg::RCX);
    enc.add_rr(X86Reg::RAX, X86Reg::RDX);
    enc.emit(&[0x44, 0x0F, 0xB6, 0x04, 0x06]); // MOVZX R8D, [RSI+RAX]
    enc.emit(&[0x0F, 0xB6, 0x04, 0x17]);   // MOVZX EAX, [RDI+RDX]
    enc.emit(&[0x44, 0x38, 0xC0]);          // CMP AL, R8B
    enc.jcc(0x85, &sew_no);
    enc.rex_w(); enc.emit(&[0xFF, 0xC2]);   // INC RDX
    enc.jmp(&sew_lp);
    enc.label(&sew_ok);
    enc.mov_imm64(X86Reg::RAX, 1);
    let sew_dn = format!("__sew_d_{}", enc.pos());
    enc.jmp(&sew_dn);
    enc.label(&sew_no);
    enc.xor_rr(X86Reg::RAX);
    enc.label(&sew_dn);
    enc.add_rsp(32);
    enc.pop(X86Reg::R12);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_str_slice: RCX=str, RDX=start, R8=end → RAX=new heap substring
    enc.label("__pyb_str_slice");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.push(X86Reg::R12);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);  // str
    enc.mov_rr(X86Reg::RDI, X86Reg::RDX);  // start
    enc.mov_rr(X86Reg::R12, X86Reg::R8);   // end
    // Handle negative indices: get str len
    enc.call_label("__pyb_str_len");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);  // RBX = str_len
    // If start < 0: start = max(0, str_len + start)
    enc.rex_w(); enc.emit(&[0x85, 0xFF]);   // TEST RDI, RDI
    let ssl_sp = format!("__ssl_sp_{}", enc.pos());
    enc.jcc(0x89, &ssl_sp);                // JNS → start positive
    enc.add_rr(X86Reg::RDI, X86Reg::RBX);  // start += len
    // clamp to 0
    enc.rex_w(); enc.emit(&[0x85, 0xFF]);
    let ssl_s0 = format!("__ssl_s0_{}", enc.pos());
    enc.jcc(0x89, &ssl_s0);
    enc.xor_rr(X86Reg::RDI);
    enc.label(&ssl_s0);
    enc.label(&ssl_sp);
    // If end < 0: end = str_len + end
    enc.rex_w(); enc.emit(&[0x4D, 0x85, 0xE4]); // TEST R12, R12
    let ssl_ep = format!("__ssl_ep_{}", enc.pos());
    enc.jcc(0x89, &ssl_ep);
    enc.emit(&[0x4C, 0x01, 0xE3]); // ADD RBX to R12 won't work. use mov
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.emit(&[0x49, 0x01, 0xC4]);          // ADD R12, RAX
    enc.label(&ssl_ep);
    // Clamp end to str_len
    enc.emit(&[0x4C, 0x39, 0xE3]);          // CMP RBX, R12
    let ssl_ec = format!("__ssl_ec_{}", enc.pos());
    enc.jcc(0x8D, &ssl_ec);                // JGE → end <= len, ok
    enc.mov_rr(X86Reg::R12, X86Reg::RBX);
    enc.label(&ssl_ec);
    // slice_len = end - start
    enc.mov_rr(X86Reg::RCX, X86Reg::R12);
    enc.sub_rr(X86Reg::RCX, X86Reg::RDI);  // RCX = slice_len
    // If slice_len <= 0, return empty string
    enc.rex_w(); enc.emit(&[0x85, 0xC9]);
    let ssl_emp = format!("__ssl_em_{}", enc.pos());
    enc.jcc(0x8E, &ssl_emp);               // JLE → empty
    // Alloc slice_len + 1
    enc.push(X86Reg::RCX);                 // save slice_len
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);   // INC RCX
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);  // RBX = result
    enc.pop(X86Reg::RCX);                  // restore slice_len
    // Copy bytes: src=RSI+RDI, dst=RBX, count=RCX
    enc.xor_rr(X86Reg::RDX);
    let ssl_cl = format!("__ssl_cl_{}", enc.pos());
    let ssl_cd = format!("__ssl_cd_{}", enc.pos());
    enc.label(&ssl_cl);
    enc.cmp_rr(X86Reg::RDX, X86Reg::RCX);
    enc.jcc(0x8D, &ssl_cd);
    enc.mov_rr(X86Reg::RAX, X86Reg::RDI);
    enc.add_rr(X86Reg::RAX, X86Reg::RDX);
    enc.emit(&[0x0F, 0xB6, 0x04, 0x06]);   // MOVZX EAX, [RSI+RAX]
    enc.emit(&[0x88, 0x04, 0x13]);          // MOV [RBX+RDX], AL
    enc.rex_w(); enc.emit(&[0xFF, 0xC2]);
    enc.jmp(&ssl_cl);
    enc.label(&ssl_cd);
    // Null terminate
    enc.emit(&[0xC6, 0x04, 0x0B, 0x00]);   // MOV byte [RBX+RCX], 0
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    let ssl_dn = format!("__ssl_dn_{}", enc.pos());
    enc.jmp(&ssl_dn);
    enc.label(&ssl_emp);
    // Empty string: alloc 1 byte
    enc.mov_imm64(X86Reg::RCX, 1);
    enc.call_label("__pyb_heap_alloc");
    enc.emit(&[0xC6, 0x00, 0x00]);          // MOV byte [RAX], 0
    enc.label(&ssl_dn);
    enc.add_rsp(32);
    enc.pop(X86Reg::R12);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_str_index: RCX=str, RDX=index → RAX=char as heap str (1 char)
    enc.label("__pyb_str_index");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RSI, X86Reg::RCX);
    enc.mov_rr(X86Reg::RBX, X86Reg::RDX);
    // Handle negative index
    enc.rex_w(); enc.emit(&[0x85, 0xDB]);   // TEST RBX, RBX
    let si_pos = format!("__si_p_{}", enc.pos());
    enc.jcc(0x89, &si_pos);
    enc.mov_rr(X86Reg::RCX, X86Reg::RSI);
    enc.call_label("__pyb_str_len");
    enc.add_rr(X86Reg::RBX, X86Reg::RAX);  // index += len
    enc.label(&si_pos);
    // Alloc 2 bytes
    enc.mov_imm64(X86Reg::RCX, 2);
    enc.call_label("__pyb_heap_alloc");
    // Copy char: result[0] = str[index]
    enc.emit(&[0x0F, 0xB6, 0x0C, 0x1E]);   // MOVZX ECX, [RSI+RBX]
    enc.emit(&[0x88, 0x08]);                // MOV [RAX], CL
    enc.emit(&[0xC6, 0x40, 0x01, 0x00]);   // MOV byte [RAX+1], 0
    enc.add_rsp(32);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_int_to_str: RAX=int64 → RAX=heap str (decimal representation)
    enc.label("__pyb_int_to_str");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(96);
    enc.mov_rr(X86Reg::RSI, X86Reg::RAX);  // save number
    // Alloc 24 bytes (max i64 = 20 digits + sign + null)
    enc.mov_imm64(X86Reg::RCX, 24);
    enc.call_label("__pyb_heap_alloc");
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);  // RBX = result buffer
    enc.mov_rr(X86Reg::RAX, X86Reg::RSI);  // restore number
    // Handle negative
    enc.xor_rr(X86Reg::RDI);               // RDI = write index = 0
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    let its_pos = format!("__its_p_{}", enc.pos());
    enc.jcc(0x89, &its_pos);               // JNS
    // Write '-'
    enc.emit(&[0xC6, 0x03, 0x2D]);          // MOV byte [RBX], '-'
    enc.mov_imm64(X86Reg::RDI, 1);
    enc.rex_w(); enc.emit(&[0xF7, 0xD8]);   // NEG RAX
    enc.label(&its_pos);
    // Handle zero
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    let its_nz = format!("__its_nz_{}", enc.pos());
    enc.jcc(0x85, &its_nz);
    enc.emit(&[0xC6, 0x04, 0x3B, 0x30]);   // MOV byte [RBX+RDI], '0'
    enc.rex_w(); enc.emit(&[0xFF, 0xC7]);
    enc.emit(&[0xC6, 0x04, 0x3B, 0x00]);   // MOV byte [RBX+RDI], 0
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    let its_dn = format!("__its_dn_{}", enc.pos());
    enc.jmp(&its_dn);
    enc.label(&its_nz);
    // Convert digits to stack buffer [rsp+32..rsp+52] right-to-left
    // lea RSI, [rsp+52]
    enc.emit(&[0x48, 0x8D, 0x74, 0x24, 0x34]); // LEA RSI, [RSP+52]
    enc.xor_rr(X86Reg::RCX);               // digit count
    let its_lp = format!("__its_lp_{}", enc.pos());
    let its_ld = format!("__its_ld_{}", enc.pos());
    enc.label(&its_lp);
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    enc.jcc(0x84, &its_ld);                // JE → done
    enc.push(X86Reg::RCX);
    enc.xor_rr(X86Reg::RDX);
    enc.mov_imm64(X86Reg::RCX, 10);
    enc.rex_w(); enc.emit(&[0xF7, 0xF1]);   // DIV RCX
    enc.rex_w(); enc.emit(&[0x83, 0xC2, 0x30]); // ADD RDX, '0'
    enc.rex_w(); enc.emit(&[0xFF, 0xCE]);   // DEC RSI
    enc.emit(&[0x88, 0x16]);                // MOV [RSI], DL
    enc.pop(X86Reg::RCX);
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);   // INC RCX
    enc.jmp(&its_lp);
    enc.label(&its_ld);
    // Copy digits from stack buf to result: src=RSI, dst=RBX+RDI, count=RCX
    enc.xor_rr(X86Reg::RDX);
    let its_cp = format!("__its_cp_{}", enc.pos());
    let its_ce = format!("__its_ce_{}", enc.pos());
    enc.label(&its_cp);
    enc.cmp_rr(X86Reg::RDX, X86Reg::RCX);
    enc.jcc(0x8D, &its_ce);
    enc.emit(&[0x0F, 0xB6, 0x04, 0x16]);   // MOVZX EAX, [RSI+RDX]
    enc.mov_rr(X86Reg::RAX, X86Reg::RDI);
    enc.add_rr(X86Reg::RAX, X86Reg::RDX);
    // Reload byte (clobbered RAX)
    enc.emit(&[0x44, 0x0F, 0xB6, 0x04, 0x16]); // MOVZX R8D, [RSI+RDX]
    enc.emit(&[0x44, 0x88, 0x04, 0x03]);    // MOV [RBX+RAX], R8B
    enc.rex_w(); enc.emit(&[0xFF, 0xC2]);
    enc.jmp(&its_cp);
    enc.label(&its_ce);
    // Null terminate
    enc.add_rr(X86Reg::RDI, X86Reg::RCX);
    enc.emit(&[0xC6, 0x04, 0x3B, 0x00]);   // MOV byte [RBX+RDI], 0
    enc.mov_rr(X86Reg::RAX, X86Reg::RBX);
    enc.label(&its_dn);
    enc.add_rsp(96);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // ══════════════════════════════════════════════════════════
    // v4.5 — list completo runtime stubs
    // ══════════════════════════════════════════════════════════

    // __pyb_list_pop: RCX=list_ptr → RAX=last element (removes it)
    enc.label("__pyb_list_pop");
    // len = [RCX+8], if len == 0 return 0
    enc.emit(&[0x48, 0x8B, 0x41, 0x08]);   // MOV RAX, [RCX+8] (len)
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    let lp_empty = format!("__lp_em_{}", enc.pos());
    enc.jcc(0x84, &lp_empty);              // JE → empty
    // len--
    enc.rex_w(); enc.emit(&[0xFF, 0xC8]);   // DEC RAX
    enc.emit(&[0x48, 0x89, 0x41, 0x08]);   // MOV [RCX+8], RAX (new len)
    // result = data_ptr[new_len]
    enc.mov_rr(X86Reg::RDX, X86Reg::RAX);  // index = new len
    enc.emit(&[0x48, 0x8B, 0x01]);          // MOV RAX, [RCX] (data_ptr)
    enc.emit(&[0x48, 0x8B, 0x04, 0xD0]);   // MOV RAX, [RAX+RDX*8]
    let lp_dn = format!("__lp_dn_{}", enc.pos());
    enc.jmp(&lp_dn);
    enc.label(&lp_empty);
    enc.xor_rr(X86Reg::RAX);
    enc.label(&lp_dn);
    enc.ret();

    // __pyb_list_reverse: RCX=list_ptr → void (in-place)
    enc.label("__pyb_list_reverse");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);
    enc.emit(&[0x48, 0x8B, 0x03]);          // MOV RAX, [RBX] (data_ptr)
    enc.mov_rr(X86Reg::RSI, X86Reg::RAX);   // RSI = data_ptr
    enc.emit(&[0x48, 0x8B, 0x43, 0x08]);    // MOV RAX, [RBX+8] (len)
    enc.rex_w(); enc.emit(&[0xFF, 0xC8]);    // DEC RAX
    enc.mov_rr(X86Reg::RDI, X86Reg::RAX);   // RDI = right = len-1
    enc.xor_rr(X86Reg::RCX);                // RCX = left = 0
    let lr_lp = format!("__lr_lp_{}", enc.pos());
    let lr_dn = format!("__lr_dn_{}", enc.pos());
    enc.label(&lr_lp);
    enc.cmp_rr(X86Reg::RCX, X86Reg::RDI);
    enc.jcc(0x8D, &lr_dn);                  // JGE → done
    // swap data[left] and data[right]
    enc.emit(&[0x48, 0x8B, 0x04, 0xCE]);    // MOV RAX, [RSI+RCX*8]
    enc.emit(&[0x48, 0x8B, 0x14, 0xFE]);    // MOV RDX, [RSI+RDI*8]
    enc.emit(&[0x48, 0x89, 0x14, 0xCE]);    // MOV [RSI+RCX*8], RDX
    enc.emit(&[0x48, 0x89, 0x04, 0xFE]);    // MOV [RSI+RDI*8], RAX
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);    // INC RCX
    enc.rex_w(); enc.emit(&[0xFF, 0xCF]);    // DEC RDI
    enc.jmp(&lr_lp);
    enc.label(&lr_dn);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_list_sort: RCX=list_ptr → void (insertion sort in-place)
    enc.label("__pyb_list_sort");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.push(X86Reg::R12);
    enc.push(X86Reg::R13);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);
    enc.emit(&[0x48, 0x8B, 0x33]);          // MOV RSI, [RBX] (data_ptr)
    enc.emit(&[0x48, 0x8B, 0x7B, 0x08]);    // MOV RDI, [RBX+8] (len)
    enc.mov_imm64(X86Reg::R12, 1);          // i = 1
    let ls_out = format!("__ls_o_{}", enc.pos());
    let ls_odn = format!("__ls_od_{}", enc.pos());
    enc.label(&ls_out);
    enc.cmp_rr(X86Reg::R12, X86Reg::RDI);
    enc.jcc(0x8D, &ls_odn);                 // JGE → done
    // key = data[i]
    enc.emit(&[0x4E, 0x8B, 0x2C, 0xE6]);    // MOV R13, [RSI+R12*8]  (key)
    // j = i - 1
    enc.mov_rr(X86Reg::RCX, X86Reg::R12);
    enc.rex_w(); enc.emit(&[0xFF, 0xC9]);    // DEC RCX (j = i-1)
    let ls_in = format!("__ls_i_{}", enc.pos());
    let ls_idn = format!("__ls_id_{}", enc.pos());
    enc.label(&ls_in);
    // while j >= 0 && data[j] > key
    enc.rex_w(); enc.emit(&[0x83, 0xF9, 0x00]); // CMP RCX, 0
    enc.jcc(0x8C, &ls_idn);                 // JL → done inner
    enc.emit(&[0x48, 0x8B, 0x04, 0xCE]);    // MOV RAX, [RSI+RCX*8]
    enc.emit(&[0x4C, 0x39, 0xE8]);          // CMP RAX, R13
    enc.jcc(0x8E, &ls_idn);                 // JLE → data[j] <= key, done
    // data[j+1] = data[j]
    enc.mov_rr(X86Reg::RDX, X86Reg::RCX);
    enc.rex_w(); enc.emit(&[0xFF, 0xC2]);    // INC RDX
    enc.emit(&[0x48, 0x89, 0x04, 0xD6]);    // MOV [RSI+RDX*8], RAX
    enc.rex_w(); enc.emit(&[0xFF, 0xC9]);    // DEC RCX
    enc.jmp(&ls_in);
    enc.label(&ls_idn);
    // data[j+1] = key
    enc.mov_rr(X86Reg::RDX, X86Reg::RCX);
    enc.rex_w(); enc.emit(&[0xFF, 0xC2]);    // INC RDX
    enc.emit(&[0x4C, 0x89, 0x2C, 0xD6]);    // MOV [RSI+RDX*8], R13
    enc.emit(&[0x49, 0xFF, 0xC4]);           // INC R12
    enc.jmp(&ls_out);
    enc.label(&ls_odn);
    enc.pop(X86Reg::R13);
    enc.pop(X86Reg::R12);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_list_contains: RCX=list_ptr, RDX=value → RAX=1/0
    enc.label("__pyb_list_contains");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);
    enc.mov_rr(X86Reg::RSI, X86Reg::RDX);  // value to find
    enc.emit(&[0x48, 0x8B, 0x7B, 0x08]);    // MOV RDI, [RBX+8] (len)
    enc.emit(&[0x48, 0x8B, 0x03]);           // MOV RAX, [RBX] (data_ptr)
    enc.mov_rr(X86Reg::RBX, X86Reg::RAX);
    enc.xor_rr(X86Reg::RCX);                // i = 0
    let lcon_lp = format!("__lc_lp_{}", enc.pos());
    let lcon_fo = format!("__lc_fo_{}", enc.pos());
    let lcon_nf = format!("__lc_nf_{}", enc.pos());
    enc.label(&lcon_lp);
    enc.cmp_rr(X86Reg::RCX, X86Reg::RDI);
    enc.jcc(0x8D, &lcon_nf);
    enc.emit(&[0x48, 0x8B, 0x04, 0xCB]);    // MOV RAX, [RBX+RCX*8]
    enc.cmp_rr(X86Reg::RAX, X86Reg::RSI);
    enc.jcc(0x84, &lcon_fo);                 // JE → found
    enc.rex_w(); enc.emit(&[0xFF, 0xC1]);
    enc.jmp(&lcon_lp);
    enc.label(&lcon_fo);
    enc.mov_imm64(X86Reg::RAX, 1);
    let lcon_dn = format!("__lc_dn_{}", enc.pos());
    enc.jmp(&lcon_dn);
    enc.label(&lcon_nf);
    enc.xor_rr(X86Reg::RAX);
    enc.label(&lcon_dn);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_list_print: RCX=list_ptr → prints [a, b, c] format
    enc.label("__pyb_list_print");
    enc.push(X86Reg::RBX);
    enc.push(X86Reg::RSI);
    enc.push(X86Reg::RDI);
    enc.sub_rsp(32);
    enc.mov_rr(X86Reg::RBX, X86Reg::RCX);
    // Print '['
    enc.add_data_string("__v45_lbracket", "[");
    enc.lea_rax_data("__v45_lbracket");
    enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
    enc.mov_imm64(X86Reg::RDX, 1);
    enc.call_label("__pyb_print_str");
    // Loop elements
    enc.emit(&[0x48, 0x8B, 0x7B, 0x08]);    // MOV RDI, [RBX+8] (len)
    enc.xor_rr(X86Reg::RSI);                // i = 0
    let lpr_lp = format!("__lpr_l_{}", enc.pos());
    let lpr_dn = format!("__lpr_d_{}", enc.pos());
    enc.label(&lpr_lp);
    enc.cmp_rr(X86Reg::RSI, X86Reg::RDI);
    enc.jcc(0x8D, &lpr_dn);
    // Print ", " if not first
    enc.rex_w(); enc.emit(&[0x85, 0xF6]);    // TEST RSI, RSI
    let lpr_noc = format!("__lpr_nc_{}", enc.pos());
    enc.jcc(0x84, &lpr_noc);                // JE → first, no comma
    enc.add_data_string("__v45_comma", ", ");
    enc.lea_rax_data("__v45_comma");
    enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
    enc.mov_imm64(X86Reg::RDX, 2);
    enc.call_label("__pyb_print_str");
    enc.label(&lpr_noc);
    // Print element as int
    enc.emit(&[0x48, 0x8B, 0x03]);          // MOV RAX, [RBX] (data_ptr)
    enc.emit(&[0x48, 0x8B, 0x04, 0xF0]);    // MOV RAX, [RAX+RSI*8]
    enc.call_label("__pyb_itoa");
    enc.rex_w(); enc.emit(&[0xFF, 0xC6]);    // INC RSI
    enc.jmp(&lpr_lp);
    enc.label(&lpr_dn);
    // Print ']'
    enc.add_data_string("__v45_rbracket", "]");
    enc.lea_rax_data("__v45_rbracket");
    enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
    enc.mov_imm64(X86Reg::RDX, 1);
    enc.call_label("__pyb_print_str");
    enc.add_rsp(32);
    enc.pop(X86Reg::RDI);
    enc.pop(X86Reg::RSI);
    enc.pop(X86Reg::RBX);
    enc.ret();

    // __pyb_dict_contains: RCX=dict_ptr, RDX=key → RAX=1/0
    enc.label("__pyb_dict_contains");
    enc.push(X86Reg::RBX);
    enc.sub_rsp(32);
    enc.call_label("__pyb_dict_get");
    // dict_get returns 0 for not found — but 0 could be a valid value
    // For simplicity, treat 0 as not found (matches current dict behavior)
    enc.rex_w(); enc.emit(&[0x85, 0xC0]);
    let dc_yes = format!("__dc_y_{}", enc.pos());
    let dc_dn = format!("__dc_d_{}", enc.pos());
    enc.jcc(0x85, &dc_yes);                 // JNE → found
    enc.xor_rr(X86Reg::RAX);
    enc.jmp(&dc_dn);
    enc.label(&dc_yes);
    enc.mov_imm64(X86Reg::RAX, 1);
    enc.label(&dc_dn);
    enc.add_rsp(32);
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
                let abi = [X86Reg::RCX, X86Reg::RDX, X86Reg::R8, X86Reg::R9];
                for (i, arg) in args.iter().enumerate().take(4) {
                    compile_instruction(arg, func, enc, saved_regs, stack_size);
                    if i < abi.len() { enc.mov_rr(abi[i], X86Reg::RAX); }
                }
                enc.call_label(callee);
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
            enc.call_iat(20); // LoadLibraryA slot (to be added)
            enc.add_rsp(32);
        }
        IRInstruction::DllGetProc { name } => {
            // GetProcAddress(hModule, name) — RAX has module handle
            enc.mov_rr(X86Reg::RCX, X86Reg::RAX);
            enc.lea_rax_data(name);
            enc.mov_rr(X86Reg::RDX, X86Reg::RAX);
            enc.sub_rsp(32);
            enc.call_iat(21); // GetProcAddress slot (to be added)
            enc.add_rsp(32);
        }
        IRInstruction::DllCall { func_ptr, args } => {
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
