// ============================================================
// ADead-BIB v8.0 — VEX Prefix Emitter
// ============================================================
// Genera instrucciones AVX/AVX2 con VEX prefix (C4/C5).
//
// VEX prefix encoding:
//   C5 = 2-byte VEX (cuando R=1, X=1, B=1 y map=0F)
//   C4 = 3-byte VEX (cuando necesitamos REX.R/X/B o map≠0F)
//
// Instrucciones 256-bit generadas:
//   VMOVAPS   ymm, [mem]    — carga alineada 256 bits
//   VMOVAPS   [mem], ymm    — almacena alineada 256 bits
//   VADDPS    ymm, ymm, ymm — suma 8 floats en paralelo
//   VSUBPS    ymm, ymm, ymm — resta 8 floats
//   VMULPS    ymm, ymm, ymm — multiplica 8 floats
//   VDIVPS    ymm, ymm, ymm — divide 8 floats
//   VFMADD231PS ymm, ymm, ymm — FMA (a += b * c)
//   VPCMPEQD  ymm, ymm, ymm — comparación paralela (BG)
//   VPTEST    ymm, ymm      — test paralelo (BG)
//   VZEROUPPER                — limpiar estado YMM superior
//
// Autor: Eddi Andreé Salazar Matos — Lima, Perú
// ADead-BIB — Binary Is Binary — VEX prefix nativo
// ============================================================

use super::ymm_allocator::YmmReg;

// ============================================================
// VEX Prefix Types
// ============================================================

/// VEX prefix form
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VexForm {
    /// 2-byte VEX (C5 xx) — most common, when no REX.X/B needed and map=0F
    Vex2,
    /// 3-byte VEX (C4 xx xx) — when REX.R/X/B or map 0F38/0F3A needed
    Vex3,
}

/// VEX opcode map
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VexMap {
    /// 0F — most SSE/AVX instructions
    Map0F,
    /// 0F 38 — SSSE3, SSE4.1, AVX2 integer, FMA
    Map0F38,
    /// 0F 3A — SSE4.1 immediate, AVX immediate
    Map0F3A,
}

impl VexMap {
    /// Map field encoding for 3-byte VEX
    pub fn mmmmm(&self) -> u8 {
        match self {
            VexMap::Map0F => 0x01,
            VexMap::Map0F38 => 0x02,
            VexMap::Map0F3A => 0x03,
        }
    }
}

/// VEX operand size prefix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VexPP {
    /// No prefix (packed single-precision float)
    None,
    /// 66 prefix (packed double-precision float, packed integer)
    P66,
    /// F3 prefix (scalar single-precision float)
    F3,
    /// F2 prefix (scalar double-precision float)
    F2,
}

impl VexPP {
    pub fn bits(&self) -> u8 {
        match self {
            VexPP::None => 0b00,
            VexPP::P66 => 0b01,
            VexPP::F3 => 0b10,
            VexPP::F2 => 0b11,
        }
    }
}

/// Vector length
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VexL {
    /// 128-bit (XMM)
    L128,
    /// 256-bit (YMM)
    L256,
}

impl VexL {
    pub fn bit(&self) -> u8 {
        match self {
            VexL::L128 => 0,
            VexL::L256 => 1,
        }
    }
}

// ============================================================
// AVX Instruction Definitions
// ============================================================

