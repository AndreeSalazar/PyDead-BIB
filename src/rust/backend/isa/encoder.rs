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
pub const IAT_LOAD_LIBRARY: usize = 20;
pub const IAT_GET_PROC_ADDRESS: usize = 21;
pub const IAT_FREE_LIBRARY: usize = 22;
pub const IAT_SLOT_COUNT: usize = 23;

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
pub(crate) struct Encoder {
    pub(crate) code: Vec<u8>,
    pub(crate) data: Vec<u8>,
    pub(crate) data_labels: Vec<(String, u32)>,
    pub(crate) label_offsets: Vec<(String, u32)>,
    pub(crate) fixups: Vec<(usize, String)>,
    pub(crate) iat_fixups: Vec<(u32, usize)>,
    pub(crate) data_fixups: Vec<(u32, String)>,
    pub(crate) stats: ISAStats,
}

impl Encoder {
    fn new() -> Self {
        Self {
            code: Vec::new(),
            data: Vec::new(),
            pub(crate) data_labels: Vec::new(),
            pub(crate) label_offsets: Vec::new(),
            pub(crate) fixups: Vec::new(),
            pub(crate) iat_fixups: Vec::new(),
            pub(crate) data_fixups: Vec::new(),
            pub(crate) stats: ISAStats::default(),
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

}
