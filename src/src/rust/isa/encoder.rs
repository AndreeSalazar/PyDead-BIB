// ============================================================
// ADead-BIB ISA Encoder — ADeadOp → bytes x86-64
// ============================================================
// Convierte instrucciones tipadas (ADeadOp) en bytes de máquina
// exactamente iguales a los que codegen_v2.rs emitía directamente.
//
// Pipeline: AST → ADeadIR → Encoder → bytes → PE/ELF
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use super::*;
use std::collections::HashMap;

/// Resultado de la codificación de un programa completo.
#[derive(Debug, Clone)]
pub struct EncodeResult {
    /// Bytes de código x86-64 generados
    pub code: Vec<u8>,
    /// Llamadas a funciones no resueltas: (offset_en_code, nombre_función)
    pub unresolved_calls: Vec<(usize, String)>,
    /// FASM-inspired: exact offsets of IAT call disp32 fields (FF 15 [disp32])
    /// Each entry is the code offset of the 4-byte disp32 field
    pub iat_call_offsets: Vec<usize>,
    /// FASM-inspired: exact offsets of 64-bit string address immediates (48 B8+ [imm64])
    /// Each entry is the code offset of the 8-byte imm64 field
    pub string_imm64_offsets: Vec<usize>,
}

/// Tipo de patch pendiente para resolución de saltos.
#[derive(Debug, Clone)]
struct PendingPatch {
    /// Offset en el buffer de código donde escribir el desplazamiento
    code_offset: usize,
    /// Label destino del salto
    target: Label,
    /// Tipo de salto (rel8 o rel32)
    kind: PatchKind,
    /// Index into the ops[] array that generated this patch
    op_idx: usize,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum PatchKind {
    Rel32,
    Rel8,
}

/// Encoder de instrucciones ISA a bytes x86-64.
pub struct Encoder {
    code: Vec<u8>,
    label_positions: HashMap<u32, usize>,
    pending_patches: Vec<PendingPatch>,
    unresolved_calls: Vec<(usize, String)>,
    iat_call_offsets: Vec<usize>,
    string_imm64_offsets: Vec<usize>,
    current_op_idx: usize,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            label_positions: HashMap::new(),
            pending_patches: Vec::new(),
            unresolved_calls: Vec::new(),
            iat_call_offsets: Vec::new(),
            string_imm64_offsets: Vec::new(),
            current_op_idx: 0,
        }
    }

    /// FASM-inspired multi-pass encoder (from assembler_loop in ASSEMBLE.INC).
    ///
    /// Pass 1: Encode all ops with rel32 jumps, record label positions.
    /// Pass 2: Patch all jump displacement fields.
    /// Pass 3: Verify short-jump candidates — only backward jumps whose
    ///         target label was already seen (no forward-reference risk).
    ///         Re-encode if any can be shortened; repeat until stable.
    ///
    /// Conservative approach: only shorten backward jumps where the label
    /// position is definitively known before the jump instruction.
    pub fn encode_all(&mut self, ops: &[ADeadOp]) -> EncodeResult {
        // Track which (jump_index, target_label) can use short encoding.
        // Key = index into ops[] of the Jmp/Jcc, value = true if short-safe.
        let mut short_jump_indices: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
        let max_passes = 3;

        for _pass in 0..max_passes {
            self.code.clear();
            self.label_positions.clear();
            self.pending_patches.clear();
            self.unresolved_calls.clear();
            self.iat_call_offsets.clear();
            self.string_imm64_offsets.clear();

            for (op_idx, op) in ops.iter().enumerate() {
                if short_jump_indices.contains(&op_idx) {
                    // Emit short form for this jump
                    match op {
                        ADeadOp::Jmp { target } => {
                            self.emit(&[0xEB]);
                            let patch_offset = self.code.len();
                            self.emit(&[0x00]);
                            self.pending_patches.push(PendingPatch {
                                code_offset: patch_offset,
                                target: *target,
                                kind: PatchKind::Rel8,
                                op_idx,
                            });
                            continue;
                        }
                        ADeadOp::Jcc { cond, target } => {
                            let cc = match cond {
                                Condition::Equal => 0x04u8,
                                Condition::NotEqual => 0x05,
                                Condition::Less => 0x0C,
                                Condition::LessEq => 0x0E,
                                Condition::Greater => 0x0F,
                                Condition::GreaterEq => 0x0D,
                                Condition::Always => {
                                    self.emit(&[0xEB]);
                                    let patch_offset = self.code.len();
                                    self.emit(&[0x00]);
                                    self.pending_patches.push(PendingPatch {
                                        code_offset: patch_offset,
                                        target: *target,
                                        kind: PatchKind::Rel8,
                                        op_idx,
                                    });
                                    continue;
                                }
                            };
                            self.emit(&[0x70 | cc]);
                            let patch_offset = self.code.len();
                            self.emit(&[0x00]);
                            self.pending_patches.push(PendingPatch {
                                code_offset: patch_offset,
                                target: *target,
                                kind: PatchKind::Rel8,
                                op_idx,
                            });
                            continue;
                        }
                        _ => {}
                    }
                }
                self.current_op_idx = op_idx;
                self.encode_op(op);
            }

            // Resolve all patches
            let mut unresolved_patch_count = 0usize;
            for patch in &self.pending_patches {
                if let Some(&target_pos) = self.label_positions.get(&patch.target.0) {
                    match patch.kind {
                        PatchKind::Rel32 => {
                            let rel = (target_pos as i64 - (patch.code_offset as i64 + 4)) as i32;
                            self.code[patch.code_offset..patch.code_offset + 4]
                                .copy_from_slice(&rel.to_le_bytes());
                        }
                        PatchKind::Rel8 => {
                            let rel = target_pos as i64 - (patch.code_offset as i64 + 1);
                            // Safety: verify it still fits; if not, we have a bug
                            self.code[patch.code_offset] = (rel as i8) as u8;
                        }
                    }
                } else {
                    unresolved_patch_count += 1;
                }
            }
            if unresolved_patch_count > 0 {
                eprintln!(
                    "   ⚠️  Encoder: {} unresolved label patches ({} labels known, {} patches total)",
                    unresolved_patch_count,
                    self.label_positions.len(),
                    self.pending_patches.len()
                );
            }

            // Convergence: find backward rel32 jumps that fit in rel8
            let mut changed = false;
            for patch in self.pending_patches.iter() {
                if let PatchKind::Rel32 = patch.kind {
                    if let Some(&target_pos) = self.label_positions.get(&patch.target.0) {
                        // Only shorten BACKWARD jumps (target before jump)
                        if target_pos < patch.code_offset {
                            let rel = target_pos as i64 - (patch.code_offset as i64 + 4);
                            // Conservative: only if well within range after shrink
                            if rel >= -120 && rel <= 120 {
                                if short_jump_indices.insert(patch.op_idx) {
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }

            if !changed {
                break;
            }
        }

        EncodeResult {
            code: self.code.clone(),
            unresolved_calls: self.unresolved_calls.clone(),
            iat_call_offsets: self.iat_call_offsets.clone(),
            string_imm64_offsets: self.string_imm64_offsets.clone(),
        }
    }

    /// Codifica una instrucción individual.
    pub fn encode_op(&mut self, op: &ADeadOp) {
        match op {
            ADeadOp::Mov { dst, src } => self.encode_mov(dst, src),
            ADeadOp::Store16 { base, disp, src } => self.encode_store16(base, *disp, src),
            ADeadOp::Store32 { base, disp, src } => self.encode_store32(base, *disp, src),
            ADeadOp::MovZx { dst, src } => self.encode_movzx(dst, src),
            ADeadOp::Lea { dst, src } => self.encode_lea(dst, src),
            ADeadOp::Add { dst, src } => self.encode_add(dst, src),
            ADeadOp::Sub { dst, src } => self.encode_sub(dst, src),
            ADeadOp::Mul { dst, src } => self.encode_mul(dst, src),
            ADeadOp::Div { src } => self.encode_div(src),
            ADeadOp::And { dst, src } => self.encode_and(dst, src),
            ADeadOp::Or { dst, src } => self.encode_or(dst, src),
            ADeadOp::Xor { dst, src } => self.encode_xor(dst, src),
            ADeadOp::Inc { dst } => self.encode_inc(dst),
            ADeadOp::Dec { dst } => self.encode_dec(dst),
            ADeadOp::Neg { dst } => self.encode_neg(dst),
            ADeadOp::Not { dst } => self.encode_not(dst),
            ADeadOp::Shl { dst, amount } => self.encode_shl(dst, *amount),
            ADeadOp::Cmp { left, right } => self.encode_cmp(left, right),
            ADeadOp::Test { left, right } => self.encode_test(left, right),
            ADeadOp::SetCC { cond, dst: _ } => self.encode_setcc(cond),
            ADeadOp::Push { src } => self.encode_push(src),
            ADeadOp::Pop { dst } => self.encode_pop(dst),
            ADeadOp::Call { target } => self.encode_call(target),
            ADeadOp::Jmp { target } => self.encode_jmp(target),
            ADeadOp::Jcc { cond, target } => self.encode_jcc(cond, target),
            ADeadOp::Ret => self.emit(&[0xC3]),
            ADeadOp::Syscall => self.emit(&[0x0F, 0x05]),
            ADeadOp::CvtSi2Sd { dst, src } => {
                let (src_idx, src_ext) = reg_index(src);
                let (dst_idx, _) = reg_index(dst);
                let rex = 0x48 | if src_ext { 0x01 } else { 0x00 };
                let modrm = 0xC0 | (dst_idx << 3) | src_idx;
                self.emit(&[0xF2, rex, 0x0F, 0x2A, modrm]);
            }
            ADeadOp::MovQ { dst, src } => self.encode_movq(dst, src),
            ADeadOp::Label(label) => {
                self.label_positions.insert(label.0, self.code.len());
            }
            ADeadOp::Nop => self.emit(&[0x90]),
            ADeadOp::RawBytes(bytes) => self.emit(bytes),
            ADeadOp::CallIAT { iat_rva } => self.encode_call_iat(*iat_rva),

            // ================================================================
            // OS-Level / Privileged Instructions
            // ================================================================
            ADeadOp::Cli => self.emit(&[0xFA]),
            ADeadOp::Sti => self.emit(&[0xFB]),
            ADeadOp::Hlt => self.emit(&[0xF4]),
            ADeadOp::Iret => self.emit(&[0x48, 0xCF]), // iretq (REX.W + IRET)
            ADeadOp::Int { vector } => {
                self.emit(&[0xCD, *vector]);
            }
            ADeadOp::Lgdt { src } => self.encode_lgdt(src),
            ADeadOp::Lidt { src } => self.encode_lidt(src),
            ADeadOp::MovToCr { cr, src } => self.encode_mov_to_cr(*cr, src),
            ADeadOp::MovFromCr { cr, dst } => self.encode_mov_from_cr(*cr, dst),
            ADeadOp::Cpuid => self.emit(&[0x0F, 0xA2]),
            ADeadOp::Rdmsr => self.emit(&[0x0F, 0x32]),
            ADeadOp::Wrmsr => self.emit(&[0x0F, 0x30]),
            ADeadOp::Invlpg { addr } => self.encode_invlpg(addr),
            ADeadOp::InByte { port } => self.encode_in_byte(port),
            ADeadOp::OutByte { port, src: _ } => self.encode_out_byte(port),
            ADeadOp::InDword { port } => self.encode_in_dword(port),
            ADeadOp::OutDword { port, src: _ } => self.encode_out_dword(port),
            ADeadOp::Shr { dst, amount } => self.encode_shr(dst, *amount),
            ADeadOp::BitwiseNot { dst } => self.encode_bitwise_not(dst),
            ADeadOp::ShlCl { dst } => self.encode_shl_cl(dst),
            ADeadOp::ShrCl { dst } => self.encode_shr_cl(dst),
            ADeadOp::LeaLabel { dst, label } => {
                // LEA reg, [rip + disp32] — load function address into register
                let (dst_idx, dst_ext) = reg_index(dst);
                let rex = 0x48 | if dst_ext { 0x04 } else { 0x00 }; // REX.W + REX.R if needed
                let modrm = (dst_idx << 3) | 0x05; // mod=00, rm=101 (RIP-relative)
                self.emit(&[rex, 0x8D, modrm]);
                let patch_offset = self.code.len();
                self.emit_i32(0);
                self.pending_patches.push(PendingPatch {
                    code_offset: patch_offset,
                    target: *label,
                    kind: PatchKind::Rel32,
                    op_idx: self.current_op_idx,
                });
            }
            ADeadOp::FarJmp { selector, offset } => self.encode_far_jmp(*selector, *offset),
            ADeadOp::LabelAddrRef {
                label,
                size,
                base_addr,
            } => {
                // Emit the absolute address of a label
                // This requires the label to be already defined (resolved in second pass)
                if let Some(&pos) = self.label_positions.get(&label.0) {
                    let addr = *base_addr as usize + pos;
                    match size {
                        2 => self.emit_u16(addr as u16),
                        4 => self.emit_u32(addr as u32),
                        _ => self.emit_u32(addr as u32),
                    }
                } else {
                    // Label not yet defined - emit placeholder and record for later resolution
                    // For now, emit zeros as placeholder
                    match size {
                        2 => self.emit_u16(0),
                        4 => self.emit_u32(0),
                        _ => self.emit_u32(0),
                    }
                }
            }
        }
    }

    // ========================================
    // MOV
    // ========================================

    /// FASM-inspired generic MOV encoder — supports ALL register combinations.
    /// Replaces ~120 lines of per-register match arms with computed encoding.
    fn encode_mov(&mut self, dst: &Operand, src: &Operand) {
        match (dst, src) {
            // mov r64, imm64 — FASM: mov_reg_imm_64bit (REX.W B8+rd io)
            (Operand::Reg(r), Operand::Imm64(v)) => {
                let (idx, ext) = reg_index(r);
                let rex = self.rex_wrxb(true, false, ext);
                self.emit(&[rex, 0xB8 + idx]);
                // Track the exact offset of the imm64 field for PE string patching
                self.string_imm64_offsets.push(self.code.len());
                self.emit_u64(*v);
            }
            // mov r32, imm32 — FASM: mov_reg_imm_32bit (B8+rd id)
            (Operand::Reg(r), Operand::Imm32(v)) if r.is_32bit() => {
                let (idx, _) = reg_index(r);
                self.emit(&[0xB8 + idx]);
                self.emit_i32(*v);
            }
            // mov r64, imm32 (sign-extended) — FASM: mov_reg_64bit_imm_32bit
            (Operand::Reg(r), Operand::Imm32(v)) => {
                let (idx, ext) = reg_index(r);
                let rex = self.rex_wrxb(true, false, ext);
                let modrm = self.modrm(3, 0, idx);
                self.emit(&[rex, 0xC7, modrm]);
                self.emit_i32(*v);
            }
            // mov r64, [base+disp] — FASM: mov_reg_mem (8B /r)
            (Operand::Reg(r), Operand::Mem { base, disp }) => {
                self.encode_rm_disp(0x8B, r, base, *disp);
            }
            // mov [base+disp], r64 — FASM: basic_mem_reg (89 /r)
            (Operand::Mem { base, disp }, Operand::Reg(r)) => {
                self.encode_rm_disp(0x89, r, base, *disp);
            }
            // mov r64, r64 — FASM: mov_reg_reg (89 /r)
            (Operand::Reg(dst_r), Operand::Reg(src_r)) => {
                self.encode_rr(0x89, src_r, dst_r);
            }
            _ => {}
        }
    }

    // ========================================
    // MOVZX, LEA
    // ========================================

    /// FASM-inspired: generic MOVZX r64, r8 — supports ALL registers
    fn encode_movzx(&mut self, dst: &Reg, src: &Reg) {
        let (dst_idx, dst_ext) = reg_index(dst);
        let (src_idx, src_ext) = reg_index(src);
        let rex = self.rex_wrxb(true, dst_ext, src_ext);
        let modrm = self.modrm(3, dst_idx, src_idx);
        self.emit(&[rex, 0x0F, 0xB6, modrm]);
    }

    /// FASM-inspired: generic LEA r64, [base+disp] — supports ALL registers
    fn encode_lea(&mut self, dst: &Reg, src: &Operand) {
        if let Operand::Mem { base, disp } = src {
            self.encode_rm_disp(0x8D, dst, base, *disp);
        }
    }

    // ========================================
    // Arithmetic: ADD, SUB, MUL, DIV
    // ========================================

    /// FASM-inspired: generic ADD — supports ALL register/immediate combinations
    fn encode_add(&mut self, dst: &Operand, src: &Operand) {
        match (dst, src) {
            // add r64, r64 — FASM: basic_reg_reg (01 /r)
            (Operand::Reg(d), Operand::Reg(s)) => self.encode_rr(0x01, s, d),
            // add r64, imm — FASM: basic_reg_imm with auto imm8/imm32 (/0)
            (Operand::Reg(r), Operand::Imm8(v)) => self.encode_alu_ri(0, r, *v as i32),
            (Operand::Reg(r), Operand::Imm32(v)) => self.encode_alu_ri(0, r, *v),
            _ => {}
        }
    }

    /// FASM-inspired: generic SUB — supports ALL register/immediate combinations
    fn encode_sub(&mut self, dst: &Operand, src: &Operand) {
        match (dst, src) {
            // sub r64, r64 — FASM: basic_reg_reg (29 /r)
            (Operand::Reg(d), Operand::Reg(s)) => self.encode_rr(0x29, s, d),
            // sub r64, imm — FASM: basic_reg_imm with auto imm8/imm32 (/5)
            (Operand::Reg(r), Operand::Imm8(v)) => self.encode_alu_ri(5, r, *v as i32),
            (Operand::Reg(r), Operand::Imm32(v)) => self.encode_alu_ri(5, r, *v),
            _ => {}
        }
    }

    /// FASM-inspired: generic IMUL r64, r64 — supports ALL register combinations
    fn encode_mul(&mut self, dst: &Reg, src: &Reg) {
        let (dst_idx, dst_ext) = reg_index(dst);
        let (src_idx, src_ext) = reg_index(src);
        let rex = self.rex_wrxb(true, dst_ext, src_ext);
        let modrm = self.modrm(3, dst_idx, src_idx);
        self.emit(&[rex, 0x0F, 0xAF, modrm]);
    }

    /// FASM-inspired: generic IDIV r64 — supports ALL registers
    fn encode_div(&mut self, src: &Reg) {
        let (idx, ext) = reg_index(src);
        let rex_cqo = self.rex_wrxb(true, false, false);
        self.emit(&[rex_cqo, 0x99]); // cqo
        let rex = self.rex_wrxb(true, false, ext);
        let modrm = self.modrm(3, 7, idx); // IDIV = /7
        self.emit(&[rex, 0xF7, modrm]);
    }

    // ========================================
    // Bitwise: AND, OR, XOR
    // ========================================

    /// FASM-inspired: generic AND r64, r64
    fn encode_and(&mut self, dst: &Reg, src: &Reg) {
        self.encode_rr(0x21, src, dst);
    }

    /// FASM-inspired: generic OR r64, r64
    fn encode_or(&mut self, dst: &Reg, src: &Reg) {
        self.encode_rr(0x09, src, dst);
    }

    /// FASM-inspired: generic XOR r, r — auto-detects 32-bit for xor eax,eax optimization
    fn encode_xor(&mut self, dst: &Reg, src: &Reg) {
        if dst.is_32bit() {
            // 32-bit XOR: no REX.W needed (saves 1 byte, FASM-style size opt)
            let (dst_idx, dst_ext) = reg_index(dst);
            let (src_idx, src_ext) = reg_index(src);
            let modrm = self.modrm(3, src_idx, dst_idx);
            if dst_ext || src_ext {
                let rex = self.rex_wrxb(false, src_ext, dst_ext);
                self.emit(&[rex, 0x31, modrm]);
            } else {
                self.emit(&[0x31, modrm]);
            }
        } else {
            self.encode_rr(0x31, src, dst);
        }
    }

    // ========================================
    // INC, DEC, NEG, NOT, SHL
    // ========================================

    /// FASM-inspired: generic INC — supports ALL registers and memory
    fn encode_inc(&mut self, dst: &Operand) {
        match dst {
            Operand::Reg(r) => {
                // INC r64: REX.W FF /0
                let (idx, ext) = reg_index(r);
                let rex = self.rex_wrxb(true, false, ext);
                let modrm = self.modrm(3, 0, idx);
                self.emit(&[rex, 0xFF, modrm]);
            }
            Operand::Mem { base, disp } => {
                // INC [base+disp]: REX.W FF /0
                self.encode_ext_rm_disp(0xFF, 0, base, *disp);
            }
            _ => {}
        }
    }

    /// FASM-inspired: generic DEC — supports ALL registers and memory
    fn encode_dec(&mut self, dst: &Operand) {
        match dst {
            Operand::Reg(r) => {
                // DEC r64: REX.W FF /1
                let (idx, ext) = reg_index(r);
                let rex = self.rex_wrxb(true, false, ext);
                let modrm = self.modrm(3, 1, idx);
                self.emit(&[rex, 0xFF, modrm]);
            }
            Operand::Mem { base, disp } => {
                // DEC [base+disp]: REX.W FF /1
                self.encode_ext_rm_disp(0xFF, 1, base, *disp);
            }
            _ => {}
        }
    }

    /// FASM-inspired: generic NEG r64
    fn encode_neg(&mut self, dst: &Reg) {
        let (idx, ext) = reg_index(dst);
        let rex = self.rex_wrxb(true, false, ext);
        let modrm = self.modrm(3, 3, idx); // NEG = /3
        self.emit(&[rex, 0xF7, modrm]);
    }

    fn encode_not(&mut self, dst: &Reg) {
        // Logical NOT: test r, r; sete al; movzx rax, al
        let (idx, ext) = reg_index(dst);
        let rex = self.rex_wrxb(true, ext, ext);
        let modrm_test = self.modrm(3, idx, idx);
        self.emit(&[rex, 0x85, modrm_test]); // test r, r
        self.emit(&[0x0F, 0x94, 0xC0]); // sete al
        self.emit(&[0x48, 0x0F, 0xB6, 0xC0]); // movzx rax, al
    }

    /// FASM-inspired: generic SHL r64, imm8
    fn encode_shl(&mut self, dst: &Reg, amount: u8) {
        let (idx, ext) = reg_index(dst);
        let rex = self.rex_wrxb(true, false, ext);
        let modrm = self.modrm(3, 4, idx); // SHL = /4
        self.emit(&[rex, 0xC1, modrm, amount]);
    }

    // ========================================
    // CMP, TEST, SETCC
    // ========================================

    /// FASM-inspired: generic CMP — supports ALL register/memory combinations
    fn encode_cmp(&mut self, left: &Operand, right: &Operand) {
        match (left, right) {
            // cmp r64, r64 — (39 /r)
            (Operand::Reg(l), Operand::Reg(r)) => self.encode_rr(0x39, r, l),
            // cmp [base+disp], r64 — (39 /r)
            (Operand::Mem { base, disp }, Operand::Reg(r)) => {
                self.encode_rm_disp(0x39, r, base, *disp);
            }
            // cmp r64, [base+disp] — (3B /r)
            (Operand::Reg(r), Operand::Mem { base, disp }) => {
                self.encode_rm_disp(0x3B, r, base, *disp);
            }
            // cmp r64, imm32 — (81 /7 or 83 /7)
            (Operand::Reg(r), Operand::Imm32(v)) => self.encode_alu_ri(7, r, *v),
            _ => self.encode_rr(0x39, &Reg::RBX, &Reg::RAX),
        }
    }

    /// FASM-inspired: generic TEST r64, r64
    fn encode_test(&mut self, left: &Reg, right: &Reg) {
        self.encode_rr(0x85, right, left);
    }

    fn encode_setcc(&mut self, cond: &Condition) {
        match cond {
            Condition::Equal => self.emit(&[0x0F, 0x94, 0xC0]),
            Condition::NotEqual => self.emit(&[0x0F, 0x95, 0xC0]),
            Condition::Less => self.emit(&[0x0F, 0x9C, 0xC0]),
            Condition::LessEq => self.emit(&[0x0F, 0x9E, 0xC0]),
            Condition::Greater => self.emit(&[0x0F, 0x9F, 0xC0]),
            Condition::GreaterEq => self.emit(&[0x0F, 0x9D, 0xC0]),
            Condition::Always => {}
        }
    }

    // ========================================
    // PUSH, POP
    // ========================================

    /// FASM-inspired: generic PUSH — computed 50+rd, supports ALL 16 GPRs
    fn encode_push(&mut self, src: &Operand) {
        if let Operand::Reg(r) = src {
            self.encode_push_reg(r);
        }
    }

    /// FASM-inspired: generic POP — computed 58+rd, supports ALL 16 GPRs
    fn encode_pop(&mut self, dst: &Reg) {
        self.encode_pop_reg(dst);
    }

    // ========================================
    // CALL, JMP, Jcc
    // ========================================

    fn encode_call(&mut self, target: &CallTarget) {
        match target {
            CallTarget::Relative(label) => {
                self.emit(&[0xE8]);
                let patch_offset = self.code.len();
                self.emit_i32(0);
                self.pending_patches.push(PendingPatch {
                    code_offset: patch_offset,
                    target: *label,
                    kind: PatchKind::Rel32,
                    op_idx: self.current_op_idx,
                });
            }
            CallTarget::RipRelative(disp) => {
                self.emit(&[0xFF, 0x15]);
                self.emit_i32(*disp);
            }
            CallTarget::Name(name) => {
                self.emit(&[0xE8]);
                let offset = self.code.len();
                self.unresolved_calls.push((offset, name.clone()));
                self.emit_i32(0);
            }
            CallTarget::Register(reg) => {
                // call reg — FF /2 with ModR/M for register direct
                let (idx, ext) = reg_index(reg);
                if ext {
                    self.emit(&[0x41]); // REX.B
                }
                self.emit(&[0xFF, 0xD0 | idx]);
            }
        }
    }

    /// FASM-inspired: JMP with short/near auto-selection in patch phase
    fn encode_jmp(&mut self, target: &Label) {
        // Always emit near (rel32) first; patch phase may shrink to short (rel8)
        self.emit(&[0xE9]);
        let patch_offset = self.code.len();
        self.emit_i32(0);
        self.pending_patches.push(PendingPatch {
            code_offset: patch_offset,
            target: *target,
            kind: PatchKind::Rel32,
            op_idx: self.current_op_idx,
        });
    }

    /// FASM-inspired: Jcc with condition code table
    fn encode_jcc(&mut self, cond: &Condition, target: &Label) {
        // FASM pattern: condition code table instead of per-condition match
        let cc = match cond {
            Condition::Equal => 0x04u8,   // JE/JZ
            Condition::NotEqual => 0x05,  // JNE/JNZ
            Condition::Less => 0x0C,      // JL
            Condition::LessEq => 0x0E,    // JLE
            Condition::Greater => 0x0F,   // JG
            Condition::GreaterEq => 0x0D, // JGE
            Condition::Always => {
                self.encode_jmp(target);
                return;
            }
        };
        // Near Jcc: 0F 8x rel32
        self.emit(&[0x0F, 0x80 | cc]);
        let patch_offset = self.code.len();
        self.emit_i32(0);
        self.pending_patches.push(PendingPatch {
            code_offset: patch_offset,
            target: *target,
            kind: PatchKind::Rel32,
            op_idx: self.current_op_idx,
        });
    }

    // ========================================
    // MOVQ (SSE ↔ GP)
    // ========================================

    fn encode_movq(&mut self, dst: &Reg, src: &Reg) {
        match (dst, src) {
            (Reg::RAX, Reg::XMM0) => self.emit(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]),
            (Reg::XMM1, Reg::RDX) => self.emit(&[0x66, 0x48, 0x0F, 0x6E, 0xCA]),
            (Reg::XMM0, Reg::RAX) => self.emit(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]),
            _ => self.emit(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]),
        }
    }

    // ========================================
    // CallIAT (Windows Import Address Table)
    // ========================================

    fn encode_call_iat(&mut self, iat_rva: u32) {
        // call [rip+offset] donde offset = iat_rva - (current_rva + 6)
        // current_rva = 0x1000 (base de .text) + posición actual en código
        // El call [rip+disp32] tiene 6 bytes: FF 15 + disp32
        let current_rva = 0x1000u32 + self.code.len() as u32 + 6;
        let offset = iat_rva as i32 - current_rva as i32;
        self.emit(&[0xFF, 0x15]);
        // Track the exact offset of the disp32 field for PE patching
        self.iat_call_offsets.push(self.code.len());
        self.emit_i32(offset);
    }

    // ========================================
    // FASM-inspired Generic Encoding Primitives
    // ========================================
    // Instead of hardcoding per-register match arms, compute
    // REX + ModR/M from register indices — like FASM does.

    /// Compute REX prefix: REX.W=w, REX.R=reg_ext, REX.X=0, REX.B=rm_ext
    #[inline(always)]
    fn rex_wrxb(&self, w: bool, reg_ext: bool, rm_ext: bool) -> u8 {
        let mut rex = 0x40u8;
        if w {
            rex |= 0x08;
        } // REX.W
        if reg_ext {
            rex |= 0x04;
        } // REX.R
        if rm_ext {
            rex |= 0x01;
        } // REX.B
        rex
    }

    /// Compute ModR/M byte: mod | (reg << 3) | rm
    #[inline(always)]
    fn modrm(&self, mode: u8, reg: u8, rm: u8) -> u8 {
        (mode << 6) | ((reg & 7) << 3) | (rm & 7)
    }

    /// Generic reg-reg encoding: REX.W + opcode + ModR/M(11, src, dst)
    /// FASM pattern: basic_instruction with register operands
    fn encode_rr(&mut self, opcode: u8, reg: &Reg, rm: &Reg) {
        let (reg_idx, reg_ext) = reg_index(reg);
        let (rm_idx, rm_ext) = reg_index(rm);
        let rex = self.rex_wrxb(true, reg_ext, rm_ext);
        let modrm = self.modrm(3, reg_idx, rm_idx);
        self.emit(&[rex, opcode, modrm]);
    }

    /// Generic reg-[base+disp] encoding with auto disp8/disp32 selection
    /// FASM pattern: store_instruction with memory operand
    fn encode_rm_disp(&mut self, opcode: u8, reg: &Reg, base: &Reg, disp: i32) {
        let (reg_idx, reg_ext) = reg_index(reg);
        let (base_idx, base_ext) = reg_index(base);
        let rex = self.rex_wrxb(true, reg_ext, base_ext);
        let fits_i8 = disp >= -128 && disp <= 127 && disp != 0;
        if disp == 0 && base_idx != 5 {
            // [base] — mod=00 (but RBP(5) always needs disp8)
            let modrm = self.modrm(0, reg_idx, base_idx);
            self.emit(&[rex, opcode, modrm]);
        } else if fits_i8 {
            // [base+disp8] — mod=01 (saves 3 bytes vs disp32!)
            let modrm = self.modrm(1, reg_idx, base_idx);
            self.emit(&[rex, opcode, modrm, disp as u8]);
        } else {
            // [base+disp32] — mod=10
            let modrm = self.modrm(2, reg_idx, base_idx);
            self.emit(&[rex, opcode, modrm]);
            self.emit_i32(disp);
        }
    }

    /// Store16: mov WORD [base+disp], reg — 16-bit store (0x66 prefix)
    /// Encodes: 0x66 [optional REX] 89 ModR/M [disp]
    /// Used for VGA text mode writes (each cell = char:8 + attr:8 = 16 bits)
    fn encode_store16(&mut self, base: &Reg, disp: i32, src: &Reg) {
        let (reg_idx, reg_ext) = reg_index(src);
        let (base_idx, base_ext) = reg_index(base);
        let rex = self.rex_wrxb(false, reg_ext, base_ext);
        let fits_i8 = disp >= -128 && disp <= 127 && disp != 0;
        if disp == 0 && base_idx != 5 {
            let modrm = self.modrm(0, reg_idx, base_idx);
            if rex != 0x40 {
                self.emit(&[0x66, rex, 0x89, modrm]);
            } else {
                self.emit(&[0x66, 0x89, modrm]);
            }
        } else if fits_i8 {
            let modrm = self.modrm(1, reg_idx, base_idx);
            if rex != 0x40 {
                self.emit(&[0x66, rex, 0x89, modrm, disp as u8]);
            } else {
                self.emit(&[0x66, 0x89, modrm, disp as u8]);
            }
        } else {
            let modrm = self.modrm(2, reg_idx, base_idx);
            if rex != 0x40 {
                self.emit(&[0x66, rex, 0x89, modrm]);
            } else {
                self.emit(&[0x66, 0x89, modrm]);
            }
            self.emit_i32(disp);
        }
    }

    /// Store32: mov DWORD [base+disp], reg — 32-bit store (no REX.W)
    /// Encodes: [optional REX] 89 ModR/M [disp]
    /// Used for writing 4-byte fields (GUID Data1/Data2/Data3, D3D12 struct fields)
    fn encode_store32(&mut self, base: &Reg, disp: i32, src: &Reg) {
        let (reg_idx, reg_ext) = reg_index(src);
        let (base_idx, base_ext) = reg_index(base);
        // REX.W = false → 32-bit operand size
        let rex = self.rex_wrxb(false, reg_ext, base_ext);
        let fits_i8 = disp >= -128 && disp <= 127 && disp != 0;
        if disp == 0 && base_idx != 5 {
            let modrm = self.modrm(0, reg_idx, base_idx);
            if rex != 0x40 {
                self.emit(&[rex, 0x89, modrm]);
            } else {
                self.emit(&[0x89, modrm]); // no REX needed
            }
        } else if fits_i8 {
            let modrm = self.modrm(1, reg_idx, base_idx);
            if rex != 0x40 {
                self.emit(&[rex, 0x89, modrm, disp as u8]);
            } else {
                self.emit(&[0x89, modrm, disp as u8]);
            }
        } else {
            let modrm = self.modrm(2, reg_idx, base_idx);
            if rex != 0x40 {
                self.emit(&[rex, 0x89, modrm]);
            } else {
                self.emit(&[0x89, modrm]);
            }
            self.emit_i32(disp);
        }
    }

    /// Generic reg-[base+disp] with extension opcode (e.g. INC, DEC, NEG /reg_field)
    fn encode_ext_rm_disp(&mut self, opcode: u8, reg_field: u8, base: &Reg, disp: i32) {
        let (base_idx, base_ext) = reg_index(base);
        let rex = self.rex_wrxb(true, false, base_ext);
        let fits_i8 = disp >= -128 && disp <= 127 && disp != 0;
        if fits_i8 {
            let modrm = self.modrm(1, reg_field, base_idx);
            self.emit(&[rex, opcode, modrm, disp as u8]);
        } else {
            let modrm = self.modrm(2, reg_field, base_idx);
            self.emit(&[rex, opcode, modrm]);
            self.emit_i32(disp);
        }
    }

    /// Generic PUSH reg — FASM pattern: 50+rd or REX.B 50+rd
    fn encode_push_reg(&mut self, r: &Reg) {
        let (idx, ext) = reg_index(r);
        if ext {
            self.emit(&[0x41, 0x50 + idx]);
        } else {
            self.emit(&[0x50 + idx]);
        }
    }

    /// Generic POP reg — FASM pattern: 58+rd or REX.B 58+rd
    fn encode_pop_reg(&mut self, r: &Reg) {
        let (idx, ext) = reg_index(r);
        if ext {
            self.emit(&[0x41, 0x58 + idx]);
        } else {
            self.emit(&[0x58 + idx]);
        }
    }

    /// Generic ALU reg, imm with auto imm8/imm32 selection
    /// FASM pattern: basic_mem_simm_8bit — use sign-extended imm8 when possible
    fn encode_alu_ri(&mut self, ext_op: u8, r: &Reg, imm: i32) {
        let (idx, r_ext) = reg_index(r);
        let rex = self.rex_wrxb(true, false, r_ext);
        if imm >= -128 && imm <= 127 {
            // Short form: REX.W 83 /ext_op ib (3+1=4 bytes vs 3+4=7)
            let modrm = self.modrm(3, ext_op, idx);
            self.emit(&[rex, 0x83, modrm, imm as u8]);
        } else {
            // Long form: REX.W 81 /ext_op id
            let modrm = self.modrm(3, ext_op, idx);
            self.emit(&[rex, 0x81, modrm]);
            self.emit_i32(imm);
        }
    }

    #[inline(always)]
    fn emit(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    #[inline(always)]
    fn emit_i32(&mut self, value: i32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    #[inline(always)]
    fn emit_u64(&mut self, value: u64) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    #[inline(always)]
    fn emit_u16(&mut self, value: u16) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    #[inline(always)]
    fn emit_u32(&mut self, value: u32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    // ========================================
    // OS-Level: LGDT, LIDT
    // ========================================

    fn encode_lgdt(&mut self, src: &Operand) {
        // lgdt [mem] = 0x0F 0x01 /2 (ModR/M reg field = 2)
        match src {
            Operand::Mem { base, disp } => {
                let (base_idx, base_ext) = reg_index(base);
                if base_ext {
                    self.emit(&[0x41]); // REX.B
                }
                self.emit(&[0x0F, 0x01]);
                // ModR/M: mod=10 (disp32), reg=010 (/2), r/m=base
                let modrm = 0x80 | (2 << 3) | base_idx;
                self.emit(&[modrm]);
                self.emit_i32(*disp);
            }
            Operand::Reg(r) => {
                // lgdt with direct register addressing (mod=00)
                let (base_idx, _) = reg_index(r);
                self.emit(&[0x0F, 0x01]);
                let modrm = (2 << 3) | base_idx;
                self.emit(&[modrm]);
            }
            _ => self.emit(&[0x90]), // fallback nop
        }
    }

    fn encode_lidt(&mut self, src: &Operand) {
        // lidt [mem] = 0x0F 0x01 /3 (ModR/M reg field = 3)
        match src {
            Operand::Mem { base, disp } => {
                let (base_idx, base_ext) = reg_index(base);
                if base_ext {
                    self.emit(&[0x41]); // REX.B
                }
                self.emit(&[0x0F, 0x01]);
                let modrm = 0x80 | (3 << 3) | base_idx;
                self.emit(&[modrm]);
                self.emit_i32(*disp);
            }
            Operand::Reg(r) => {
                let (base_idx, _) = reg_index(r);
                self.emit(&[0x0F, 0x01]);
                let modrm = (3 << 3) | base_idx;
                self.emit(&[modrm]);
            }
            _ => self.emit(&[0x90]),
        }
    }

    // ========================================
    // OS-Level: MOV CRn
    // ========================================

    fn encode_mov_to_cr(&mut self, cr: u8, src: &Reg) {
        // mov crN, reg = 0x0F 0x22 ModR/M(11, crN, reg)
        let (src_idx, src_ext) = reg_index(src);
        // Only need REX prefix for extended registers
        if src_ext {
            self.emit(&[0x41]); // REX.B
        }
        self.emit(&[0x0F, 0x22]);
        let modrm = 0xC0 | ((cr & 0x07) << 3) | src_idx;
        self.emit(&[modrm]);
    }

    fn encode_mov_from_cr(&mut self, cr: u8, dst: &Reg) {
        // mov reg, crN = 0x0F 0x20 ModR/M(11, crN, reg)
        let (dst_idx, dst_ext) = reg_index(dst);
        if dst_ext {
            self.emit(&[0x41]); // REX.B
        }
        self.emit(&[0x0F, 0x20]);
        let modrm = 0xC0 | ((cr & 0x07) << 3) | dst_idx;
        self.emit(&[modrm]);
    }

    // ========================================
    // OS-Level: INVLPG
    // ========================================

    fn encode_invlpg(&mut self, addr: &Operand) {
        // invlpg [mem] = 0x0F 0x01 /7
        match addr {
            Operand::Mem { base, disp } => {
                let (base_idx, base_ext) = reg_index(base);
                if base_ext {
                    self.emit(&[0x41]);
                }
                self.emit(&[0x0F, 0x01]);
                let modrm = 0x80 | (7 << 3) | base_idx;
                self.emit(&[modrm]);
                self.emit_i32(*disp);
            }
            Operand::Reg(r) => {
                let (base_idx, _) = reg_index(r);
                self.emit(&[0x0F, 0x01]);
                let modrm = (7 << 3) | base_idx;
                self.emit(&[modrm]);
            }
            _ => self.emit(&[0x90]),
        }
    }

    // ========================================
    // OS-Level: IN / OUT (byte)
    // ========================================

    fn encode_in_byte(&mut self, port: &Operand) {
        match port {
            Operand::Imm8(p) => {
                // in al, imm8
                self.emit(&[0xE4, *p as u8]);
            }
            Operand::Reg(Reg::DX) => {
                // in al, dx
                self.emit(&[0xEC]);
            }
            _ => self.emit(&[0x90]),
        }
    }

    fn encode_out_byte(&mut self, port: &Operand) {
        match port {
            Operand::Imm8(p) => {
                // out imm8, al
                self.emit(&[0xE6, *p as u8]);
            }
            Operand::Reg(Reg::DX) => {
                // out dx, al
                self.emit(&[0xEE]);
            }
            _ => self.emit(&[0x90]),
        }
    }

    // ========================================
    // OS-Level: IN / OUT (dword) — PCI config space
    // ========================================

    fn encode_in_dword(&mut self, port: &Operand) {
        match port {
            Operand::Imm8(p) => {
                // in eax, imm8
                self.emit(&[0xE5, *p as u8]);
            }
            Operand::Reg(Reg::DX) => {
                // in eax, dx
                self.emit(&[0xED]);
            }
            _ => self.emit(&[0x90]),
        }
    }

    fn encode_out_dword(&mut self, port: &Operand) {
        match port {
            Operand::Imm8(p) => {
                // out imm8, eax
                self.emit(&[0xE7, *p as u8]);
            }
            Operand::Reg(Reg::DX) => {
                // out dx, eax
                self.emit(&[0xEF]);
            }
            _ => self.emit(&[0x90]),
        }
    }

    // ========================================
    // OS-Level: SHR
    // ========================================

    fn encode_shr(&mut self, dst: &Reg, amount: u8) {
        let (dst_idx, dst_ext) = reg_index(dst);
        let rex = if dst.is_64bit() {
            let mut r = 0x48u8;
            if dst_ext {
                r |= 0x01;
            }
            Some(r)
        } else if dst_ext {
            Some(0x41u8)
        } else {
            None
        };
        if let Some(rex_byte) = rex {
            self.emit(&[rex_byte]);
        }
        // shr r/m, imm8: C1 /5
        let modrm = 0xC0 | (5 << 3) | dst_idx;
        self.emit(&[0xC1, modrm, amount]);
    }

    // ========================================
    // OS-Level: Far JMP
    // ========================================

    fn encode_far_jmp(&mut self, selector: u16, offset: u32) {
        // far jmp ptr16:32 = 0xEA + offset32 + selector16
        self.emit(&[0xEA]);
        self.emit_u32(offset);
        self.emit_u16(selector);
    }

    fn encode_bitwise_not(&mut self, dst: &Reg) {
        if dst.is_64bit() {
            let (idx, ext) = reg_index(dst);
            let rex = 0x48 | if ext { 0x01 } else { 0x00 };
            self.emit(&[rex, 0xF7, 0xD0 | idx]);
        } else if dst.is_32bit() {
            let (idx, _) = reg_index(dst);
            self.emit(&[0xF7, 0xD0 | idx]);
        } else {
            self.emit(&[0x48, 0xF7, 0xD0]); // fallback RAX
        }
    }

    fn encode_shl_cl(&mut self, dst: &Reg) {
        if dst.is_64bit() {
            let (idx, ext) = reg_index(dst);
            let rex = 0x48 | if ext { 0x01 } else { 0x00 };
            self.emit(&[rex, 0xD3, 0xE0 | idx]); // SHL r64, CL
        } else {
            self.emit(&[0x48, 0xD3, 0xE0]); // fallback SHL RAX, CL
        }
    }

    fn encode_shr_cl(&mut self, dst: &Reg) {
        if dst.is_64bit() {
            let (idx, ext) = reg_index(dst);
            let rex = 0x48 | if ext { 0x01 } else { 0x00 };
            self.emit(&[rex, 0xD3, 0xE8 | idx]); // SHR r64, CL
        } else {
            self.emit(&[0x48, 0xD3, 0xE8]); // fallback SHR RAX, CL
        }
    }
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Retorna (índice 0-7, necesita extensión REX.B/R) para un registro.
fn reg_index(reg: &Reg) -> (u8, bool) {
    match reg {
        // 64-bit GPR
        Reg::RAX | Reg::EAX | Reg::AX | Reg::AL => (0, false),
        Reg::RCX | Reg::ECX | Reg::CX | Reg::CL => (1, false),
        Reg::RDX | Reg::EDX | Reg::DX | Reg::DL => (2, false),
        Reg::RBX | Reg::EBX | Reg::BX | Reg::BL => (3, false),
        Reg::RSP | Reg::ESP | Reg::SP | Reg::AH => (4, false),
        Reg::RBP | Reg::EBP | Reg::BP | Reg::CH => (5, false),
        Reg::RSI | Reg::ESI | Reg::SI | Reg::DH => (6, false),
        Reg::RDI | Reg::EDI | Reg::DI | Reg::BH => (7, false),
        Reg::R8 => (0, true),
        Reg::R9 => (1, true),
        Reg::R10 => (2, true),
        Reg::R11 => (3, true),
        Reg::R12 => (4, true),
        Reg::R13 => (5, true),
        Reg::R14 => (6, true),
        Reg::R15 => (7, true),
        // SSE/AVX XMM registers (128-bit)
        Reg::XMM0 => (0, false),
        Reg::XMM1 => (1, false),
        Reg::XMM2 => (2, false),
        Reg::XMM3 => (3, false),
        Reg::XMM4 => (4, false),
        Reg::XMM5 => (5, false),
        Reg::XMM6 => (6, false),
        Reg::XMM7 => (7, false),
        Reg::XMM8 => (0, true),
        Reg::XMM9 => (1, true),
        Reg::XMM10 => (2, true),
        Reg::XMM11 => (3, true),
        Reg::XMM12 => (4, true),
        Reg::XMM13 => (5, true),
        Reg::XMM14 => (6, true),
        Reg::XMM15 => (7, true),
        // AVX2 YMM registers (256-bit) — same encoding as XMM with VEX.L=1
        Reg::YMM0 => (0, false),
        Reg::YMM1 => (1, false),
        Reg::YMM2 => (2, false),
        Reg::YMM3 => (3, false),
        Reg::YMM4 => (4, false),
        Reg::YMM5 => (5, false),
        Reg::YMM6 => (6, false),
        Reg::YMM7 => (7, false),
        Reg::YMM8 => (0, true),
        Reg::YMM9 => (1, true),
        Reg::YMM10 => (2, true),
        Reg::YMM11 => (3, true),
        Reg::YMM12 => (4, true),
        Reg::YMM13 => (5, true),
        Reg::YMM14 => (6, true),
        Reg::YMM15 => (7, true),
        // Control registers (index = CR number)
        Reg::CR0 => (0, false),
        Reg::CR2 => (2, false),
        Reg::CR3 => (3, false),
        Reg::CR4 => (4, false),
        // Debug registers
        Reg::DR0 => (0, false),
        Reg::DR1 => (1, false),
        Reg::DR2 => (2, false),
        Reg::DR3 => (3, false),
        Reg::DR6 => (6, false),
        Reg::DR7 => (7, false),
        // Segment registers (encoding order)
        Reg::CS => (1, false),
        Reg::DS => (3, false),
        Reg::ES => (0, false),
        Reg::FS => (4, false),
        Reg::GS => (5, false),
        Reg::SS => (2, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop_prologue() {
        let mut enc = Encoder::new();
        let ops = vec![
            ADeadOp::Push {
                src: Operand::Reg(Reg::RBP),
            },
            ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBP),
                src: Operand::Reg(Reg::RSP),
            },
            ADeadOp::Pop { dst: Reg::RBP },
            ADeadOp::Ret,
        ];
        let result = enc.encode_all(&ops);
        assert_eq!(result.code, vec![0x55, 0x48, 0x89, 0xE5, 0x5D, 0xC3]);
    }

    #[test]
    fn test_mov_imm64() {
        let mut enc = Encoder::new();
        let ops = vec![ADeadOp::Mov {
            dst: Operand::Reg(Reg::RAX),
            src: Operand::Imm64(42),
        }];
        let result = enc.encode_all(&ops);
        let mut expected = vec![0x48, 0xB8];
        expected.extend_from_slice(&42u64.to_le_bytes());
        assert_eq!(result.code, expected);
    }

    #[test]
    fn test_xor_eax() {
        let mut enc = Encoder::new();
        let ops = vec![ADeadOp::Xor {
            dst: Reg::EAX,
            src: Reg::EAX,
        }];
        let result = enc.encode_all(&ops);
        assert_eq!(result.code, vec![0x31, 0xC0]);
    }

    #[test]
    fn test_arithmetic() {
        let mut enc = Encoder::new();
        let ops = vec![
            ADeadOp::Add {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Reg(Reg::RBX),
            },
            ADeadOp::Sub {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Reg(Reg::RBX),
            },
            ADeadOp::Mul {
                dst: Reg::RAX,
                src: Reg::RBX,
            },
        ];
        let result = enc.encode_all(&ops);
        assert_eq!(
            result.code,
            vec![
                0x48, 0x01, 0xD8, // add rax, rbx
                0x48, 0x29, 0xD8, // sub rax, rbx
                0x48, 0x0F, 0xAF, 0xC3, // imul rax, rbx
            ]
        );
    }

    #[test]
    fn test_jmp_label_resolution() {
        let mut enc = Encoder::new();
        let mut ir = ADeadIR::new();
        let lbl = ir.new_label();
        let ops = vec![
            ADeadOp::Label(lbl),
            ADeadOp::Nop,
            ADeadOp::Jmp { target: lbl },
        ];
        let result = enc.encode_all(&ops);
        // FASM multi-pass: Label at 0, nop at 0 (1 byte), jmp at 1
        // The multi-pass encoder detects that the backward jump fits in rel8
        // and shortens it: nop(1) + jmp_short(2) = 3 bytes instead of 6
        assert_eq!(result.code.len(), 3); // nop(1) + jmp_short(2)
    }

    // ========================================
    // OS-Level instruction tests
    // ========================================

    #[test]
    fn test_cli_sti_hlt() {
        let mut enc = Encoder::new();
        let ops = vec![ADeadOp::Cli, ADeadOp::Sti, ADeadOp::Hlt];
        let result = enc.encode_all(&ops);
        assert_eq!(result.code, vec![0xFA, 0xFB, 0xF4]);
    }

    #[test]
    fn test_int() {
        let mut enc = Encoder::new();
        let ops = vec![ADeadOp::Int { vector: 0x10 }, ADeadOp::Int { vector: 0x80 }];
        let result = enc.encode_all(&ops);
        assert_eq!(result.code, vec![0xCD, 0x10, 0xCD, 0x80]);
    }

    #[test]
    fn test_cpuid_rdmsr_wrmsr() {
        let mut enc = Encoder::new();
        let ops = vec![ADeadOp::Cpuid, ADeadOp::Rdmsr, ADeadOp::Wrmsr];
        let result = enc.encode_all(&ops);
        assert_eq!(
            result.code,
            vec![
                0x0F, 0xA2, // cpuid
                0x0F, 0x32, // rdmsr
                0x0F, 0x30, // wrmsr
            ]
        );
    }

    #[test]
    fn test_iretq() {
        let mut enc = Encoder::new();
        let ops = vec![ADeadOp::Iret];
        let result = enc.encode_all(&ops);
        assert_eq!(result.code, vec![0x48, 0xCF]);
    }

    #[test]
    fn test_in_out_byte_imm() {
        let mut enc = Encoder::new();
        let ops = vec![
            ADeadOp::InByte {
                port: Operand::Imm8(0x60),
            },
            ADeadOp::OutByte {
                port: Operand::Imm8(0x20),
                src: Operand::Reg(Reg::AL),
            },
        ];
        let result = enc.encode_all(&ops);
        assert_eq!(
            result.code,
            vec![
                0xE4, 0x60, // in al, 0x60
                0xE6, 0x20, // out 0x20, al
            ]
        );
    }

    #[test]
    fn test_in_out_byte_dx() {
        let mut enc = Encoder::new();
        let ops = vec![
            ADeadOp::InByte {
                port: Operand::Reg(Reg::DX),
            },
            ADeadOp::OutByte {
                port: Operand::Reg(Reg::DX),
                src: Operand::Reg(Reg::AL),
            },
        ];
        let result = enc.encode_all(&ops);
        assert_eq!(
            result.code,
            vec![
                0xEC, // in al, dx
                0xEE, // out dx, al
            ]
        );
    }

    #[test]
    fn test_far_jmp() {
        let mut enc = Encoder::new();
        let ops = vec![ADeadOp::FarJmp {
            selector: 0x08,
            offset: 0x7C00,
        }];
        let result = enc.encode_all(&ops);
        // EA 00 7C 00 00 08 00
        let mut expected = vec![0xEA];
        expected.extend_from_slice(&0x7C00u32.to_le_bytes());
        expected.extend_from_slice(&0x0008u16.to_le_bytes());
        assert_eq!(result.code, expected);
    }

    #[test]
    fn test_shr() {
        let mut enc = Encoder::new();
        let ops = vec![ADeadOp::Shr {
            dst: Reg::RAX,
            amount: 4,
        }];
        let result = enc.encode_all(&ops);
        // REX.W + C1 /5 ib => 48 C1 E8 04
        assert_eq!(result.code, vec![0x48, 0xC1, 0xE8, 0x04]);
    }
}
