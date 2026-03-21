// ============================================================
// ISA Compiler — Array access and assignment
// ============================================================

use super::core::IsaCompiler;
use crate::frontend::ast::*;
use crate::isa::{ADeadOp, Operand, Reg};

impl IsaCompiler {
    /// Array index assignment: arr[i] = value
    pub(crate) fn emit_index_assign(&mut self, object: &Expr, index: &Expr, value: &Expr) {
        if let Expr::Variable(name) = object {
            if let Some(&base_offset) = self.variables.get(name.as_str()) {
                // Constant index — direct addressing
                if let Expr::Number(idx) = index {
                    let elem_offset = base_offset - (*idx as i32 * 8);
                    self.emit_expression(value);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp: elem_offset,
                        },
                        src: Operand::Reg(Reg::RAX),
                    });
                } else {
                    // Dynamic index
                    self.emit_expression(value);
                    self.ir.emit(ADeadOp::Push {
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.emit_expression(index);
                    self.ir.emit(ADeadOp::Shl {
                        dst: Reg::RAX,
                        amount: 3,
                    });
                    // RAX = i*8, need base_offset - i*8
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Imm32(base_offset),
                    });
                    self.ir.emit(ADeadOp::Sub {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBP),
                    });
                    self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RAX,
                            disp: 0,
                        },
                        src: Operand::Reg(Reg::RCX),
                    });
                }
            }
        } else {
            // Non-variable object — evaluate as pointer
            self.emit_expression(value);
            self.ir.emit(ADeadOp::Push {
                src: Operand::Reg(Reg::RAX),
            });
            self.emit_expression(index);
            self.ir.emit(ADeadOp::Push {
                src: Operand::Reg(Reg::RAX),
            });
            self.emit_expression(object);
            self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
            self.ir.emit(ADeadOp::Shl {
                dst: Reg::RBX,
                amount: 3,
            });
            self.ir.emit(ADeadOp::Add {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Reg(Reg::RBX),
            });
            self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
            self.ir.emit(ADeadOp::Mov {
                dst: Operand::Mem {
                    base: Reg::RAX,
                    disp: 0,
                },
                src: Operand::Reg(Reg::RCX),
            });
        }
    }

    /// Array index access: arr[i]
    pub(crate) fn emit_index_access(&mut self, object: &Box<Expr>, index: &Box<Expr>) {
        if let Expr::Variable(name) = object.as_ref() {
            if let Some(&base_offset) = self.variables.get(name.as_str()) {
                // Constant index — direct addressing
                if let Expr::Number(idx) = index.as_ref() {
                    let elem_offset = base_offset - (*idx as i32 * 8);
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Mem {
                            base: Reg::RBP,
                            disp: elem_offset,
                        },
                    });
                } else {
                    // Dynamic index
                    self.emit_expression(index);
                    self.ir.emit(ADeadOp::Shl {
                        dst: Reg::RAX,
                        amount: 3,
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RBX),
                        src: Operand::Reg(Reg::RAX),
                    });
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Imm32(base_offset),
                    });
                    self.ir.emit(ADeadOp::Sub {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBX),
                    });
                    self.ir.emit(ADeadOp::Add {
                        dst: Operand::Reg(Reg::RAX),
                        src: Operand::Reg(Reg::RBP),
                    });
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
            } else {
                // Unknown variable — evaluate as pointer
                self.emit_expression(index);
                self.ir.emit(ADeadOp::Push {
                    src: Operand::Reg(Reg::RAX),
                });
                self.emit_expression(object);
                self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
                self.ir.emit(ADeadOp::Shl {
                    dst: Reg::RBX,
                    amount: 3,
                });
                self.ir.emit(ADeadOp::Add {
                    dst: Operand::Reg(Reg::RAX),
                    src: Operand::Reg(Reg::RBX),
                });
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
        } else {
            // Non-variable object
            self.emit_expression(index);
            self.ir.emit(ADeadOp::Push {
                src: Operand::Reg(Reg::RAX),
            });
            self.emit_expression(object);
            self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
            self.ir.emit(ADeadOp::Shl {
                dst: Reg::RBX,
                amount: 3,
            });
            self.ir.emit(ADeadOp::Add {
                dst: Operand::Reg(Reg::RAX),
                src: Operand::Reg(Reg::RBX),
            });
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
}
