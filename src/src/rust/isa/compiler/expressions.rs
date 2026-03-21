// ============================================================
// ISA Compiler — Expression emission (Expr::*)
// ============================================================

use super::core::IsaCompiler;
use crate::frontend::ast::*;
use crate::isa::{ADeadOp, Condition, Operand, Reg};

impl IsaCompiler {
    pub(crate) fn emit_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Number(n) => {
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(*n as u64),
                });
            }
            Expr::Float(f) => {
                let bits = f.to_bits();
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(bits),
                });
            }
            Expr::Bool(b) => {
                let val = if *b { 1u64 } else { 0u64 };
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(val),
                });
            }
            Expr::Variable(name) => {
                if let Some(&offset) = self.variables.get(name) {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBP,
                            disp: offset,
                        },
                    });
                } else {
                    self.ir.emit(ADeadOp::Xor {
                        dst: Reg::EAX,
                        src: Reg::EAX,
                    });
                }
            }
            Expr::BinaryOp { op, left, right } => {
                self.emit_expression(left);
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                self.emit_expression(right);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });

                match op {
                    BinOp::Add => self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    }),
                    BinOp::Sub => self.ir.emit(ADeadOp::Sub {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    }),
                    BinOp::Mul => self.ir.emit(ADeadOp::Mul {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BinOp::Div => self.ir.emit(ADeadOp::Div { src: Reg::RBX }),
                    BinOp::Mod => {
                        self.ir.emit(ADeadOp::Div { src: Reg::RBX });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Reg(Reg::RDX),
                        });
                    }
                    BinOp::And => self.ir.emit(ADeadOp::And {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BinOp::Or => self.ir.emit(ADeadOp::Or {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                }
            }
            Expr::UnaryOp { op, expr: inner } => {
                self.emit_expression(inner);
                match op {
                    UnaryOp::Neg => self.ir.emit(ADeadOp::Neg { dst: Reg::RAX }),
                    UnaryOp::Not => self.ir.emit(ADeadOp::Not { dst: Reg::RAX }),
                }
            }
            Expr::Call { name, args } => {
                self.emit_call(name, args);
            }
            Expr::Comparison { .. } => self.emit_condition(expr),
            Expr::Input => {
                self.emit_input();
            }
            Expr::IntCast(inner) => {
                self.emit_expression(inner);
            }
            Expr::FloatCast(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::CvtSi2Sd {
                    dst: Reg::XMM0,
                    src: Reg::RAX,
                });
                self.ir.emit(ADeadOp::MovQ {
                    dst: Reg::RAX,
                    src: Reg::XMM0,
                });
            }
            Expr::BoolCast(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::Test {
                    left: Reg::RAX,
                    right: Reg::RAX,
                });
                self.ir.emit(ADeadOp::SetCC {
                    cond: Condition::NotEqual,
                    dst: Reg::AL,
                });
                self.ir.emit(ADeadOp::MovZx {
                    dst: Reg::RAX,
                    src: Reg::AL,
                });
            }
            Expr::RegRead { reg_name } => {
                if let Some(reg) = Self::string_to_reg(reg_name) {
                    if reg.is_control() {
                        let cr_num = match reg {
                            Reg::CR0 => 0,
                            Reg::CR2 => 2,
                            Reg::CR3 => 3,
                            Reg::CR4 => 4,
                            _ => 0,
                        };
                        self.ir.emit(ADeadOp::MovFromCr {
                            cr: cr_num,
                            dst: Reg::RAX,
                        });
                    } else {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RAX),
                            src: Operand::Reg(reg),
                        });
                    }
                }
            }
            Expr::MemRead { addr } => {
                self.emit_expression(addr);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Mem {
                        base: Reg::RBX,
                        disp: 0,
                    },
                });
            }
            Expr::PortIn { port } => match port.as_ref() {
                Expr::Number(p) if *p >= 0 && *p <= 255 => {
                    self.ir.emit(ADeadOp::InByte {
                        port: Operand::Imm8(*p as i8),
                    });
                    self.ir.emit(ADeadOp::MovZx {
                        dst: Reg::RAX,
                        src: Reg::AL,
                    });
                }
                _ => {
                    self.emit_expression(port);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RDX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::InByte {
                        port: Operand::Reg(Reg::DX),
                    });
                    self.ir.emit(ADeadOp::MovZx {
                        dst: Reg::RAX,
                        src: Reg::AL,
                    });
                }
            },
            Expr::CpuidExpr => {
                self.ir.emit(ADeadOp::Cpuid);
            }
            Expr::BitwiseOp { op, left, right } => {
                self.emit_expression(left);
                if let Some(temp) = self.temp_alloc.alloc() {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(temp),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.emit_expression(right);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(temp),
                    });
                    self.temp_alloc.free(temp);
                } else {
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.emit_expression(right);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
                }
                match op {
                    BitwiseOp::And => self.ir.emit(ADeadOp::And {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BitwiseOp::Or => self.ir.emit(ADeadOp::Or {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BitwiseOp::Xor => self.ir.emit(ADeadOp::Xor {
                        dst: Reg::RAX,
                        src: Reg::RBX,
                    }),
                    BitwiseOp::LeftShift => {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Reg(Reg::RBX),
                        });
                        self.ir.emit(ADeadOp::ShlCl { dst: Reg::RAX });
                    }
                    BitwiseOp::RightShift => {
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Reg(Reg::RCX),
                            src: Operand::Reg(Reg::RBX),
                        });
                        self.ir.emit(ADeadOp::ShrCl { dst: Reg::RAX });
                    }
                }
            }
            Expr::BitwiseNot(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::BitwiseNot { dst: Reg::RAX });
            }
            Expr::PreIncrement(inner) | Expr::PostIncrement(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::Inc {
                    dst: Operand::Reg(Reg::RAX),
                });
            }
            Expr::PreDecrement(inner) | Expr::PostDecrement(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::Dec {
                    dst: Operand::Reg(Reg::RAX),
                });
            }
            Expr::Nullptr | Expr::Null => {
                self.ir.emit(ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RAX,
                });
            }
            Expr::LabelAddr { label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::LabelAddrRef {
                    label,
                    size: 4,
                    base_addr: self.base_address as u32,
                });
            }
            Expr::String(s) => {
                let processed = s
                    .replace("\\n", "\n")
                    .replace("\\t", "\t")
                    .replace("\\r", "\r");
                if !self.strings.contains(&processed) {
                    self.strings.push(processed.clone());
                }
                let addr = self.get_string_address(&processed);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(addr),
                });
            }
            Expr::FieldAccess { object, field } => {
                let var_name = match object.as_ref() {
                    Expr::This => format!("self.{}", field),
                    Expr::Variable(obj_name) => format!("{}.{}", obj_name, field),
                    _ => format!("__field.{}", field),
                };
                if let Some(&offset) = self.variables.get(&var_name) {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBP,
                            disp: offset,
                        },
                    });
                } else {
                    self.ir.emit(ADeadOp::Xor {
                        dst: Reg::RAX,
                        src: Reg::RAX,
                    });
                }
            }
            Expr::MethodCall {
                object,
                method: _,
                args,
            } => {
                // Primary resolution is in cpp_to_ir (converts to Expr::Call).
                // This fallback handles any remaining unresolved MethodCalls.
                self.emit_expression(object);
                for arg in args.iter() {
                    self.emit_expression(arg);
                }
                self.ir.emit(ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RAX,
                });
            }
            Expr::ArrowAccess {
                pointer: ptr_expr,
                field,
            } => {
                // If the pointer is 'this', look up 'this.fieldname' as a flat variable
                let flat_name = match ptr_expr.as_ref() {
                    Expr::Variable(n) => format!("{}.{}", n, field),
                    _ => format!("this.{}", field),
                };
                if let Some(&offset) = self.variables.get(&flat_name) {
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBP,
                            disp: offset,
                        },
                    });
                } else {
                    // Pointer dereference fallback
                    self.emit_expression(ptr_expr);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBX,
                            disp: 0,
                        },
                    });
                }
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let else_label = self.ir.new_label();
                let end_label = self.ir.new_label();

                self.emit_expression(condition);
                self.ir.emit(ADeadOp::Cmp {
                    left: Operand::Reg(Reg::RAX),
                    right: Operand::Imm32(0),
                });
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::Equal,
                    target: else_label,
                });

                self.emit_expression(then_expr);
                self.ir.emit(ADeadOp::Jmp { target: end_label });

                self.ir.emit(ADeadOp::Label(else_label));
                self.emit_expression(else_expr);

                self.ir.emit(ADeadOp::Label(end_label));
            }

            // Array access — delegated to arrays module
            Expr::Index { object, index } => {
                self.emit_index_access(object, index);
            }

            // Array literal
            Expr::Array(elems) => {
                let count = elems.len();
                let base_offset = self.stack_offset - (count as i32 * 8);
                for (i, elem) in elems.iter().enumerate() {
                    self.emit_expression(elem);
                    let elem_offset = base_offset + (i as i32 * 8);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp: elem_offset,
                        },
                        src: Operand::Reg(Reg::RAX),
                    });
                }
                self.stack_offset = base_offset;
                self.ir.emit(ADeadOp::Lea {
                    dst: Reg::RAX,
                    src: Operand::Mem {
                        base: Reg::RBP,
                        disp: base_offset,
                    },
                });
            }

            // Address-of
            Expr::AddressOf(inner) => {
                if let Expr::Variable(name) = inner.as_ref() {
                    if let Some(&offset) = self.variables.get(name.as_str()) {
                        self.ir.emit(ADeadOp::Lea {
                            dst: Reg::RAX,
                            src: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    } else {
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                    }
                } else {
                    self.ir.emit(ADeadOp::Xor {
                        dst: Reg::RAX,
                        src: Reg::RAX,
                    });
                }
            }

            // Dereference
            Expr::Deref(inner) => {
                self.emit_expression(inner);
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RBX),
                    src: Operand::Reg(Reg::RAX),
                });
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Mem {
                        base: Reg::RBX,
                        disp: 0,
                    },
                });
            }

            // SizeOf
            Expr::SizeOf(_) => {
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Imm64(8),
                });
            }

            // Cast
            Expr::Cast { expr: inner, .. } => {
                self.emit_expression(inner);
            }

            _ => {
                self.ir.emit(ADeadOp::Xor {
                    dst: Reg::RAX,
                    src: Reg::RAX,
                });
            }
        }
    }
}
