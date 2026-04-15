use super::encoder::Encoder;
use crate::backend::reg_alloc::X86Reg;

impl Encoder {
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
