// ============================================================
// ADead-BIB ISA Decoder — bytes x86-64 → ADeadOp
// ============================================================
// Convierte bytes de máquina x86-64 en instrucciones tipadas.
// Esto permite el Path B del Binary Layout Optimizer:
//   PE/ELF existente → Decoder → ADeadIR → Optimizer → Rebuilder
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use super::*;
use std::collections::HashMap;

/// Decoder de bytes x86-64 a instrucciones ADeadOp.
pub struct Decoder {
    label_counter: u32,
    /// Mapeo offset_destino → Label (para reusar labels en saltos al mismo target)
    target_labels: HashMap<usize, Label>,
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            label_counter: 0,
            target_labels: HashMap::new(),
        }
    }

    /// Decodifica todos los bytes en una secuencia de ADeadOp.
    pub fn decode_all(&mut self, code: &[u8]) -> Vec<ADeadOp> {
        let mut ops: Vec<(usize, ADeadOp)> = Vec::new();
        let mut offset = 0;

        while offset < code.len() {
            if let Some((op, consumed)) = self.decode_one(code, offset) {
                ops.push((offset, op));
                offset += consumed;
            } else {
                ops.push((offset, ADeadOp::RawBytes(vec![code[offset]])));
                offset += 1;
            }
        }

        // Insertar Label pseudo-ops en las posiciones referenciadas por saltos
        let mut label_inserts: Vec<(usize, Label)> = Vec::new();
        for (label_offset, label) in &self.target_labels {
            label_inserts.push((*label_offset, *label));
        }
        label_inserts.sort_by_key(|(off, _)| *off);

        let mut result: Vec<ADeadOp> = Vec::new();
        let mut insert_idx = 0;

        for (byte_offset, op) in &ops {
            while insert_idx < label_inserts.len() && label_inserts[insert_idx].0 == *byte_offset {
                result.push(ADeadOp::Label(label_inserts[insert_idx].1));
                insert_idx += 1;
            }
            result.push(op.clone());
        }
        // Labels al final
        while insert_idx < label_inserts.len() {
            result.push(ADeadOp::Label(label_inserts[insert_idx].1));
            insert_idx += 1;
        }

        result
    }

    /// Decodifica una instrucción en el offset dado.
    /// Retorna (instrucción, bytes_consumidos) o None si no se reconoce.
    pub fn decode_one(&mut self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        if offset >= code.len() {
            return None;
        }

        let b0 = code[offset];

        match b0 {
            // Single-byte
            0x55 => Some((
                ADeadOp::Push {
                    src: Operand::Reg(Reg::RBP),
                },
                1,
            )),
            0x5D => Some((ADeadOp::Pop { dst: Reg::RBP }, 1)),
            0x50 => Some((
                ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                },
                1,
            )),
            0x58 => Some((ADeadOp::Pop { dst: Reg::RAX }, 1)),
            0x51 => Some((
                ADeadOp::Push {
                    src: Operand::Reg(Reg::RCX),
                },
                1,
            )),
            0x59 => Some((ADeadOp::Pop { dst: Reg::RCX }, 1)),
            0x52 => Some((
                ADeadOp::Push {
                    src: Operand::Reg(Reg::RDX),
                },
                1,
            )),
            0x5A => Some((ADeadOp::Pop { dst: Reg::RDX }, 1)),
            0x53 => Some((
                ADeadOp::Push {
                    src: Operand::Reg(Reg::RBX),
                },
                1,
            )),
            0x5B => Some((ADeadOp::Pop { dst: Reg::RBX }, 1)),
            0xC3 => Some((ADeadOp::Ret, 1)),
            0x90 => Some((ADeadOp::Nop, 1)),

            // mov eax, imm32
            0xB8 => {
                if offset + 5 <= code.len() {
                    let v = read_i32(code, offset + 1);
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Reg(Reg::EAX),
                            src: Operand::Imm32(v),
                        },
                        5,
                    ))
                } else {
                    None
                }
            }

            // 0x41 prefix (R8-R15)
            0x41 if offset + 1 < code.len() => match code[offset + 1] {
                0x50 => Some((
                    ADeadOp::Push {
                        src: Operand::Reg(Reg::R8),
                    },
                    2,
                )),
                0x51 => Some((
                    ADeadOp::Push {
                        src: Operand::Reg(Reg::R9),
                    },
                    2,
                )),
                0x58 => Some((ADeadOp::Pop { dst: Reg::R8 }, 2)),
                0x59 => Some((ADeadOp::Pop { dst: Reg::R9 }, 2)),
                _ => None,
            },

            // xor r32, r32
            0x31 if offset + 1 < code.len() => match code[offset + 1] {
                0xC0 => Some((
                    ADeadOp::Xor {
                        dst: Reg::EAX,
                        src: Reg::EAX,
                    },
                    2,
                )),
                0xC9 => Some((
                    ADeadOp::Xor {
                        dst: Reg::ECX,
                        src: Reg::ECX,
                    },
                    2,
                )),
                _ => None,
            },

            // Short jumps (rel8)
            0x7C if offset + 1 < code.len() => {
                let rel = code[offset + 1] as i8;
                let target_addr = (offset as isize + 2 + rel as isize) as usize;
                let label = self.get_or_create_label(target_addr);
                Some((
                    ADeadOp::Jcc {
                        cond: Condition::Less,
                        target: label,
                    },
                    2,
                ))
            }
            0x7D if offset + 1 < code.len() => {
                let rel = code[offset + 1] as i8;
                let target_addr = (offset as isize + 2 + rel as isize) as usize;
                let label = self.get_or_create_label(target_addr);
                Some((
                    ADeadOp::Jcc {
                        cond: Condition::GreaterEq,
                        target: label,
                    },
                    2,
                ))
            }

            // 0x0F two-byte opcodes
            0x0F if offset + 2 < code.len() => self.decode_0f(code, offset),

            // REX.W = 0x48
            0x48 if offset + 1 < code.len() => self.decode_rex_w(code, offset),

            // REX.WB = 0x49 (R8-R15 dst)
            0x49 if offset + 1 < code.len() => self.decode_rex_wb(code, offset),

            // REX.WR = 0x4C (R8-R15 src)
            0x4C if offset + 1 < code.len() => self.decode_rex_wr(code, offset),

            // JMP rel32
            0xE9 if offset + 4 < code.len() => {
                let rel = read_i32(code, offset + 1);
                let target_addr = (offset as isize + 5 + rel as isize) as usize;
                let label = self.get_or_create_label(target_addr);
                Some((ADeadOp::Jmp { target: label }, 5))
            }

            // CALL rel32
            0xE8 if offset + 4 < code.len() => {
                let rel = read_i32(code, offset + 1);
                let target_addr = (offset as isize + 5 + rel as isize) as usize;
                Some((
                    ADeadOp::Call {
                        target: CallTarget::Name(format!("func_{:04X}", target_addr)),
                    },
                    5,
                ))
            }

            // CALL [rip+disp32] / JMP [rip+disp32]
            0xFF if offset + 5 < code.len() && code[offset + 1] == 0x15 => {
                let disp = read_i32(code, offset + 2);
                Some((
                    ADeadOp::Call {
                        target: CallTarget::RipRelative(disp),
                    },
                    6,
                ))
            }

            // SSE: F2 prefix (cvtsi2sd)
            0xF2 if offset + 4 < code.len() => {
                if code[offset + 1] == 0x48
                    && code[offset + 2] == 0x0F
                    && code[offset + 3] == 0x2A
                    && code[offset + 4] == 0xC0
                {
                    Some((
                        ADeadOp::CvtSi2Sd {
                            dst: Reg::XMM0,
                            src: Reg::RAX,
                        },
                        5,
                    ))
                } else {
                    None
                }
            }

            // SSE: 66 prefix (movq)
            0x66 if offset + 4 < code.len() => self.decode_66_prefix(code, offset),

            _ => None,
        }
    }

    // ========================================
    // 0x0F two-byte opcodes
    // ========================================

    fn decode_0f(&mut self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        let b1 = code[offset + 1];
        match b1 {
            // SETcc al
            0x94 if offset + 2 < code.len() && code[offset + 2] == 0xC0 => Some((
                ADeadOp::SetCC {
                    cond: Condition::Equal,
                    dst: Reg::AL,
                },
                3,
            )),
            0x95 if offset + 2 < code.len() && code[offset + 2] == 0xC0 => Some((
                ADeadOp::SetCC {
                    cond: Condition::NotEqual,
                    dst: Reg::AL,
                },
                3,
            )),
            0x9C if offset + 2 < code.len() && code[offset + 2] == 0xC0 => Some((
                ADeadOp::SetCC {
                    cond: Condition::Less,
                    dst: Reg::AL,
                },
                3,
            )),
            0x9E if offset + 2 < code.len() && code[offset + 2] == 0xC0 => Some((
                ADeadOp::SetCC {
                    cond: Condition::LessEq,
                    dst: Reg::AL,
                },
                3,
            )),
            0x9F if offset + 2 < code.len() && code[offset + 2] == 0xC0 => Some((
                ADeadOp::SetCC {
                    cond: Condition::Greater,
                    dst: Reg::AL,
                },
                3,
            )),
            0x9D if offset + 2 < code.len() && code[offset + 2] == 0xC0 => Some((
                ADeadOp::SetCC {
                    cond: Condition::GreaterEq,
                    dst: Reg::AL,
                },
                3,
            )),
            // Jcc rel32
            0x84 if offset + 5 < code.len() => {
                let rel = read_i32(code, offset + 2);
                let target_addr = (offset as isize + 6 + rel as isize) as usize;
                let label = self.get_or_create_label(target_addr);
                Some((
                    ADeadOp::Jcc {
                        cond: Condition::Equal,
                        target: label,
                    },
                    6,
                ))
            }
            0x85 if offset + 5 < code.len() => {
                let rel = read_i32(code, offset + 2);
                let target_addr = (offset as isize + 6 + rel as isize) as usize;
                let label = self.get_or_create_label(target_addr);
                Some((
                    ADeadOp::Jcc {
                        cond: Condition::NotEqual,
                        target: label,
                    },
                    6,
                ))
            }
            0x8C if offset + 5 < code.len() => {
                let rel = read_i32(code, offset + 2);
                let target_addr = (offset as isize + 6 + rel as isize) as usize;
                let label = self.get_or_create_label(target_addr);
                Some((
                    ADeadOp::Jcc {
                        cond: Condition::Less,
                        target: label,
                    },
                    6,
                ))
            }
            0x8D if offset + 5 < code.len() => {
                let rel = read_i32(code, offset + 2);
                let target_addr = (offset as isize + 6 + rel as isize) as usize;
                let label = self.get_or_create_label(target_addr);
                Some((
                    ADeadOp::Jcc {
                        cond: Condition::GreaterEq,
                        target: label,
                    },
                    6,
                ))
            }
            0x8F if offset + 5 < code.len() => {
                let rel = read_i32(code, offset + 2);
                let target_addr = (offset as isize + 6 + rel as isize) as usize;
                let label = self.get_or_create_label(target_addr);
                Some((
                    ADeadOp::Jcc {
                        cond: Condition::Greater,
                        target: label,
                    },
                    6,
                ))
            }
            // SYSCALL
            0x05 => Some((ADeadOp::Syscall, 2)),
            _ => None,
        }
    }

    // ========================================
    // REX.W (0x48) prefix
    // ========================================

    fn decode_rex_w(&mut self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        let b1 = code[offset + 1];
        match b1 {
            // MOV r64, imm64 (movabs)
            0xB8 => {
                if offset + 10 <= code.len() {
                    let v = read_u64(code, offset + 2);
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Imm64(v),
                        },
                        10,
                    ))
                } else {
                    None
                }
            }
            0xB9 => {
                if offset + 10 <= code.len() {
                    let v = read_u64(code, offset + 2);
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Imm64(v),
                        },
                        10,
                    ))
                } else {
                    None
                }
            }
            0xBA => {
                if offset + 10 <= code.len() {
                    let v = read_u64(code, offset + 2);
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDX),
                            src: Operand::Imm64(v),
                        },
                        10,
                    ))
                } else {
                    None
                }
            }
            0xBB => {
                if offset + 10 <= code.len() {
                    let v = read_u64(code, offset + 2);
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RBX),
                            src: Operand::Imm64(v),
                        },
                        10,
                    ))
                } else {
                    None
                }
            }
            0xBE => {
                if offset + 10 <= code.len() {
                    let v = read_u64(code, offset + 2);
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RSI),
                            src: Operand::Imm64(v),
                        },
                        10,
                    ))
                } else {
                    None
                }
            }
            0xBF => {
                if offset + 10 <= code.len() {
                    let v = read_u64(code, offset + 2);
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDI),
                            src: Operand::Imm64(v),
                        },
                        10,
                    ))
                } else {
                    None
                }
            }

            // MOV with ModR/M
            0x89 if offset + 2 < code.len() => self.decode_48_89(code, offset),
            0x8B if offset + 2 < code.len() => self.decode_48_8b(code, offset),

            // ADD, SUB, CMP reg, reg
            0x01 if offset + 2 < code.len() && code[offset + 2] == 0xD8 => Some((
                ADeadOp::Add {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Reg(Reg::RBX),
                },
                3,
            )),
            0x29 if offset + 2 < code.len() => match code[offset + 2] {
                0xD8 => Some((
                    ADeadOp::Sub {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    },
                    3,
                )),
                0xC3 => Some((
                    ADeadOp::Sub {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    },
                    3,
                )),
                _ => None,
            },
            0x39 if offset + 2 < code.len() && code[offset + 2] == 0xD8 => Some((
                ADeadOp::Cmp {
                    left: Operand::Reg(Reg::RAX),
                    right: Operand::Reg(Reg::RBX),
                },
                3,
            )),
            0x3B if offset + 2 < code.len() && code[offset + 2] == 0x85 => {
                if offset + 6 < code.len() {
                    let disp = read_i32(code, offset + 3);
                    Some((
                        ADeadOp::Cmp {
                            left: Operand::Reg(Reg::RAX),
                            right: Operand::Mem {
                                base: Reg::RBP,
                                disp,
                            },
                        },
                        7,
                    ))
                } else {
                    None
                }
            }

            // IMUL rax, rbx
            0x0F if offset + 3 < code.len()
                && code[offset + 2] == 0xAF
                && code[offset + 3] == 0xC3 =>
            {
                Some((
                    ADeadOp::Mul {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    },
                    4,
                ))
            }
            // MOVZX rax, al
            0x0F if offset + 3 < code.len()
                && code[offset + 2] == 0xB6
                && code[offset + 3] == 0xC0 =>
            {
                Some((
                    ADeadOp::MovZx {
                        dst: Reg::RAX,
                        src: Reg::AL,
                    },
                    4,
                ))
            }

            // Bitwise
            0x21 if offset + 2 < code.len() && code[offset + 2] == 0xD8 => Some((
                ADeadOp::And {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                },
                3,
            )),
            0x09 if offset + 2 < code.len() && code[offset + 2] == 0xD8 => Some((
                ADeadOp::Or {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                },
                3,
            )),
            0x31 if offset + 2 < code.len() && code[offset + 2] == 0xC0 => Some((
                ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RAX,
                },
                3,
            )),

            // TEST rax, rax
            0x85 if offset + 2 < code.len() && code[offset + 2] == 0xC0 => Some((
                ADeadOp::Test {
                    left: Reg::RAX,
                    right: Reg::RAX,
                },
                3,
            )),

            // NEG rax
            0xF7 if offset + 2 < code.len() && code[offset + 2] == 0xD8 => {
                Some((ADeadOp::Neg { dst: Reg::RAX }, 3))
            }
            // IDIV rbx (without preceding cqo)
            0xF7 if offset + 2 < code.len() && code[offset + 2] == 0xFB => {
                Some((ADeadOp::Div { src: Reg::RBX }, 3))
            }

            // CQO (0x48 0x99) — consumed as part of Div when followed by idiv
            0x99 => {
                if offset + 4 < code.len()
                    && code[offset + 2] == 0x48
                    && code[offset + 3] == 0xF7
                    && code[offset + 4] == 0xFB
                {
                    Some((ADeadOp::Div { src: Reg::RBX }, 5))
                } else {
                    Some((ADeadOp::RawBytes(vec![0x48, 0x99]), 2))
                }
            }

            // INC/DEC
            0xFF if offset + 2 < code.len() => self.decode_48_ff(code, offset),

            // SUB/ADD rsp
            0x81 if offset + 2 < code.len() => match code[offset + 2] {
                0xEC if offset + 6 < code.len() => {
                    let v = read_i32(code, offset + 3);
                    Some((
                        ADeadOp::Sub {
                            dst: Operand::Reg(Reg::RSP),
                            src: Operand::Imm32(v),
                        },
                        7,
                    ))
                }
                0xC4 if offset + 6 < code.len() => {
                    let v = read_i32(code, offset + 3);
                    Some((
                        ADeadOp::Add {
                            dst: Operand::Reg(Reg::RSP),
                            src: Operand::Imm32(v),
                        },
                        7,
                    ))
                }
                _ => None,
            },
            0x83 if offset + 3 < code.len() => match code[offset + 2] {
                0xEC => {
                    let v = code[offset + 3] as i8;
                    Some((
                        ADeadOp::Sub {
                            dst: Operand::Reg(Reg::RSP),
                            src: Operand::Imm8(v),
                        },
                        4,
                    ))
                }
                0xC4 => {
                    let v = code[offset + 3] as i8;
                    Some((
                        ADeadOp::Add {
                            dst: Operand::Reg(Reg::RSP),
                            src: Operand::Imm8(v),
                        },
                        4,
                    ))
                }
                _ => None,
            },

            // MOV reg64, imm32 (sign-extended)
            0xC7 if offset + 6 < code.len() => {
                let modrm = code[offset + 2];
                let v = read_i32(code, offset + 3);
                let reg = match modrm {
                    0xC0 => Some(Reg::RAX),
                    0xC1 => Some(Reg::RCX),
                    0xC7 => Some(Reg::RDI),
                    _ => None,
                };
                reg.map(|r| {
                    (
                        ADeadOp::Mov {
                            dst: Operand::Reg(r),
                            src: Operand::Imm32(v),
                        },
                        7,
                    )
                })
            }

            // LEA
            0x8D if offset + 2 < code.len() => match code[offset + 2] {
                0x85 if offset + 6 < code.len() => {
                    let disp = read_i32(code, offset + 3);
                    Some((
                        ADeadOp::Lea {
                            dst: Reg::RAX,
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp,
                            },
                        },
                        7,
                    ))
                }
                0x95 if offset + 6 < code.len() => {
                    let disp = read_i32(code, offset + 3);
                    Some((
                        ADeadOp::Lea {
                            dst: Reg::RDX,
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp,
                            },
                        },
                        7,
                    ))
                }
                _ => None,
            },

            // SHL rax, imm8
            0xC1 if offset + 3 < code.len() && code[offset + 2] == 0xE0 => {
                let amount = code[offset + 3];
                Some((
                    ADeadOp::Shl {
                        dst: Reg::RAX,
                        amount,
                    },
                    4,
                ))
            }

            _ => None,
        }
    }

    // ========================================
    // 0x48 0x89 — MOV r/m64, r64
    // ========================================

    fn decode_48_89(&self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        let modrm = code[offset + 2];
        match modrm {
            0xE5 => Some((
                ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBP),
                    src: Operand::Reg(Reg::RSP),
                },
                3,
            )),
            0xEC => Some((
                ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RSP),
                    src: Operand::Reg(Reg::RBP),
                },
                3,
            )),
            0xC3 => Some((
                ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                },
                3,
            )),
            0xC1 => Some((
                ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RCX),
                    src: Operand::Reg(Reg::RAX),
                },
                3,
            )),
            0xC2 => Some((
                ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RDX),
                    src: Operand::Reg(Reg::RAX),
                },
                3,
            )),
            0xC8 => Some((
                ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Reg(Reg::RCX),
                },
                3,
            )),
            // [rbp+disp32] stores
            0x85 if offset + 6 < code.len() => {
                let disp = read_i32(code, offset + 3);
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                        src: Operand::Reg(Reg::RAX),
                    },
                    7,
                ))
            }
            0x8D if offset + 6 < code.len() => {
                let disp = read_i32(code, offset + 3);
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                        src: Operand::Reg(Reg::RCX),
                    },
                    7,
                ))
            }
            0x95 if offset + 6 < code.len() => {
                let disp = read_i32(code, offset + 3);
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                        src: Operand::Reg(Reg::RDX),
                    },
                    7,
                ))
            }
            // [rbp+disp8] stores
            0x4D if offset + 3 < code.len() => {
                let disp = code[offset + 3] as i8 as i32;
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                        src: Operand::Reg(Reg::RCX),
                    },
                    4,
                ))
            }
            0x55 if offset + 3 < code.len() => {
                let disp = code[offset + 3] as i8 as i32;
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                        src: Operand::Reg(Reg::RDX),
                    },
                    4,
                ))
            }
            _ => None,
        }
    }

    // ========================================
    // 0x48 0x8B — MOV r64, r/m64
    // ========================================

    fn decode_48_8b(&self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        let modrm = code[offset + 2];
        match modrm {
            // [rbp+disp32] loads
            0x85 if offset + 6 < code.len() => {
                let disp = read_i32(code, offset + 3);
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                    },
                    7,
                ))
            }
            0x8D if offset + 6 < code.len() => {
                let disp = read_i32(code, offset + 3);
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RCX),
                        src: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                    },
                    7,
                ))
            }
            0x9D if offset + 6 < code.len() => {
                let disp = read_i32(code, offset + 3);
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                    },
                    7,
                ))
            }
            // [reg] loads (no displacement)
            0x00 => Some((
                ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Mem {
                        base: Reg::RAX,
                        disp: 0,
                    },
                },
                3,
            )),
            0x03 => Some((
                ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Mem {
                        base: Reg::RBX,
                        disp: 0,
                    },
                },
                3,
            )),
            _ => None,
        }
    }

    // ========================================
    // 0x48 0xFF — INC/DEC
    // ========================================

    fn decode_48_ff(&self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        let modrm = code[offset + 2];
        match modrm {
            // inc reg
            0xC0 => Some((
                ADeadOp::Inc {
                    dst: Operand::Reg(Reg::RAX),
                },
                3,
            )),
            0xC1 => Some((
                ADeadOp::Inc {
                    dst: Operand::Reg(Reg::RCX),
                },
                3,
            )),
            // dec reg
            0xC9 => Some((
                ADeadOp::Dec {
                    dst: Operand::Reg(Reg::RCX),
                },
                3,
            )),
            // inc [rbp+disp32]
            0x85 if offset + 6 < code.len() => {
                let disp = read_i32(code, offset + 3);
                Some((
                    ADeadOp::Inc {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                    },
                    7,
                ))
            }
            // dec [rbp+disp32]
            0x8D if offset + 6 < code.len() => {
                let disp = read_i32(code, offset + 3);
                Some((
                    ADeadOp::Dec {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp,
                        },
                    },
                    7,
                ))
            }
            _ => None,
        }
    }

    // ========================================
    // REX.WB (0x49) — R8-R15 destination
    // ========================================

    fn decode_rex_wb(&mut self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        let b1 = code[offset + 1];
        match b1 {
            0xB8 if offset + 10 <= code.len() => {
                let v = read_u64(code, offset + 2);
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Reg(Reg::R8),
                        src: Operand::Imm64(v),
                    },
                    10,
                ))
            }
            0xB9 if offset + 10 <= code.len() => {
                let v = read_u64(code, offset + 2);
                Some((
                    ADeadOp::Mov {
                        dst: Operand::Reg(Reg::R9),
                        src: Operand::Imm64(v),
                    },
                    10,
                ))
            }
            0x89 if offset + 2 < code.len() => match code[offset + 2] {
                0xC0 => Some((
                    ADeadOp::Mov {
                        dst: Operand::Reg(Reg::R8),
                        src: Operand::Reg(Reg::RAX),
                    },
                    3,
                )),
                0xC1 => Some((
                    ADeadOp::Mov {
                        dst: Operand::Reg(Reg::R9),
                        src: Operand::Reg(Reg::RAX),
                    },
                    3,
                )),
                _ => None,
            },
            _ => None,
        }
    }

    // ========================================
    // REX.WR (0x4C) — R8-R15 source
    // ========================================

    fn decode_rex_wr(&mut self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        let b1 = code[offset + 1];
        match b1 {
            // CMP
            0x39 if offset + 2 < code.len() => match code[offset + 2] {
                0xC1 => Some((
                    ADeadOp::Cmp {
                        left: Operand::Reg(Reg::RCX),
                        right: Operand::Reg(Reg::R8),
                    },
                    3,
                )),
                0x85 if offset + 6 < code.len() => {
                    let disp = read_i32(code, offset + 3);
                    Some((
                        ADeadOp::Cmp {
                            left: Operand::Mem {
                                base: Reg::RBP,
                                disp,
                            },
                            right: Operand::Reg(Reg::R8),
                        },
                        7,
                    ))
                }
                _ => None,
            },
            // MOV [rbp+disp8], R8/R9
            0x89 if offset + 3 < code.len() => match code[offset + 2] {
                0x45 => {
                    let disp = code[offset + 3] as i8 as i32;
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp,
                            },
                            src: Operand::Reg(Reg::R8),
                        },
                        4,
                    ))
                }
                0x4D => {
                    let disp = code[offset + 3] as i8 as i32;
                    Some((
                        ADeadOp::Mov {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp,
                            },
                            src: Operand::Reg(Reg::R9),
                        },
                        4,
                    ))
                }
                _ => None,
            },
            _ => None,
        }
    }

    // ========================================
    // 0x66 prefix (SSE movq)
    // ========================================

    fn decode_66_prefix(&self, code: &[u8], offset: usize) -> Option<(ADeadOp, usize)> {
        if offset + 4 < code.len() && code[offset + 1] == 0x48 && code[offset + 2] == 0x0F {
            match code[offset + 3] {
                // movq rax, xmm0
                0x7E if offset + 4 < code.len() && code[offset + 4] == 0xC0 => Some((
                    ADeadOp::MovQ {
                        dst: Reg::RAX,
                        src: Reg::XMM0,
                    },
                    5,
                )),
                // movq xmm1, rdx
                0x6E if offset + 4 < code.len() && code[offset + 4] == 0xCA => Some((
                    ADeadOp::MovQ {
                        dst: Reg::XMM1,
                        src: Reg::RDX,
                    },
                    5,
                )),
                // movq xmm0, rax
                0x6E if offset + 4 < code.len() && code[offset + 4] == 0xC0 => Some((
                    ADeadOp::MovQ {
                        dst: Reg::XMM0,
                        src: Reg::RAX,
                    },
                    5,
                )),
                _ => None,
            }
        } else {
            None
        }
    }

    // ========================================
    // Helpers
    // ========================================

    fn get_or_create_label(&mut self, target_offset: usize) -> Label {
        if let Some(&label) = self.target_labels.get(&target_offset) {
            label
        } else {
            let label = Label(self.label_counter);
            self.label_counter += 1;
            self.target_labels.insert(target_offset, label);
            label
        }
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn read_i32(code: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        code[offset],
        code[offset + 1],
        code[offset + 2],
        code[offset + 3],
    ])
}

