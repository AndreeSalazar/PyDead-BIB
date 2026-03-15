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
    string_vars: std::collections::HashMap<String, String>, // var_name → string label
}

impl PyToIR {
    pub fn new() -> Self {
        Self {
            string_counter: 0,
            string_vars: std::collections::HashMap::new(),
        }
    }

    /// Main entry: Convert Python module → IR program
    pub fn convert(&mut self, module: &PyModule) -> Result<IRProgram, String> {
        let mut program = IRProgram::new();
        let mut toplevel_stmts: Vec<&PyStmt> = Vec::new();

        // First pass: collect functions/classes, note top-level stmts
        for stmt in &module.body {
            match stmt {
                PyStmt::FunctionDef { .. } | PyStmt::ClassDef { .. } => {
                    self.convert_stmt(stmt, &mut program)?;
                }
                PyStmt::Import { .. } | PyStmt::ImportFrom { .. } => {}
                _ => {
                    toplevel_stmts.push(stmt);
                }
            }
        }

        // Second pass: generate __main__ for top-level expressions
        if !toplevel_stmts.is_empty() {
            let mut main_func = IRFunction::new("__main__".to_string(), vec![], IRType::Void);
            for stmt in &toplevel_stmts {
                self.convert_body_stmt(stmt, &mut main_func, &mut program)?;
            }
            main_func.body.push(IRInstruction::ReturnVoid);
            program.functions.push(main_func);
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
                        // Track string literal assignments for f-string support
                        if let PyExpr::StringLiteral(s) = value {
                            let label = self.add_string(s, program);
                            self.string_vars.insert(name.clone(), label);
                        }
                        func.body.push(IRInstruction::VarDecl {
                            name: name.clone(),
                            ir_type: self.infer_expr_type(value),
                        });
                        func.body.push(val_instr.clone());
                        func.body.push(IRInstruction::Store(name.clone()));
                    }
                }
            }
            PyStmt::If { test, body, elif_clauses, orelse } => {
                let cond = self.convert_expr_to_instr(test, program);
                func.body.push(cond);
                let end_label = fresh_temp("endif");
                let next_label = if !elif_clauses.is_empty() || !orelse.is_empty() {
                    fresh_temp("else")
                } else {
                    end_label.clone()
                };
                func.body.push(IRInstruction::BranchIfFalse(next_label.clone()));
                for s in body {
                    self.convert_body_stmt(s, func, program)?;
                }
                func.body.push(IRInstruction::Jump(end_label.clone()));

                // Handle elif clauses
                let mut prev_label = next_label;
                for (i, (elif_test, elif_body)) in elif_clauses.iter().enumerate() {
                    func.body.push(IRInstruction::Label(prev_label));
                    let elif_cond = self.convert_expr_to_instr(elif_test, program);
                    func.body.push(elif_cond);
                    let next = if i + 1 < elif_clauses.len() || !orelse.is_empty() {
                        fresh_temp("elif")
                    } else {
                        end_label.clone()
                    };
                    func.body.push(IRInstruction::BranchIfFalse(next.clone()));
                    for s in elif_body {
                        self.convert_body_stmt(s, func, program)?;
                    }
                    func.body.push(IRInstruction::Jump(end_label.clone()));
                    prev_label = next;
                }

                // Handle else
                if !orelse.is_empty() {
                    func.body.push(IRInstruction::Label(prev_label));
                    for s in orelse {
                        self.convert_body_stmt(s, func, program)?;
                    }
                } else if prev_label != end_label {
                    func.body.push(IRInstruction::Label(prev_label));
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
                // Handle for i in range(...)
                if let PyExpr::Name(var_name) = target {
                    if let PyExpr::Call { func: call_func, args, .. } = iter {
                        if let PyExpr::Name(fn_name) = call_func.as_ref() {
                            if fn_name == "range" {
                                // Parse range(stop), range(start, stop), range(start, stop, step)
                                let (start, step) = match args.len() {
                                    1 => (0i64, 1i64),
                                    2 => {
                                        let s = self.extract_int_literal(&args[0]).unwrap_or(0);
                                        (s, 1)
                                    }
                                    3 => {
                                        let s = self.extract_int_literal(&args[0]).unwrap_or(0);
                                        let st = self.extract_int_literal(&args[2]).unwrap_or(1);
                                        (s, st)
                                    }
                                    _ => (0, 1),
                                };

                                let stop_expr = if args.len() == 1 {
                                    self.convert_expr_to_instr(&args[0], program)
                                } else {
                                    self.convert_expr_to_instr(&args[1], program)
                                };

                                // Declare loop variable
                                func.body.push(IRInstruction::VarDecl {
                                    name: var_name.clone(), ir_type: crate::middle::ir::IRType::I64,
                                });
                                // Declare __end and __step temp vars
                                let end_var = fresh_temp("end");
                                let step_var = fresh_temp("step");
                                func.body.push(IRInstruction::VarDecl {
                                    name: end_var.clone(), ir_type: crate::middle::ir::IRType::I64,
                                });
                                func.body.push(IRInstruction::VarDecl {
                                    name: step_var.clone(), ir_type: crate::middle::ir::IRType::I64,
                                });

                                // Init: var = start
                                func.body.push(IRInstruction::LoadConst(IRConstValue::Int(start)));
                                func.body.push(IRInstruction::Store(var_name.clone()));
                                // Init: __end = stop_expr
                                func.body.push(stop_expr);
                                func.body.push(IRInstruction::Store(end_var.clone()));
                                // Init: __step = step
                                func.body.push(IRInstruction::LoadConst(IRConstValue::Int(step)));
                                func.body.push(IRInstruction::Store(step_var.clone()));

                                let loop_label = fresh_temp("for");
                                let end_label = fresh_temp("endfor");
                                func.body.push(IRInstruction::Label(loop_label.clone()));

                                // Compare: var < __end (for positive step)
                                func.body.push(IRInstruction::Compare {
                                    op: if step >= 0 { IRCmpOp::Lt } else { IRCmpOp::Gt },
                                    left: Box::new(IRInstruction::Load(var_name.clone())),
                                    right: Box::new(IRInstruction::Load(end_var.clone())),
                                });
                                func.body.push(IRInstruction::BranchIfFalse(end_label.clone()));

                                // Body
                                for s in body {
                                    self.convert_body_stmt(s, func, program)?;
                                }

                                // Increment: var += step
                                func.body.push(IRInstruction::BinOp {
                                    op: IROp::Add,
                                    left: Box::new(IRInstruction::Load(var_name.clone())),
                                    right: Box::new(IRInstruction::Load(step_var.clone())),
                                });
                                func.body.push(IRInstruction::Store(var_name.clone()));
                                func.body.push(IRInstruction::Jump(loop_label));
                                func.body.push(IRInstruction::Label(end_label));
                                return Ok(());
                            }
                        }
                    }
                }
                // Fallback: generic for (not implemented yet)
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
                // Intercept print() calls → emit PrintStr/PrintInt/PrintNewline
                if let PyExpr::Call { func: call_func, args, .. } = expr {
                    if let PyExpr::Name(name) = call_func.as_ref() {
                        if name == "print" {
                            for (i, arg) in args.iter().enumerate() {
                                if i > 0 {
                                    // Space between args
                                    let sp_label = self.add_string(" ", program);
                                    func.body.push(IRInstruction::PrintStr(sp_label));
                                }
                                match arg {
                                    PyExpr::StringLiteral(s) => {
                                        let label = self.add_string(s, program);
                                        func.body.push(IRInstruction::PrintStr(label));
                                    }
                                    PyExpr::IntLiteral(n) => {
                                        func.body.push(IRInstruction::LoadConst(IRConstValue::Int(*n)));
                                        func.body.push(IRInstruction::PrintInt);
                                    }
                                    PyExpr::FloatLiteral(f) => {
                                        func.body.push(IRInstruction::LoadConst(IRConstValue::Float(*f)));
                                        func.body.push(IRInstruction::PrintFloat);
                                    }
                                    PyExpr::BoolLiteral(b) => {
                                        let label = self.add_string(if *b { "True" } else { "False" }, program);
                                        func.body.push(IRInstruction::PrintStr(label));
                                    }
                                    PyExpr::FString { parts } => {
                                        for part in parts {
                                            match part {
                                                FStringPart::Literal(s) => {
                                                    let label = self.add_string(s, program);
                                                    func.body.push(IRInstruction::PrintStr(label));
                                                }
                                                FStringPart::Expression(expr, _fmt) => {
                                                    match expr {
                                                        PyExpr::Name(n) => {
                                                            // Check if this is a known string variable
                                                            if let Some(label) = self.string_vars.get(n).cloned() {
                                                                func.body.push(IRInstruction::PrintStr(label));
                                                            } else {
                                                                let instr = self.convert_expr_to_instr(expr, program);
                                                                func.body.push(instr);
                                                                func.body.push(IRInstruction::PrintInt);
                                                            }
                                                        }
                                                        PyExpr::IntLiteral(_) | PyExpr::BinOp { .. } => {
                                                            let instr = self.convert_expr_to_instr(expr, program);
                                                            func.body.push(instr);
                                                            func.body.push(IRInstruction::PrintInt);
                                                        }
                                                        PyExpr::StringLiteral(s) => {
                                                            let label = self.add_string(s, program);
                                                            func.body.push(IRInstruction::PrintStr(label));
                                                        }
                                                        PyExpr::FloatLiteral(f) => {
                                                            func.body.push(IRInstruction::LoadConst(IRConstValue::Float(*f)));
                                                            func.body.push(IRInstruction::PrintFloat);
                                                        }
                                                        _ => {
                                                            let instr = self.convert_expr_to_instr(expr, program);
                                                            func.body.push(instr);
                                                            func.body.push(IRInstruction::PrintInt);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        // Variable or expression → evaluate → print as int
                                        let instr = self.convert_expr_to_instr(arg, program);
                                        func.body.push(instr);
                                        func.body.push(IRInstruction::PrintInt);
                                    }
                                }
                            }
                            if args.is_empty() {
                                // print() with no args → just newline
                            }
                            func.body.push(IRInstruction::PrintNewline);
                            return Ok(());
                        }
                    }
                }
                let instr = self.convert_expr_to_instr(expr, program);
                func.body.push(instr);
            }
            PyStmt::AugAssign { target, op, value } => {
                if let PyExpr::Name(name) = target {
                    let val_instr = self.convert_expr_to_instr(value, program);
                    func.body.push(IRInstruction::VarDecl {
                        name: name.clone(),
                        ir_type: self.infer_expr_type(value),
                    });
                    func.body.push(IRInstruction::BinOp {
                        op: self.convert_binop(op),
                        left: Box::new(IRInstruction::Load(name.clone())),
                        right: Box::new(val_instr),
                    });
                    func.body.push(IRInstruction::Store(name.clone()));
                }
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
                // Builtins handled specially — not a generic Call
                if func_name == "print" || func_name == "abs" || func_name == "min" || func_name == "max" {
                    let ir_args: Vec<IRInstruction> = args.iter()
                        .map(|a| self.convert_expr_to_instr(a, program))
                        .collect();
                    return IRInstruction::Call {
                        func: func_name,
                        args: ir_args,
                    };
                }
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
            PyExpr::UnaryOp { op, operand } => {
                match op {
                    PyUnaryOp::Neg => {
                        // -x → 0 - x
                        if let PyExpr::IntLiteral(n) = operand.as_ref() {
                            return IRInstruction::LoadConst(IRConstValue::Int(-n));
                        }
                        if let PyExpr::FloatLiteral(f) = operand.as_ref() {
                            return IRInstruction::LoadConst(IRConstValue::Float(-f));
                        }
                        let inner = self.convert_expr_to_instr(operand, program);
                        IRInstruction::BinOp {
                            op: IROp::Sub,
                            left: Box::new(IRInstruction::LoadConst(IRConstValue::Int(0))),
                            right: Box::new(inner),
                        }
                    }
                    PyUnaryOp::Pos => self.convert_expr_to_instr(operand, program),
                    PyUnaryOp::Invert => {
                        let inner = self.convert_expr_to_instr(operand, program);
                        IRInstruction::BinOp {
                            op: IROp::Xor,
                            left: Box::new(inner),
                            right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(-1))),
                        }
                    }
                    _ => self.convert_expr_to_instr(operand, program),
                }
            }
            _ => IRInstruction::Nop,
        }
    }

    fn extract_int_literal(&self, expr: &PyExpr) -> Option<i64> {
        match expr {
            PyExpr::IntLiteral(n) => Some(*n),
            PyExpr::UnaryOp { op: PyUnaryOp::Neg, operand } => {
                if let PyExpr::IntLiteral(n) = operand.as_ref() {
                    Some(-n)
                } else {
                    None
                }
            }
            _ => None,
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
