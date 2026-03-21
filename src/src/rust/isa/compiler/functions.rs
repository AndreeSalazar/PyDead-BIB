// ============================================================
// ISA Compiler — Function compilation, prologue/epilogue
// ============================================================

use super::core::{IsaCompiler, Target};
use crate::frontend::ast::*;
use crate::isa::reg_alloc::TempAllocator;
use crate::isa::{ADeadOp, Operand, Reg};

impl IsaCompiler {
    pub(crate) fn compile_function(&mut self, func: &Function) {
        self.current_function = Some(func.name.clone());
        self.variables.clear();
        self.stack_offset = -40;

        let is_interrupt = func.attributes.is_interrupt;
        let is_exception = func.attributes.is_exception;
        let is_naked = func.attributes.is_naked;

        if let Some(compiled) = self.functions.get(&func.name) {
            let label = compiled.label;
            self.ir.emit(ADeadOp::Label(label));
        }

        if is_interrupt || is_exception {
            self.emit_interrupt_prologue();
        } else if !is_naked {
            self.emit_prologue();

            for (i, param) in func.params.iter().enumerate() {
                let param_offset = if i <= 3 {
                    let off = self.stack_offset;
                    self.stack_offset -= 8;
                    off
                } else {
                    16 + ((i - 4) as i32 * 8)
                };
                self.variables.insert(param.name.clone(), param_offset);

                if i <= 3 {
                    let src_reg = match i {
                        0 => Reg::RCX,
                        1 => Reg::RDX,
                        2 => Reg::R8,
                        3 => Reg::R9,
                        _ => unreachable!(),
                    };
                    self.ir.emit(ADeadOp::Mov {
                        dst: Operand::Mem {
                            base: Reg::RBP,
                            disp: param_offset,
                        },
                        src: Operand::Reg(src_reg),
                    });
                }
            }
        }

        for stmt in &func.body {
            self.emit_statement(stmt);
        }

        if is_interrupt || is_exception {
            self.emit_interrupt_epilogue();
        } else if !is_naked {
            self.patch_prologue();
            self.emit_epilogue();
        }

        self.current_function = None;
    }

    pub(crate) fn compile_top_level(&mut self, stmts: &[Stmt]) {
        self.current_function = Some("__entry".to_string());
        self.variables.clear();
        self.stack_offset = -40;

        let is_raw = self.target == Target::Raw;

        if !is_raw {
            self.emit_prologue();
        }

        for stmt in stmts {
            self.emit_statement(stmt);
        }

        if !is_raw {
            self.patch_prologue();
            self.emit_epilogue();
        }
        self.current_function = None;
    }

    pub(crate) fn emit_prologue(&mut self) {
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RBP),
        });
        self.ir.emit(ADeadOp::Mov {
            dst: Operand::Reg(Reg::RBP),
            src: Operand::Reg(Reg::RSP),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RBX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R12),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RSI),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RDI),
        });
        self.prologue_sub_index = Some(self.ir.len());
        self.ir.emit(ADeadOp::Sub {
            dst: Operand::Reg(Reg::RSP),
            src: Operand::Imm32(0),
        });
        self.temp_alloc = TempAllocator::new();
    }

    pub(crate) fn patch_prologue(&mut self) {
        if let Some(idx) = self.prologue_sub_index.take() {
            let locals_size = (-self.stack_offset) as i32;
            let shadow_space = if self.target == Target::Windows {
                32
            } else {
                0
            };
            let raw_size = locals_size + shadow_space;
            let aligned_size = ((raw_size + 15) / 16) * 16;
            let final_size = if aligned_size < 32 { 32 } else { aligned_size };

            if let Some(op) = self.ir.ops_mut().get_mut(idx) {
                *op = ADeadOp::Sub {
                    dst: Operand::Reg(Reg::RSP),
                    src: Operand::Imm32(final_size),
                };
            }
        }
    }

    pub(crate) fn emit_epilogue(&mut self) {
        self.ir.emit(ADeadOp::Lea {
            dst: Reg::RSP,
            src: Operand::Mem {
                base: Reg::RBP,
                disp: -32,
            },
        });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RDI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RSI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R12 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBP });
        self.ir.emit(ADeadOp::Ret);
    }

    pub(crate) fn emit_interrupt_prologue(&mut self) {
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RAX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RBX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RCX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RDX),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RSI),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RDI),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::RBP),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R8),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R9),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R10),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R11),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R12),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R13),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R14),
        });
        self.ir.emit(ADeadOp::Push {
            src: Operand::Reg(Reg::R15),
        });
    }

    pub(crate) fn emit_interrupt_epilogue(&mut self) {
        self.ir.emit(ADeadOp::Pop { dst: Reg::R15 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R14 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R13 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R12 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R11 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R10 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R9 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::R8 });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBP });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RDI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RSI });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RDX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RCX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RBX });
        self.ir.emit(ADeadOp::Pop { dst: Reg::RAX });
        self.ir.emit(ADeadOp::Iret);
    }
}