/// An AVX/AVX2 instruction to emit
#[derive(Debug, Clone)]
pub enum AvxInst {
    /// VMOVAPS ymm, [base+disp] — aligned load
    VmovapsLoad {
        dst: YmmReg,
        base: u8, // GP register encoding (0=RAX, 3=RBX, 5=RBP, etc.)
        disp: i32,
    },
    /// VMOVAPS [base+disp], ymm — aligned store
    VmovapsStore {
        src: YmmReg,
        base: u8,
        disp: i32,
    },
    /// VADDPS ymm, ymm, ymm — packed float32 add
    Vaddps {
        dst: YmmReg,
        src1: YmmReg,
        src2: YmmReg,
    },
    /// VSUBPS ymm, ymm, ymm — packed float32 sub
    Vsubps {
        dst: YmmReg,
        src1: YmmReg,
        src2: YmmReg,
    },
    /// VMULPS ymm, ymm, ymm — packed float32 mul
    Vmulps {
        dst: YmmReg,
        src1: YmmReg,
        src2: YmmReg,
    },
    /// VDIVPS ymm, ymm, ymm — packed float32 div
    Vdivps {
        dst: YmmReg,
        src1: YmmReg,
        src2: YmmReg,
    },
    /// VFMADD231PS ymm, ymm, ymm — fused multiply-add (dst += src1 * src2)
    Vfmadd231ps {
        dst: YmmReg,
        src1: YmmReg,
        src2: YmmReg,
    },
    /// VPCMPEQD ymm, ymm, ymm — packed int32 compare equal
    Vpcmpeqd {
        dst: YmmReg,
        src1: YmmReg,
        src2: YmmReg,
    },
    /// VPTEST ymm, ymm — packed bitwise test (sets ZF/CF)
    Vptest {
        src1: YmmReg,
        src2: YmmReg,
    },
    /// VXORPS ymm, ymm, ymm — bitwise XOR (used for zeroing)
    Vxorps {
        dst: YmmReg,
        src1: YmmReg,
        src2: YmmReg,
    },
    /// VZEROUPPER — clear upper 128 bits of all YMM registers
    Vzeroupper,
}

impl std::fmt::Display for AvxInst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AvxInst::VmovapsLoad { dst, base, disp } => {
                write!(f, "vmovaps {}, [gp{}+{}]", dst, base, disp)
            }
            AvxInst::VmovapsStore { src, base, disp } => {
                write!(f, "vmovaps [gp{}+{}], {}", base, disp, src)
            }
            AvxInst::Vaddps { dst, src1, src2 } => {
                write!(f, "vaddps {}, {}, {}", dst, src1, src2)
            }
            AvxInst::Vsubps { dst, src1, src2 } => {
                write!(f, "vsubps {}, {}, {}", dst, src1, src2)
            }
            AvxInst::Vmulps { dst, src1, src2 } => {
                write!(f, "vmulps {}, {}, {}", dst, src1, src2)
            }
            AvxInst::Vdivps { dst, src1, src2 } => {
                write!(f, "vdivps {}, {}, {}", dst, src1, src2)
            }
            AvxInst::Vfmadd231ps { dst, src1, src2 } => {
                write!(f, "vfmadd231ps {}, {}, {}", dst, src1, src2)
            }
            AvxInst::Vpcmpeqd { dst, src1, src2 } => {
                write!(f, "vpcmpeqd {}, {}, {}", dst, src1, src2)
            }
            AvxInst::Vptest { src1, src2 } => {
                write!(f, "vptest {}, {}", src1, src2)
            }
            AvxInst::Vxorps { dst, src1, src2 } => {
                write!(f, "vxorps {}, {}, {}", dst, src1, src2)
            }
            AvxInst::Vzeroupper => write!(f, "vzeroupper"),
        }
    }
}

// ============================================================
// VexEmitter — Byte-level encoder
// ============================================================

/// Emits VEX-prefixed x86-64 bytes for AVX/AVX2 instructions.
///
/// # Encoding Reference
/// ```text
/// 2-byte VEX: C5 [RvvvvLpp]
///   R: inverted REX.R (1=normal, 0=REX.R)
///   vvvv: inverted source register (1111=none)
///   L: vector length (0=128, 1=256)
///   pp: prefix (00=none, 01=66, 10=F3, 11=F2)
///
/// 3-byte VEX: C4 [RXBmmmmm] [WvvvvLpp]
///   R/X/B: inverted REX bits
///   mmmmm: opcode map (01=0F, 02=0F38, 03=0F3A)
///   W: REX.W (0 for most AVX)
///   vvvv/L/pp: same as 2-byte
/// ```
pub struct VexEmitter {
    /// Accumulated bytes
    bytes: Vec<u8>,
}

