// ============================================================
// ISA Compiler — Helper functions (print, assign, call, etc.)
// ============================================================

use super::core::{IsaCompiler, Target};
use crate::backend::cpu::iat_registry;
use crate::frontend::ast::*;
use crate::isa::{ADeadOp, CallTarget, Operand, Reg};

impl IsaCompiler {
    pub(crate) fn emit_print(&mut self, expr: &Expr) {
        if let Expr::String(s) = expr {
            let processed = s
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r");
            if !self.strings.contains(&processed) {
                self.strings.push(processed.clone());
            }
            let string_addr = self.get_string_address(&processed);

            match self.target {
                Target::Linux => {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Imm32(1),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDI),
                        src: Operand::Imm32(1),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RSI),
                        src: Operand::Imm64(string_addr),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDX),
                        src: Operand::Imm32(processed.len() as i32),
                    });
                    self.ir.emit(ADeadOp::Syscall);
                }
                Target::Windows | Target::Raw => {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RCX),
                        src: Operand::Imm64(string_addr),
                    });
                    self.emit_call_printf();
                }
            }
        } else {
            self.emit_expression(expr);

            let is_float = matches!(expr, Expr::Float(_));
            let is_integer = matches!(
                expr,
                Expr::Number(_)
                    | Expr::Variable(_)
                    | Expr::BinaryOp { .. }
                    | Expr::Bool(_)
                    | Expr::Call { .. }
                    | Expr::IntCast(_)
                    | Expr::Len(_)
            );

            match self.target {
                Target::Windows | Target::Raw => {
                    if is_float {
                        let fmt_addr = self.get_string_address("%.2f");
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::MovQ {
                            dst: Reg::XMM1,
                            src: Reg::RDX,
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Imm64(fmt_addr),
                        });
                        self.emit_call_printf();
                    } else if is_integer {
                        let fmt_addr = self.get_string_address("%d");
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Imm64(fmt_addr),
                        });
                        self.emit_call_printf();
                    } else {
                        let fmt_addr = self.get_string_address("%s");
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RDX),
                            src: Operand::Reg(Reg::RAX),
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Imm64(fmt_addr),
                        });
                        self.emit_call_printf();
                    }
                }
                Target::Linux => {}
            }
        }
    }

    pub(crate) fn emit_println(&mut self, expr: &Expr) {
        self.emit_print(expr);
        let newline = "\n".to_string();
        if !self.strings.contains(&newline) {
            self.strings.push(newline);
        }
        let nl_addr = self.get_string_address("\n");
        match self.target {
            Target::Windows | Target::Raw => {
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RCX),
                    src: Operand::Imm64(nl_addr),
                });
                self.emit_call_printf();
            }
            Target::Linux => {}
        }
    }

    pub(crate) fn emit_print_num(&mut self, expr: &Expr) {
        self.emit_expression(expr);
        let fmt_addr = self.get_string_address("%d");
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RDX),
            src: Operand::Reg(Reg::RAX),
        });
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RCX),
            src: Operand::Imm64(fmt_addr),
        });
        self.emit_call_printf();
    }

    pub(crate) fn emit_call_printf(&mut self) {
        self.emit_call_iat("printf");
    }

    /// Emit a call to an IAT-imported function by name.
    /// Looks up the IAT slot RVA from the registry using assumed idata_rva=0x2000.
    pub(crate) fn emit_call_iat(&mut self, func_name: &str) {
        let assumed_idata_rva: u32 = 0x2000;
        let idata_result = iat_registry::build_idata(assumed_idata_rva, &[]);
        let slot = iat_registry::slot_for_function(func_name)
            .unwrap_or_else(|| panic!("IAT function not found: {}", func_name));
        let iat_rva = idata_result.slot_to_iat_rva[slot];

        self.ir.emit(ADeadOp::Sub {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm8(32),
        });
        self.ir.emit(ADeadOp::CallIAT { iat_rva });
        self.ir.emit(ADeadOp::Add {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm8(32),
        });
    }

    pub(crate) fn emit_assign(&mut self, name: &str, value: &Expr) {
        // Optimization: x = x + 1 → inc, x = x - 1 → dec
        if let Some(&offset) = self.variables.get(name) {
            if let Expr::BinaryOp { op, left, right } = value {
                if let Expr::Variable(var_name) = left.as_ref() {
                    if var_name == name {
                        if let Expr::Number(n) = right.as_ref() {
                            if *n == 1 {
                                match op {
                                    BinOp::Add => {
                                        self.ir.emit(ADeadOp::Inc {
                                            dst: Operand::Mem {
                                                base: Reg::RBP,
                                                disp: offset,
                                            },
                                        });
                                        return;
                                    }
                                    BinOp::Sub => {
                                        self.ir.emit(ADeadOp::Dec {
                                            dst: Operand::Mem {
                                                base: Reg::RBP,
                                                disp: offset,
                                            },
                                        });
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        self.emit_expression(value);

        let offset = if let Some(&off) = self.variables.get(name) {
            off
        } else {
            let off = self.stack_offset;
            self.variables.insert(name.to_string(), off);
            self.stack_offset -= 8;
            off
        };

        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Mem {
                base: Reg::RBP,
                disp: offset,
            },
            src: Operand::Reg(Reg::RAX),
        });
    }

    pub(crate) fn emit_call(&mut self, name: &str, args: &[Expr]) {
        let arg_count = args.len().min(4);

        for arg in args.iter().take(4) {
            self.emit_expression(arg);
            self.ir.emit(ADeadOp::Push {
                src: Operand::Reg(Reg::RAX),
            });
        }

        for i in (0..arg_count).rev() {
            let dst = self.arg_register(i);
            self.ir.emit(ADeadOp::Pop { dst });
        }

        // Check IAT registry for imported functions
        let iat_name = match name {
            "printf" | "std::printf" => Some("printf"),
            "scanf" | "std::scanf" => Some("scanf"),
            "malloc" => Some("malloc"),
            "free" => Some("free"),
            _ => {
                if iat_registry::slot_for_function(name).is_some() {
                    Some(name)
                } else {
                    None
                }
            }
        };
        if let Some(iat_func) = iat_name {
            self.emit_call_iat(iat_func);
            return;
        }

        self.ir.emit(ADeadOp::Sub {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm8(32),
        });

        if let Some(func) = self.functions.get(name) {
            let label = func.label;
            self.ir.emit(ADeadOp::Call {
                target: CallTarget::Relative(label),
            });
        } else {
            self.ir.emit(ADeadOp::Call {
                target: CallTarget::Name(name.to_string()),
            });
        }

        self.ir.emit(ADeadOp::Add {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm8(32),
        });
    }

    pub(crate) fn emit_input(&mut self) {
        let input_offset = self.stack_offset;
        self.stack_offset -= 8;

        self.ir.emit(ADeadOp::Xor {
            dst: Reg::RAX,
            src: Reg::RAX,
        });
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Mem {
                base: Reg::RBP,
                disp: input_offset,
            },
            src: Operand::Reg(Reg::RAX),
        });

        let fmt_addr = self.get_string_address("%d");
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RCX),
            src: Operand::Imm64(fmt_addr),
        });
        self.ir.emit(ADeadOp::Lea {
            dst: Reg::RDX,
            src: Operand::Mem {
                base: Reg::RBP,
                disp: input_offset,
            },
        });

        // Call scanf via dynamic IAT lookup
        self.emit_call_iat("scanf");

        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RAX),
            src: Operand::Mem {
                base: Reg::RBP,
                disp: input_offset,
            },
        });
    }

    pub(crate) fn emit_reg_assign(&mut self, reg_name: &str, value: &Expr) {
        self.emit_expression(value);
        if let Some(reg) = Self::string_to_reg(reg_name) {
            if reg.is_control() {
                let cr_num = match reg {
                    Reg::CR0 => 0,
                    Reg::CR2 => 2,
                    Reg::CR3 => 3,
                    Reg::CR4 => 4,
                    _ => 0,
                };
                self.ir.emit(ADeadOp::MovToCr {
                    cr: cr_num,
                    src: Reg::RAX,
                });
            } else if reg.is_segment() {
                let seg_code: u8 = match reg {
                    Reg::DS => 0xD8,
                    Reg::ES => 0xC0,
                    Reg::SS => 0xD0,
                    Reg::FS => 0xE0,
                    Reg::GS => 0xE8,
                    _ => 0xD8,
                };
                self.ir.emit(ADeadOp::RawBytes(vec![0x8E, seg_code]));
            } else {
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(reg),
                    src: Operand::Reg(Reg::RAX),
                });
            }
        }
    }

    pub(crate) fn emit_mem_write(&mut self, addr: &Expr, value: &Expr) {
        self.emit_expression(value);
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RAX),
        });
        self.emit_expression(addr);
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RBX),
            src: Operand::Reg(Reg::RAX),
        });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
        self.ir.emit(ADeadOp::RawBytes(vec![0x48, 0x89, 0x03]));
    }

    pub(crate) fn emit_port_out(&mut self, port: &Expr, value: &Expr) {
        self.emit_expression(value);
        match port {
            Expr::Number(p) if *p >= 0 && *p <= 255 => {
                self.ir.emit(ADeadOp::OutByte {
                    port: Operand::Imm8(*p as i8),
                    src: Operand::Reg(Reg::AL),
                });
            }
            _ => {
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                self.emit_expression(port);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RDX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                self.ir.emit(ADeadOp::OutByte {
                    port: Operand::Reg(Reg::DX),
                    src: Operand::Reg(Reg::AL),
                });
            }
        }
    }

    pub(crate) fn emit_compound_assign(&mut self, name: &str, op: &CompoundOp, value: &Expr) {
        if let Some(&offset) = self.variables.get(name) {
            self.emit_expression(value);
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBX),
                src: Operand::Reg(Reg::RAX),
            });
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Mem {
                    base: Reg::RBP,
                    disp: offset,
                },
            });
            match op {
                CompoundOp::AddAssign => self.ir.emit(ADeadOp::Add {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Reg(Reg::RBX),
                }),
                CompoundOp::SubAssign => self.ir.emit(ADeadOp::Sub {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Reg(Reg::RBX),
                }),
                CompoundOp::MulAssign => self.ir.emit(ADeadOp::Mul {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                }),
                CompoundOp::DivAssign => self.ir.emit(ADeadOp::Div { src: Reg::RBX }),
                CompoundOp::AndAssign => self.ir.emit(ADeadOp::And {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                }),
                CompoundOp::OrAssign => self.ir.emit(ADeadOp::Or {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                }),
                CompoundOp::XorAssign => self.ir.emit(ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RBX,
                }),
                CompoundOp::ShlAssign => {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RCX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    self.ir.emit(ADeadOp::ShlCl { dst: Reg::RAX });
                }
                CompoundOp::ShrAssign => {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RCX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    self.ir.emit(ADeadOp::ShrCl { dst: Reg::RAX });
                }
                CompoundOp::ModAssign => {
                    self.ir.emit(ADeadOp::Div { src: Reg::RBX });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RDX),
                    });
                }
            }
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Mem {
                    base: Reg::RBP,
                    disp: offset,
                },
                src: Operand::Reg(Reg::RAX),
            });
        }
    }
}
