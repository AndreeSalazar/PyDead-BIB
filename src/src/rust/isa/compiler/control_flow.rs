// ============================================================
// ISA Compiler — Control flow (if, while, for, conditions)
// ============================================================

use super::core::IsaCompiler;
use crate::frontend::ast::*;
use crate::isa::{ADeadOp, Condition, Operand, Reg};

impl IsaCompiler {
    pub(crate) fn emit_if(
        &mut self,
        condition: &Expr,
        then_body: &[Stmt],
        else_body: Option<&[Stmt]>,
    ) {
        self.emit_condition(condition);
        self.ir.emit(ADeadOp::Test {
            left: Reg::RAX,
            right: Reg::RAX,
        });

        let else_label = self.ir.new_label();
        self.ir.emit(ADeadOp::Jcc {
            cond: Condition::Equal,
            target: else_label,
        });

        for stmt in then_body {
            self.emit_statement(stmt);
        }

        if let Some(else_stmts) = else_body {
            let end_label = self.ir.new_label();
            self.ir.emit(ADeadOp::Jmp { target: end_label });
            self.ir.emit(ADeadOp::Label(else_label));
            for stmt in else_stmts {
                self.emit_statement(stmt);
            }
            self.ir.emit(ADeadOp::Label(end_label));
        } else {
            self.ir.emit(ADeadOp::Label(else_label));
        }
    }

    pub(crate) fn emit_while(&mut self, condition: &Expr, body: &[Stmt]) {
        let loop_start = self.ir.new_label();
        let loop_end = self.ir.new_label();

        self.loop_stack.push((loop_end, loop_start));

        self.ir.emit(ADeadOp::Label(loop_start));
        self.emit_condition(condition);
        self.ir.emit(ADeadOp::Test {
            left: Reg::RAX,
            right: Reg::RAX,
        });
        self.ir.emit(ADeadOp::Jcc {
            cond: Condition::Equal,
            target: loop_end,
        });

        for stmt in body {
            self.emit_statement(stmt);
        }

        self.ir.emit(ADeadOp::Jmp { target: loop_start });
        self.ir.emit(ADeadOp::Label(loop_end));

        self.loop_stack.pop();
    }

    pub(crate) fn emit_for(&mut self, var: &str, start: &Expr, end: &Expr, body: &[Stmt]) {
        self.emit_expression(start);
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RCX),
            src: Operand::Reg(Reg::RAX),
        });
        self.emit_expression(end);
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::R8),
            src: Operand::Reg(Reg::RAX),
        });

        let var_offset = self.stack_offset;
        self.variables.insert(var.to_string(), var_offset);
        self.stack_offset -= 8;

        let loop_start = self.ir.new_label();
        let loop_end = self.ir.new_label();

        self.loop_stack.push((loop_end, loop_start));

        self.ir.emit(ADeadOp::Label(loop_start));
        self.ir.emit(ADeadOp::Cmp {
            left: Operand::Reg(Reg::RCX),
            right: Operand::Reg(Reg::R8),
        });
        self.ir.emit(ADeadOp::Jcc {
            cond: Condition::GreaterEq,
            target: loop_end,
        });

        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Mem {
                base: Reg::RBP,
                disp: var_offset,
            },
            src: Operand::Reg(Reg::RCX),
        });

        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RCX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R8),
        });

        for stmt in body {
            self.emit_statement(stmt);
        }

        self.ir.emit(ADeadOp::Pop { dst: Reg::R8 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
        self.ir.emit(ADeadOp::Inc {
            dst: Operand::Reg(Reg::RCX),
        });
        self.ir.emit(ADeadOp::Jmp { target: loop_start });
        self.ir.emit(ADeadOp::Label(loop_end));

        self.loop_stack.pop();
    }

    pub(crate) fn emit_return(&mut self, expr: Option<&Expr>) {
        if let Some(e) = expr {
            self.emit_expression(e);
        } else {
            self.ir.emit(ADeadOp::Xor {
                dst: Reg::EAX,
                src: Reg::EAX,
            });
        }
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Reg(Reg::RBP),
        });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBP });
        self.ir.emit(ADeadOp::Ret);
    }

    pub(crate) fn emit_condition(&mut self, expr: &Expr) {
        match expr {
            Expr::Comparison { op, left, right } => {
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

                self.ir.emit(ADeadOp::Cmp {
                    left: Operand::Reg(Reg::RAX),
                    right: Operand::Reg(Reg::RBX),
                });

                let cond = match op {
                    CmpOp::Eq => Condition::Equal,
                    CmpOp::Ne => Condition::NotEqual,
                    CmpOp::Lt => Condition::Less,
                    CmpOp::Le => Condition::LessEq,
                    CmpOp::Gt => Condition::Greater,
                    CmpOp::Ge => Condition::GreaterEq,
                };
                self.ir.emit(ADeadOp::SetCC { cond, dst: Reg::AL });
                self.ir.emit(ADeadOp::MovZx {
                    dst: Reg::RAX,
                    src: Reg::AL,
                });
            }
            Expr::Bool(b) => {
                let val = if *b { 1 } else { 0 };
                self.ir.emit(ADeadOp::Mov {
                    dst: Operand::Reg(Reg::EAX),
                    src: Operand::Imm32(val),
                });
            }
            _ => self.emit_expression(expr),
        }
    }
}