impl VexEmitter {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Get the emitted bytes
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Take ownership of the emitted bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Number of bytes emitted
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether any bytes have been emitted
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Clear all emitted bytes
    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    /// Emit a 2-byte VEX prefix
    fn emit_vex2(&mut self, r: bool, vvvv: u8, l: VexL, pp: VexPP) {
        let byte2 = (!r as u8 & 1) << 7
            | ((!vvvv & 0xF) << 3)
            | (l.bit() << 2)
            | pp.bits();
        self.bytes.push(0xC5);
        self.bytes.push(byte2);
    }

    /// Emit a 3-byte VEX prefix
    fn emit_vex3(
        &mut self,
        r: bool,
        x: bool,
        b: bool,
        map: VexMap,
        w: bool,
        vvvv: u8,
        l: VexL,
        pp: VexPP,
    ) {
        let byte2 = (!r as u8 & 1) << 7
            | (!x as u8 & 1) << 6
            | (!b as u8 & 1) << 5
            | map.mmmmm();
        let byte3 = (w as u8) << 7
            | ((!vvvv & 0xF) << 3)
            | (l.bit() << 2)
            | pp.bits();
        self.bytes.push(0xC4);
        self.bytes.push(byte2);
        self.bytes.push(byte3);
    }

    /// Emit ModR/M byte for reg-reg operation
    fn emit_modrm_rr(&mut self, reg: u8, rm: u8) {
        self.bytes.push(0xC0 | ((reg & 7) << 3) | (rm & 7));
    }

    /// Emit ModR/M + optional SIB + displacement for memory operand
    fn emit_modrm_mem(&mut self, reg: u8, base: u8, disp: i32) {
        let reg_field = (reg & 7) << 3;

        if disp == 0 && (base & 7) != 5 {
            // [base] — no displacement (RBP requires disp8=0)
            self.bytes.push(reg_field | (base & 7));
            if (base & 7) == 4 {
                // RSP needs SIB
                self.bytes.push(0x24);
            }
        } else if disp >= -128 && disp <= 127 {
            // [base+disp8]
            self.bytes.push(0x40 | reg_field | (base & 7));
            if (base & 7) == 4 {
                self.bytes.push(0x24);
            }
            self.bytes.push(disp as u8);
        } else {
            // [base+disp32]
            self.bytes.push(0x80 | reg_field | (base & 7));
            if (base & 7) == 4 {
                self.bytes.push(0x24);
            }
            self.bytes.extend_from_slice(&(disp as i32).to_le_bytes());
        }
    }

    /// Determine whether we need 2-byte or 3-byte VEX
    fn needs_vex3(dst: &YmmReg, src2: Option<&YmmReg>, map: VexMap) -> bool {
        // Need VEX3 if: map != 0F, or any register >= 8
        if map != VexMap::Map0F {
            return true;
        }
        if dst.needs_rex_r() {
            return true;
        }
        if let Some(s) = src2 {
            if s.needs_rex_r() {
                return true;
            }
        }
        false
    }

    /// Emit a register-register AVX instruction: OP ymm, ymm, ymm
    fn emit_avx_rrr(
        &mut self,
        opcode: u8,
        dst: &YmmReg,
        src1: &YmmReg,
        src2: &YmmReg,
        map: VexMap,
        pp: VexPP,
    ) {
        if Self::needs_vex3(dst, Some(src2), map) {
            self.emit_vex3(
                !dst.needs_rex_r(),
                true,
                !src2.needs_rex_r(),
                map,
                false,
                src1.index(),
                VexL::L256,
                pp,
            );
        } else {
            self.emit_vex2(!dst.needs_rex_r(), src1.index(), VexL::L256, pp);
        }
        self.bytes.push(opcode);
        self.emit_modrm_rr(dst.modrm_reg(), src2.modrm_reg());
    }

