// ============================================================
// Python AST → ADeadOp IR Converter for PyDead-BIB
// ============================================================
// Lowers Python AST to ADeadOp IR (SSA-form)
// This is the bridge: Python enters here, ADeadOp IR exits.
//
// Pipeline: Python Source → PyLexer → PyParser → PyModule
//           → PyToIR → IRProgram → ISA Compiler → PE/ELF
//
// GIL eliminado: cada objeto tiene ownership estático ✓
// ============================================================

use super::py_ast::*;
use crate::middle::ir::{IRCmpOp, IRConstValue, IRFunction, IRInstruction, IRModule, IROp, IRType};

use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn fresh_temp(prefix: &str) -> String {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("__{}{}", prefix, id)
}

/// IR program output from Python compilation
#[derive(Debug)]
pub struct IRProgram {
    pub module: IRModule,
    pub functions: Vec<IRFunction>,
    pub globals: Vec<IRGlobal>,
    pub string_data: Vec<(String, String)>, // label → string content
}

/// Global variable in IR
#[derive(Debug, Clone)]
pub struct IRGlobal {
    pub name: String,
    pub ir_type: IRType,
    pub init_value: Option<IRConstant>,
}

/// IR constant value
#[derive(Debug, Clone)]
pub enum IRConstant {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    None,
}

impl IRProgram {
    pub fn new() -> Self {
        Self {
            module: IRModule::new("main"),
            functions: Vec::new(),
            globals: Vec::new(),
            string_data: Vec::new(),
        }
    }

    pub fn statement_count(&self) -> usize {
        self.functions.iter().map(|f| f.body.len()).sum()
    }
}

/// Python to IR converter
pub struct PyToIR {
    string_counter: u64,
}

impl PyToIR {
    pub fn new() -> Self {
        Self {
            string_counter: 0,
        }
    }

    /// Main entry: Convert Python module → IR program
    pub fn convert(&mut self, module: &PyModule) -> Result<IRProgram, String> {
        let mut program = IRProgram::new();

        for stmt in &module.body {
            self.convert_stmt(stmt, &mut program)?;
        }

        Ok(program)
    }

