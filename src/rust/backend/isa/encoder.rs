use super::types::*;

use super::compiler::*;
use super::stubs::*;
use super::instructions::*;
use crate::middle::ir::*;
use crate::backend::reg_alloc::*;
use std::collections::HashMap;

pub struct Encoder {
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    pub data_labels: Vec<(String, u32)>,
    label_offsets: Vec<(String, u32)>,
    fixups: Vec<(usize, String)>,
    pub iat_fixups: Vec<(u32, usize)>,
    pub data_fixups: Vec<(u32, String)>,
    pub stats: ISAStats,
}

impl Encoder {
    pub fn new() -> Self {
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

    pub fn pos(&self) -> u32 { self.code.len() as u32 }

    pub fn emit(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
        self.stats.instructions_emitted += 1;
    }

    pub fn emit_u8(&mut self, b: u8) { self.code.push(b); }

    pub fn emit_u32_le(&mut self, v: u32) { self.code.extend_from_slice(&v.to_le_bytes()); }

    pub fn emit_i32_le(&mut self, v: i32) { self.code.extend_from_slice(&v.to_le_bytes()); }

    pub fn emit_u64_le(&mut self, v: u64) { self.code.extend_from_slice(&v.to_le_bytes()); }

    // REX.W prefix (no extended regs)
    pub fn rex_w(&mut self) { self.emit_u8(0x48); }

    // REX.W with R and B bits for extended registers
    // For instructions like MOV r/m64, r64:  REX.R = src>=8, REX.B = dst>=8
    // For instructions like MOV r64, r/m64:  REX.R = dst>=8, REX.B = src>=8
    pub fn rex_wrb(&mut self, reg: X86Reg, rm: X86Reg) {
        let mut rex: u8 = 0x48; // REX.W
        if reg.encoding() >= 8 { rex |= 0x04; } // REX.R
        if rm.encoding() >= 8 { rex |= 0x01; }  // REX.B
        self.emit_u8(rex);
    }

    pub fn rex_wb(&mut self, rm: X86Reg) {
        let mut rex: u8 = 0x48;
        if rm.encoding() >= 8 { rex |= 0x01; }
        self.emit_u8(rex);
    }

    // MOV reg, imm64
    pub fn mov_imm64(&mut self, reg: X86Reg, val: i64) {
        let r = reg.encoding();
        if r >= 8 { self.emit_u8(0x49); } else { self.emit_u8(0x48); }
        self.emit_u8(0xB8 + (r & 7));
        self.emit_u64_le(val as u64);
        self.stats.instructions_emitted += 1;
    }

    // MOV r/m64, r64  (opcode 0x89: src=reg field, dst=r/m field)
    pub fn mov_rr(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_wrb(src, dst);
        self.emit(&[0x89, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    pub fn add_rr(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_wrb(src, dst);
        self.emit(&[0x01, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    pub fn sub_rr(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_wrb(src, dst);
        self.emit(&[0x29, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    pub fn imul_rr(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_wrb(dst, src); // 0F AF: reg=dst, r/m=src
        self.emit(&[0x0F, 0xAF, 0xC0 | ((dst.encoding() & 7) << 3) | (src.encoding() & 7)]);
    }

    pub fn idiv_r(&mut self, src: X86Reg) {
        self.rex_w(); self.emit_u8(0x99); // CQO
        self.rex_wb(src); self.emit(&[0xF7, 0xF8 | (src.encoding() & 7)]);
    }

    pub fn cmp_rr(&mut self, a: X86Reg, b: X86Reg) {
        self.rex_wrb(b, a); // CMP r/m64, r64: reg=b, r/m=a
        self.emit(&[0x39, 0xC0 | ((b.encoding() & 7) << 3) | (a.encoding() & 7)]);
    }

    pub fn xor_rr(&mut self, reg: X86Reg) {
        self.rex_wrb(reg, reg);
        let r = reg.encoding() & 7;
        self.emit(&[0x31, 0xC0 | (r << 3) | r]);
    }

    pub fn push(&mut self, reg: X86Reg) {
        if reg.needs_rex() { self.emit_u8(0x41); }
        self.emit_u8(0x50 + (reg.encoding() & 7));
    }

    pub fn pop(&mut self, reg: X86Reg) {
        if reg.needs_rex() { self.emit_u8(0x41); }
        self.emit_u8(0x58 + (reg.encoding() & 7));
    }

    pub fn sub_rsp(&mut self, val: u8) { self.rex_w(); self.emit(&[0x83, 0xEC, val]); }
    pub fn add_rsp(&mut self, val: u8) { self.rex_w(); self.emit(&[0x83, 0xC4, val]); }
    pub fn ret(&mut self) { self.emit_u8(0xC3); }

    pub fn label(&mut self, name: &str) {
        self.label_offsets.push((name.to_string(), self.pos()));
    }

    pub fn jmp(&mut self, lbl: &str) {
        self.emit_u8(0xE9);
        self.fixups.push((self.code.len(), lbl.to_string()));
        self.emit_u32_le(0);
    }

    pub fn jcc(&mut self, cc: u8, lbl: &str) {
        self.emit(&[0x0F, cc]);
        self.fixups.push((self.code.len(), lbl.to_string()));
        self.emit_u32_le(0);
    }

    pub fn call_label(&mut self, lbl: &str) {
        self.emit_u8(0xE8);
        self.fixups.push((self.code.len(), lbl.to_string()));
        self.emit_u32_le(0);
    }

    // CALL [RIP+disp32] — indirect call through IAT
    pub fn call_iat(&mut self, slot: usize) {
        // FF 15 xx xx xx xx = CALL [RIP+disp32]
        self.emit(&[0xFF, 0x15]);
        let fixup_pos = self.pos();
        self.emit_u32_le(0); // placeholder — output.rs patches this
        self.iat_fixups.push((fixup_pos, slot));
    }

    // LEA RAX, [RIP+disp32] — load data address
    pub fn lea_rax_data(&mut self, data_label: &str) {
        // 48 8D 05 xx xx xx xx = LEA RAX, [RIP+disp32]
        self.emit(&[0x48, 0x8D, 0x05]);
        let fixup_pos = self.pos();
        self.emit_u32_le(0);
        self.data_fixups.push((fixup_pos, data_label.to_string()));
    }

    // Ensure a data label exists (for globals) — 8-byte slot initialized to value
    pub fn ensure_data_label(&mut self, label: &str, init_val: i64) {
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

    pub fn add_data_string(&mut self, label: &str, s: &str) {
        let offset = self.data.len() as u32;
        self.data_labels.push((label.to_string(), offset));
        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0);
    }

    pub fn add_data_f64(&mut self, label: &str, val: f64) {
        // Align to 8 bytes
        while self.data.len() % 8 != 0 { self.data.push(0); }
        let offset = self.data.len() as u32;
        self.data_labels.push((label.to_string(), offset));
        self.data.extend_from_slice(&val.to_le_bytes());
    }

    // ── SSE2 float instructions ─────────────────────────────
    // MOVSD XMM, [RIP+disp32]  — load f64 from data
    pub fn movsd_xmm_data(&mut self, xmm: u8, data_label: &str) {
        // F2 0F 10 /r = MOVSD xmm1, xmm2/m64
        // ModRM: 00 reg 101 = RIP+disp32
        self.emit(&[0xF2, 0x0F, 0x10, 0x05 | (xmm << 3)]);
        let fixup_pos = self.pos();
        self.emit_u32_le(0);
        self.data_fixups.push((fixup_pos, data_label.to_string()));
    }

    // MOVSD xmm1, xmm2
    pub fn movsd_xmm_xmm(&mut self, dst: u8, src: u8) {
        // F2 0F 10 /r (reg-reg: mod=11)
        self.emit(&[0xF2, 0x0F, 0x10, 0xC0 | (dst << 3) | src]);
    }

    // ADDSD xmm1, xmm2
    pub fn addsd(&mut self, dst: u8, src: u8) {
        self.emit(&[0xF2, 0x0F, 0x58, 0xC0 | (dst << 3) | src]);
    }
    // SUBSD xmm1, xmm2
    pub fn subsd(&mut self, dst: u8, src: u8) {
        self.emit(&[0xF2, 0x0F, 0x5C, 0xC0 | (dst << 3) | src]);
    }
    // MULSD xmm1, xmm2
    pub fn mulsd(&mut self, dst: u8, src: u8) {
        self.emit(&[0xF2, 0x0F, 0x59, 0xC0 | (dst << 3) | src]);
    }
    // DIVSD xmm1, xmm2
    pub fn divsd(&mut self, dst: u8, src: u8) {
        self.emit(&[0xF2, 0x0F, 0x5E, 0xC0 | (dst << 3) | src]);
    }
    // CVTTSD2SI reg64, xmm  (truncate f64 → i64)
    pub fn cvttsd2si(&mut self, dst: X86Reg, xmm: u8) {
        // F2 REX.W 0F 2C /r
        let mut rex: u8 = 0x48;
        if dst.encoding() >= 8 { rex |= 0x04; }
        self.emit(&[0xF2, rex, 0x0F, 0x2C, 0xC0 | ((dst.encoding() & 7) << 3) | xmm]);
    }
    // CVTSI2SD xmm, reg64  (i64 → f64)
    pub fn cvtsi2sd(&mut self, xmm: u8, src: X86Reg) {
        // F2 REX.W 0F 2A /r
        let mut rex: u8 = 0x48;
        if src.encoding() >= 8 { rex |= 0x01; }
        self.emit(&[0xF2, rex, 0x0F, 0x2A, 0xC0 | (xmm << 3) | (src.encoding() & 7)]);
    }
    // MOVQ RAX, XMM0 (move 64 bits from xmm to gpr)
    pub fn movq_rax_xmm0(&mut self) {
        // 66 48 0F 7E C0 = MOVQ RAX, XMM0
        self.emit(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]);
    }
    // MOVQ XMM0, RAX
    pub fn movq_xmm0_rax(&mut self) {
        // 66 48 0F 6E C0 = MOVQ XMM0, RAX
        self.emit(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]);
    }
    // XORPD xmm, xmm (zero a float register)
    pub fn xorpd(&mut self, xmm: u8) {
        self.emit(&[0x66, 0x0F, 0x57, 0xC0 | (xmm << 3) | xmm]);
    }
    // UCOMISD xmm1, xmm2
    pub fn ucomisd(&mut self, a: u8, b: u8) {
        self.emit(&[0x66, 0x0F, 0x2E, 0xC0 | (a << 3) | b]);
    }
    // MULSD xmm, [RIP+disp32]
    pub fn mulsd_data(&mut self, xmm: u8, label: &str) {
        self.emit(&[0xF2, 0x0F, 0x59, 0x05 | (xmm << 3)]);
        let fixup_pos = self.pos();
        self.emit_u32_le(0);
        self.data_fixups.push((fixup_pos, label.to_string()));
    }

    pub fn resolve_label_fixups(&mut self) {
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