    /// Emit an AVX instruction
    pub fn emit(&mut self, inst: &AvxInst) {
        match inst {
            AvxInst::VmovapsLoad { dst, base, disp } => {
                // VMOVAPS ymm, [mem]: VEX.256.0F.WIG 28 /r
                if dst.needs_rex_r() || *base >= 8 {
                    self.emit_vex3(
                        !dst.needs_rex_r(),
                        true,
                        *base < 8,
                        VexMap::Map0F,
                        false,
                        0xF, // no vvvv source
                        VexL::L256,
                        VexPP::None,
                    );
                } else {
                    self.emit_vex2(!dst.needs_rex_r(), 0xF, VexL::L256, VexPP::None);
                }
                self.bytes.push(0x28);
                self.emit_modrm_mem(dst.modrm_reg(), *base, *disp);
            }

            AvxInst::VmovapsStore { src, base, disp } => {
                // VMOVAPS [mem], ymm: VEX.256.0F.WIG 29 /r
                if src.needs_rex_r() || *base >= 8 {
                    self.emit_vex3(
                        !src.needs_rex_r(),
                        true,
                        *base < 8,
                        VexMap::Map0F,
                        false,
                        0xF,
                        VexL::L256,
                        VexPP::None,
                    );
                } else {
                    self.emit_vex2(!src.needs_rex_r(), 0xF, VexL::L256, VexPP::None);
                }
                self.bytes.push(0x29);
                self.emit_modrm_mem(src.modrm_reg(), *base, *disp);
            }

            AvxInst::Vaddps { dst, src1, src2 } => {
                // VADDPS: VEX.NDS.256.0F.WIG 58 /r
                self.emit_avx_rrr(0x58, dst, src1, src2, VexMap::Map0F, VexPP::None);
            }

            AvxInst::Vsubps { dst, src1, src2 } => {
                // VSUBPS: VEX.NDS.256.0F.WIG 5C /r
                self.emit_avx_rrr(0x5C, dst, src1, src2, VexMap::Map0F, VexPP::None);
            }

            AvxInst::Vmulps { dst, src1, src2 } => {
                // VMULPS: VEX.NDS.256.0F.WIG 59 /r
                self.emit_avx_rrr(0x59, dst, src1, src2, VexMap::Map0F, VexPP::None);
            }

            AvxInst::Vdivps { dst, src1, src2 } => {
                // VDIVPS: VEX.NDS.256.0F.WIG 5E /r
                self.emit_avx_rrr(0x5E, dst, src1, src2, VexMap::Map0F, VexPP::None);
            }

            AvxInst::Vfmadd231ps { dst, src1, src2 } => {
                // VFMADD231PS: VEX.NDS.256.66.0F38.W0 B8 /r
                self.emit_avx_rrr(0xB8, dst, src1, src2, VexMap::Map0F38, VexPP::P66);
            }

            AvxInst::Vpcmpeqd { dst, src1, src2 } => {
                // VPCMPEQD: VEX.NDS.256.66.0F.WIG 76 /r
                self.emit_avx_rrr(0x76, dst, src1, src2, VexMap::Map0F, VexPP::P66);
            }

            AvxInst::Vptest { src1, src2 } => {
                // VPTEST: VEX.256.66.0F38.WIG 17 /r
                if Self::needs_vex3(src1, Some(src2), VexMap::Map0F38) {
                    self.emit_vex3(
                        !src1.needs_rex_r(),
                        true,
                        !src2.needs_rex_r(),
                        VexMap::Map0F38,
                        false,
                        0xF, // no vvvv for VPTEST
                        VexL::L256,
                        VexPP::P66,
                    );
                } else {
                    self.emit_vex2(!src1.needs_rex_r(), 0xF, VexL::L256, VexPP::P66);
                }
                self.bytes.push(0x17);
                self.emit_modrm_rr(src1.modrm_reg(), src2.modrm_reg());
            }

            AvxInst::Vxorps { dst, src1, src2 } => {
                // VXORPS: VEX.NDS.256.0F.WIG 57 /r
                self.emit_avx_rrr(0x57, dst, src1, src2, VexMap::Map0F, VexPP::None);
            }

            AvxInst::Vzeroupper => {
                // VZEROUPPER: VEX.128.0F.WIG 77
                self.emit_vex2(true, 0xF, VexL::L128, VexPP::None);
                self.bytes.push(0x77);
            }
        }
    }

