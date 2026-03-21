// ============================================================
// ISA Compiler — Statement emission (Stmt::*)
// ============================================================

use super::core::IsaCompiler;
use crate::frontend::ast::*;
use crate::isa::{ADeadOp, Condition, Operand, Reg};

impl IsaCompiler {
    pub(crate) fn emit_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Print(expr) => self.emit_print(expr),
            Stmt::Println(expr) => self.emit_println(expr),
            Stmt::PrintNum(expr) => self.emit_print_num(expr),
            Stmt::Assign { name, value } => self.emit_assign(name, value),
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.emit_if(condition, then_body, else_body.as_deref());
            }
            Stmt::While { condition, body } => self.emit_while(condition, body),
            Stmt::For {
                var,
                start,
                end,
                body,
            } => self.emit_for(var, start, end, body),
            Stmt::Return(expr) => self.emit_return(expr.as_ref()),
            Stmt::Expr(expr) => {
                self.emit_expression(expr);
            }
            Stmt::Pass => {}

            // OS-LEVEL
            Stmt::Cli => {
                self.ir.emit(ADeadOp::Cli);
            }
            Stmt::Sti => {
                self.ir.emit(ADeadOp::Sti);
            }
            Stmt::Hlt => {
                self.ir.emit(ADeadOp::Hlt);
            }
            Stmt::Iret => {
                self.ir.emit(ADeadOp::Iret);
            }
            Stmt::Cpuid => {
                self.ir.emit(ADeadOp::Cpuid);
            }
            Stmt::IntCall { vector } => {
                self.ir.emit(ADeadOp::Int { vector: *vector });
            }
            Stmt::RegAssign { reg_name, value } => {
                self.emit_reg_assign(reg_name, value);
            }
            Stmt::MemWrite { addr, value } => {
                self.emit_mem_write(addr, value);
            }
            Stmt::PortOut { port, value } => {
                self.emit_port_out(port, value);
            }
            Stmt::RawBlock { bytes } => {
                self.ir.emit(ADeadOp::RawBytes(bytes.clone()));
            }
            Stmt::OrgDirective { address } => {
                self.base_address = *address;
            }
            Stmt::AlignDirective { alignment } => {
                let align = *alignment as usize;
                if align > 0 {
                    self.ir.emit(ADeadOp::RawBytes(vec![0x90]));
                }
            }
            Stmt::FarJump { selector, offset } => {
                self.ir.emit(ADeadOp::FarJmp {
                    selector: *selector,
                    offset: *offset,
                });
            }

            // OOP field assignment
            Stmt::FieldAssign {
                object,
                field,
                value,
            } => {
                let var_name = match object {
                    Expr::This => format!("self.{}", field),
                    Expr::Variable(obj_name) => format!("{}.{}", obj_name, field),
                    _ => format!("__field.{}", field),
                };
                self.emit_assign(&var_name, value);
            }

            // VarDecl
            Stmt::VarDecl {
                var_type,
                name,
                value,
            } => {
                // IMPORTANT: The ISA compiler uses 8-byte (64-bit) slots for ALL
                // stack variables, including array elements. So arrays always need
                // count * 8 bytes, regardless of element type size.
                let alloc_size = match var_type {
                    Type::Array(_, Some(n)) => (*n as i32) * 8,
                    Type::Array(_, None) => 8,
                    _ => 8,
                };

                if let Some(val) = value {
                    self.emit_assign(name, val);
                } else {
                    let offset = self.stack_offset;
                    self.variables.insert(name.clone(), offset);
                    self.stack_offset -= alloc_size;

                    if alloc_size <= 8 {
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                        self.ir.emit(ADeadOp::Mov {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                            src: Operand::Reg(Reg::RAX),
                        });
                    } else {
                        let num_qwords = (alloc_size + 7) / 8;
                        self.ir.emit(ADeadOp::Xor {
                            dst: Reg::RAX,
                            src: Reg::RAX,
                        });
                        for i in 0..num_qwords {
                            let elem_offset = offset - (i * 8);
                            self.ir.emit(ADeadOp::Mov {
                                dst: Operand::Mem {
                                    base: Reg::RBP,
                                    disp: elem_offset,
                                },
                                src: Operand::Reg(Reg::RAX),
                            });
                        }
                    }
                }
            }
            Stmt::CompoundAssign { name, op, value } => {
                self.emit_compound_assign(name, op, value);
            }

            // IndexAssign — delegated to arrays module
            Stmt::IndexAssign {
                object,
                index,
                value,
            } => {
                self.emit_index_assign(object, index, value);
            }

            Stmt::Increment {
                name,
                is_pre: _,
                is_increment,
            } => {
                if let Some(&offset) = self.variables.get(name.as_str()) {
                    if *is_increment {
                        self.ir.emit(ADeadOp::Inc {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    } else {
                        self.ir.emit(ADeadOp::Dec {
                            dst: Operand::Mem {
                                base: Reg::RBP,
                                disp: offset,
                            },
                        });
                    }
                }
            }
            Stmt::DoWhile { body, condition } => {
                let loop_start = self.ir.new_label();
                let loop_end = self.ir.new_label();

                self.loop_stack.push((loop_end, loop_start));
                self.ir.emit(ADeadOp::Label(loop_start));

                for s in body {
                    self.emit_statement(s);
                }

                self.emit_expression(condition);
                self.ir.emit(ADeadOp::Cmp {
                    left: Operand::Reg(Reg::RAX),
                    right: Operand::Imm32(0),
                });
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::NotEqual,
                    target: loop_start,
                });
                self.ir.emit(ADeadOp::Label(loop_end));
                self.loop_stack.pop();
            }

            // Labels and jumps
            Stmt::LabelDef { name } => {
                let label = self.get_or_create_named_label(name);
                self.ir.emit(ADeadOp::Label(label));
            }
            Stmt::JumpTo { label: label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jmp { target: label });
            }
            Stmt::JumpIfZero { label: label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::Equal,
                    target: label,
                });
            }
            Stmt::JumpIfNotZero { label: label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::NotEqual,
                    target: label,
                });
            }
            Stmt::JumpIfCarry { label: label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::Less,
                    target: label,
                });
            }
            Stmt::JumpIfNotCarry { label: label_name } => {
                let label = self.get_or_create_named_label(label_name);
                self.ir.emit(ADeadOp::Jcc {
                    cond: Condition::GreaterEq,
                    target: label,
                });
            }
            Stmt::DataBytes { bytes } => {
                self.ir.emit(ADeadOp::RawBytes(bytes.clone()));
            }
            Stmt::DataWords { words } => {
                let mut bytes = Vec::new();
                for w in words {
                    bytes.extend_from_slice(&w.to_le_bytes());
                }
                self.ir.emit(ADeadOp::RawBytes(bytes));
            }
            Stmt::DataDwords { dwords } => {
                let mut bytes = Vec::new();
                for d in dwords {
                    bytes.extend_from_slice(&d.to_le_bytes());
                }
                self.ir.emit(ADeadOp::RawBytes(bytes));
            }
            Stmt::TimesDirective { count, byte } => {
                let bytes = vec![*byte; *count];
                self.ir.emit(ADeadOp::RawBytes(bytes));
            }

            Stmt::Break => {
                if let Some(&(break_label, _)) = self.loop_stack.last() {
                    self.ir.emit(ADeadOp::Jmp {
                        target: break_label,
                    });
                }
            }
            Stmt::Continue => {
                if let Some(&(_, continue_label)) = self.loop_stack.last() {
                    self.ir.emit(ADeadOp::Jmp {
                        target: continue_label,
                    });
                }
            }

            _ => {}
        }
    }
}