#[inline]
fn read_u64(code: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        code[offset],
        code[offset + 1],
        code[offset + 2],
        code[offset + 3],
        code[offset + 4],
        code[offset + 5],
        code[offset + 6],
        code[offset + 7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_prologue() {
        let mut dec = Decoder::new();
        let code = vec![0x55, 0x48, 0x89, 0xE5, 0x5D, 0xC3];
        let ops = dec.decode_all(&code);

        assert_eq!(
            ops[0],
            ADeadOp::Push {
                src: Operand::Reg(Reg::RBP)
            }
        );
        assert_eq!(
            ops[1],
            ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBP),
                src: Operand::Reg(Reg::RSP),
            }
        );
        assert_eq!(ops[2], ADeadOp::Pop { dst: Reg::RBP });
        assert_eq!(ops[3], ADeadOp::Ret);
    }

    #[test]
    fn test_decode_xor_eax() {
        let mut dec = Decoder::new();
        let code = vec![0x31, 0xC0];
        let ops = dec.decode_all(&code);
        assert_eq!(
            ops[0],
            ADeadOp::Xor {
                dst: Reg::EAX,
                src: Reg::EAX
            }
        );
    }

    #[test]
    fn test_decode_mov_imm64() {
        let mut dec = Decoder::new();
        let mut code = vec![0x48, 0xB8];
        code.extend_from_slice(&42u64.to_le_bytes());
        let ops = dec.decode_all(&code);
        assert_eq!(
            ops[0],
            ADeadOp::Mov {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Imm64(42),
            }
        );
    }

    #[test]
    fn test_roundtrip_prologue() {
        // Encode → bytes → Decode → verify same ops
        let original_ops = vec![
            ADeadOp::Push {
                src: Operand::Reg(Reg::RBP),
            },
            ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBP),
                src: Operand::Reg(Reg::RSP),
            },
            ADeadOp::Xor {
                dst: Reg::EAX,
                src: Reg::EAX,
            },
            ADeadOp::Mov {
                dst: Operand::Reg(Reg::RSP),
                src: Operand::Reg(Reg::RBP),
            },
            ADeadOp::Pop { dst: Reg::RBP },
            ADeadOp::Ret,
        ];

        let mut encoder = super::super::encoder::Encoder::new();
        let result = encoder.encode_all(&original_ops);

        let mut decoder = Decoder::new();
        let decoded = decoder.decode_all(&result.code);

        assert_eq!(decoded.len(), original_ops.len());
        for (orig, dec) in original_ops.iter().zip(decoded.iter()) {
            assert_eq!(orig, dec);
        }
    }
}