    /// Emit multiple instructions
    pub fn emit_all(&mut self, insts: &[AvxInst]) {
        for inst in insts {
            self.emit(inst);
        }
    }
}

impl Default for VexEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vzeroupper() {
        let mut emitter = VexEmitter::new();
        emitter.emit(&AvxInst::Vzeroupper);
        let bytes = emitter.bytes();
        // VZEROUPPER = C5 F8 77
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes[0], 0xC5);
        assert_eq!(bytes[2], 0x77);
    }

    #[test]
    fn test_vaddps_ymm0_ymm0_ymm1() {
        let mut emitter = VexEmitter::new();
        emitter.emit(&AvxInst::Vaddps {
            dst: YmmReg(0),
            src1: YmmReg(0),
            src2: YmmReg(1),
        });
        let bytes = emitter.bytes();
        // Should be 2-byte VEX (all regs < 8, map=0F)
        assert_eq!(bytes[0], 0xC5);
        // Opcode for VADDPS = 0x58
        assert!(bytes.contains(&0x58));
    }

    #[test]
    fn test_vmovaps_load() {
        let mut emitter = VexEmitter::new();
        emitter.emit(&AvxInst::VmovapsLoad {
            dst: YmmReg(0),
            base: 5, // RBP
            disp: -32,
        });
        let bytes = emitter.bytes();
        assert!(!bytes.is_empty());
        // First byte should be VEX prefix
        assert!(bytes[0] == 0xC5 || bytes[0] == 0xC4);
    }

    #[test]
    fn test_vfmadd231ps_uses_vex3() {
        let mut emitter = VexEmitter::new();
        emitter.emit(&AvxInst::Vfmadd231ps {
            dst: YmmReg(0),
            src1: YmmReg(1),
            src2: YmmReg(2),
        });
        let bytes = emitter.bytes();
        // VFMADD231PS needs map 0F38 → must use 3-byte VEX (C4)
        assert_eq!(bytes[0], 0xC4);
    }

    #[test]
    fn test_vxorps_for_zeroing() {
        let mut emitter = VexEmitter::new();
        // VXORPS ymm0, ymm0, ymm0 — efficient zero
        emitter.emit(&AvxInst::Vxorps {
            dst: YmmReg(0),
            src1: YmmReg(0),
            src2: YmmReg(0),
        });
        let bytes = emitter.bytes();
        assert!(!bytes.is_empty());
        assert!(bytes.contains(&0x57)); // VXORPS opcode
    }

    #[test]
    fn test_multiple_instructions() {
        let mut emitter = VexEmitter::new();
        let insts = vec![
            AvxInst::VmovapsLoad {
                dst: YmmReg(0),
                base: 3, // RBX
                disp: 0,
            },
            AvxInst::VmovapsLoad {
                dst: YmmReg(1),
                base: 3,
                disp: 32,
            },
            AvxInst::Vaddps {
                dst: YmmReg(0),
                src1: YmmReg(0),
                src2: YmmReg(1),
            },
            AvxInst::VmovapsStore {
                src: YmmReg(0),
                base: 3,
                disp: 0,
            },
            AvxInst::Vzeroupper,
        ];
        emitter.emit_all(&insts);
        assert!(emitter.len() > 15); // should be multiple instructions
    }

    #[test]
    fn test_high_ymm_register() {
        let mut emitter = VexEmitter::new();
        // YMM8 needs REX.R → should use 3-byte VEX
        emitter.emit(&AvxInst::Vaddps {
            dst: YmmReg(8),
            src1: YmmReg(0),
            src2: YmmReg(1),
        });
        let bytes = emitter.bytes();
        assert_eq!(bytes[0], 0xC4); // 3-byte VEX needed
    }
}