    fn convert_stmt(&mut self, stmt: &PyStmt, program: &mut IRProgram) -> Result<(), String> {
        match stmt {
            PyStmt::FunctionDef { name, params, body, return_type, .. } => {
                let ret_type = return_type.as_ref()
                    .map(|t| self.pytype_to_ir(t))
                    .unwrap_or(IRType::Void);

                let ir_params: Vec<(String, IRType)> = params.iter().map(|p| {
                    let t = p.annotation.as_ref()
                        .map(|a| self.pytype_to_ir(a))
                        .unwrap_or(IRType::I64);
                    (p.name.clone(), t)
                }).collect();

                let mut func = IRFunction::new(name.clone(), ir_params, ret_type);

                for s in body {
                    self.convert_body_stmt(s, &mut func, program)?;
                }

                program.functions.push(func);
            }
            PyStmt::Assign { targets, value } => {
                for target in targets {
                    if let PyExpr::Name(name) = target {
                        let ir_type = self.infer_expr_type(value);
                        let init = self.expr_to_constant(value);
                        program.globals.push(IRGlobal {
                            name: name.clone(),
                            ir_type,
                            init_value: init,
                        });
                    }
                }
            }
            PyStmt::ClassDef { name, body, .. } => {
                // Convert methods to functions with class prefix
                for s in body {
                    if let PyStmt::FunctionDef { name: method_name, params, body: method_body, return_type, .. } = s {
                        let full_name = format!("{}__{}", name, method_name);
                        let ret_type = return_type.as_ref()
                            .map(|t| self.pytype_to_ir(t))
                            .unwrap_or(IRType::Void);

                        let ir_params: Vec<(String, IRType)> = params.iter().map(|p| {
                            let t = p.annotation.as_ref()
                                .map(|a| self.pytype_to_ir(a))
                                .unwrap_or(IRType::I64);
                            (p.name.clone(), t)
                        }).collect();

                        let mut func = IRFunction::new(full_name, ir_params, ret_type);
                        for ms in method_body {
                            self.convert_body_stmt(ms, &mut func, program)?;
                        }
                        program.functions.push(func);
                    }
                }
            }
            PyStmt::Import { .. } | PyStmt::ImportFrom { .. } => {
                // Imports resolved at compile time — no IR needed
            }
            PyStmt::Expr(expr) => {
                // Top-level expression (e.g., function call)
                if let PyExpr::Call { func, args, .. } = expr {
                    if let PyExpr::Name(name) = func.as_ref() {
                        if name == "print" {
                            // print() at module level → emit in __main__
                            for arg in args {
                                if let PyExpr::StringLiteral(s) = arg {
                                    let label = self.add_string(s, program);
                                    // Will be handled by __main__ generation
                                    let _ = label;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn convert_body_stmt(&mut self, stmt: &PyStmt, func: &mut IRFunction, program: &mut IRProgram) -> Result<(), String> {
        match stmt {
            PyStmt::Return(Some(expr)) => {
                let instr = self.convert_expr_to_instr(expr, program);
                func.body.push(instr);
                func.body.push(IRInstruction::Return);
            }
            PyStmt::Return(None) => {
                func.body.push(IRInstruction::ReturnVoid);
            }
            PyStmt::Assign { targets, value } => {
                let val_instr = self.convert_expr_to_instr(value, program);
                for target in targets {
                    if let PyExpr::Name(name) = target {
                        func.body.push(IRInstruction::VarDecl {
                            name: name.clone(),
                            ir_type: self.infer_expr_type(value),
                        });
                        func.body.push(val_instr.clone());
                        func.body.push(IRInstruction::Store(name.clone()));
                    }
                }
            }
            PyStmt::If { test, body, elif_clauses: _, orelse } => {
                let cond = self.convert_expr_to_instr(test, program);
                func.body.push(cond);
                let else_label = fresh_temp("else");
                let end_label = fresh_temp("endif");
                func.body.push(IRInstruction::BranchIfFalse(else_label.clone()));
                for s in body {
                    self.convert_body_stmt(s, func, program)?;
                }
                func.body.push(IRInstruction::Jump(end_label.clone()));
                func.body.push(IRInstruction::Label(else_label));
                for s in orelse {
                    self.convert_body_stmt(s, func, program)?;
                }
                func.body.push(IRInstruction::Label(end_label));
            }
            PyStmt::While { test, body, .. } => {
                let loop_label = fresh_temp("while");
                let end_label = fresh_temp("endwhile");
                func.body.push(IRInstruction::Label(loop_label.clone()));
                let cond = self.convert_expr_to_instr(test, program);
                func.body.push(cond);
                func.body.push(IRInstruction::BranchIfFalse(end_label.clone()));
                for s in body {
                    self.convert_body_stmt(s, func, program)?;
                }
                func.body.push(IRInstruction::Jump(loop_label));
                func.body.push(IRInstruction::Label(end_label));
            }
            PyStmt::For { target, iter, body, .. } => {
                let iter_label = fresh_temp("for");
                let end_label = fresh_temp("endfor");
                let iter_instr = self.convert_expr_to_instr(iter, program);
                func.body.push(iter_instr);
                func.body.push(IRInstruction::Label(iter_label.clone()));
                if let PyExpr::Name(name) = target {
                    func.body.push(IRInstruction::IterNext {
                        target: name.clone(),
                        end_label: end_label.clone(),
                    });
                }
                for s in body {
                    self.convert_body_stmt(s, func, program)?;
                }
                func.body.push(IRInstruction::Jump(iter_label));
                func.body.push(IRInstruction::Label(end_label));
            }
            PyStmt::Expr(expr) => {
                let instr = self.convert_expr_to_instr(expr, program);
                func.body.push(instr);
            }
            PyStmt::Pass => {}
            PyStmt::Break => {
                func.body.push(IRInstruction::Break);
            }
            PyStmt::Continue => {
                func.body.push(IRInstruction::Continue);
            }
            _ => {}
        }
        Ok(())
    }

    fn convert_expr_to_instr(&mut self, expr: &PyExpr, program: &mut IRProgram) -> IRInstruction {
        match expr {
            PyExpr::IntLiteral(n) => IRInstruction::LoadConst(IRConstValue::Int(*n)),
            PyExpr::FloatLiteral(f) => IRInstruction::LoadConst(IRConstValue::Float(*f)),
            PyExpr::BoolLiteral(b) => IRInstruction::LoadConst(IRConstValue::Bool(*b)),
            PyExpr::NoneLiteral => IRInstruction::LoadConst(IRConstValue::None),
            PyExpr::StringLiteral(s) => {
                let label = self.add_string(s, program);
                IRInstruction::LoadString(label)
            }
            PyExpr::Name(name) => IRInstruction::Load(name.clone()),
            PyExpr::BinOp { op, left, right } => {
                let l = self.convert_expr_to_instr(left, program);
                let r = self.convert_expr_to_instr(right, program);
                IRInstruction::BinOp {
                    op: self.convert_binop(op),
                    left: Box::new(l),
                    right: Box::new(r),
                }
            }
            PyExpr::Call { func, args, .. } => {
                let func_name = match func.as_ref() {
                    PyExpr::Name(n) => n.clone(),
                    PyExpr::Attribute { value, attr } => {
                        if let PyExpr::Name(obj) = value.as_ref() {
                            format!("{}.{}", obj, attr)
                        } else {
                            "unknown".to_string()
                        }
                    }
                    _ => "unknown".to_string(),
                };
                let ir_args: Vec<IRInstruction> = args.iter()
                    .map(|a| self.convert_expr_to_instr(a, program))
                    .collect();
                IRInstruction::Call {
                    func: func_name,
                    args: ir_args,
                }
            }
            PyExpr::Compare { left, ops, comparators } => {
                let l = self.convert_expr_to_instr(left, program);
                if let (Some(op), Some(right)) = (ops.first(), comparators.first()) {
                    let r = self.convert_expr_to_instr(right, program);
                    IRInstruction::Compare {
                        op: self.convert_cmpop(op),
                        left: Box::new(l),
                        right: Box::new(r),
                    }
                } else {
                    l
                }
            }
            _ => IRInstruction::Nop,
        }
    }

    fn add_string(&mut self, s: &str, program: &mut IRProgram) -> String {
        let label = format!("__str{}", self.string_counter);
        self.string_counter += 1;
        program.string_data.push((label.clone(), s.to_string()));
        label
    }

    fn pytype_to_ir(&self, py_type: &PyType) -> IRType {
        match py_type {
            PyType::Int => IRType::I64,
            PyType::Float => IRType::F64,
            PyType::Bool => IRType::I8,
            PyType::Str => IRType::Ptr,
            PyType::None => IRType::Void,
            PyType::Bytes => IRType::Ptr,
            PyType::List(_) => IRType::Ptr,
            PyType::Dict(_, _) => IRType::Ptr,
            _ => IRType::I64,
        }
    }

    fn infer_expr_type(&self, expr: &PyExpr) -> IRType {
        match expr {
            PyExpr::IntLiteral(_) => IRType::I64,
            PyExpr::FloatLiteral(_) => IRType::F64,
            PyExpr::BoolLiteral(_) => IRType::I8,
            PyExpr::StringLiteral(_) | PyExpr::FString { .. } => IRType::Ptr,
            PyExpr::NoneLiteral => IRType::Void,
            PyExpr::List(_) => IRType::Ptr,
            PyExpr::Dict { .. } => IRType::Ptr,
            _ => IRType::I64,
        }
    }

    fn expr_to_constant(&self, expr: &PyExpr) -> Option<IRConstant> {
        match expr {
            PyExpr::IntLiteral(n) => Some(IRConstant::Int(*n)),
            PyExpr::FloatLiteral(f) => Some(IRConstant::Float(*f)),
            PyExpr::BoolLiteral(b) => Some(IRConstant::Bool(*b)),
            PyExpr::StringLiteral(s) => Some(IRConstant::Str(s.clone())),
            PyExpr::NoneLiteral => Some(IRConstant::None),
            _ => std::option::Option::None,
        }
    }

    fn convert_binop(&self, op: &PyBinOp) -> IROp {
        match op {
            PyBinOp::Add => IROp::Add,
            PyBinOp::Sub => IROp::Sub,
            PyBinOp::Mul => IROp::Mul,
            PyBinOp::Div => IROp::Div,
            PyBinOp::FloorDiv => IROp::FloorDiv,
            PyBinOp::Mod => IROp::Mod,
            PyBinOp::Pow => IROp::Pow,
            PyBinOp::LShift => IROp::Shl,
            PyBinOp::RShift => IROp::Shr,
            PyBinOp::BitOr => IROp::Or,
            PyBinOp::BitXor => IROp::Xor,
            PyBinOp::BitAnd => IROp::And,
            PyBinOp::MatMul => IROp::MatMul,
        }
    }

    fn convert_cmpop(&self, op: &PyCmpOp) -> IRCmpOp {
        match op {
            PyCmpOp::Eq => IRCmpOp::Eq,
            PyCmpOp::NotEq => IRCmpOp::Ne,
            PyCmpOp::Lt => IRCmpOp::Lt,
            PyCmpOp::LtE => IRCmpOp::Le,
            PyCmpOp::Gt => IRCmpOp::Gt,
            PyCmpOp::GtE => IRCmpOp::Ge,
            PyCmpOp::Is => IRCmpOp::Eq,
            PyCmpOp::IsNot => IRCmpOp::Ne,
            PyCmpOp::In => IRCmpOp::In,
            PyCmpOp::NotIn => IRCmpOp::NotIn,
        }
    }
}

/// Public entry point: compile Python module to IR
pub fn compile_python_to_ir(module: &PyModule) -> Result<IRProgram, Box<dyn std::error::Error>> {
    let mut converter = PyToIR::new();
    converter.convert(module).map_err(|e| e.into())
}
