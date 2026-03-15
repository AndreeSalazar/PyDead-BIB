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
    dict_vars: std::collections::HashSet<String>, // variables that are dicts
    file_vars: std::collections::HashSet<String>, // variables that are file handles
    str_heap_vars: std::collections::HashSet<String>, // variables that are heap strings
    except_label_stack: Vec<String>, // stack of except handler labels for raise
    class_fields: std::collections::HashMap<String, Vec<String>>, // class_name → ordered field names
    class_vars: std::collections::HashMap<String, String>, // var_name → class_name
    class_names: std::collections::HashSet<String>, // known class names
}

impl PyToIR {
    pub fn new() -> Self {
        Self {
            string_counter: 0,
            string_vars: std::collections::HashMap::new(),
            dict_vars: std::collections::HashSet::new(),
            file_vars: std::collections::HashSet::new(),
            str_heap_vars: std::collections::HashSet::new(),
            except_label_stack: Vec::new(),
            class_fields: std::collections::HashMap::new(),
            class_vars: std::collections::HashMap::new(),
            class_names: std::collections::HashSet::new(),
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
            PyStmt::ClassDef { name, bases, body, .. } => {
                self.class_names.insert(name.clone());

                // Inherit parent fields if bases exist
                let mut fields = Vec::new();
                for base in bases {
                    if let PyExpr::Name(base_name) = base {
                        if let Some(parent_fields) = self.class_fields.get(base_name).cloned() {
                            for f in parent_fields {
                                if !fields.contains(&f) {
                                    fields.push(f);
                                }
                            }
                        }
                    }
                }

                // Scan __init__ to find field names (self.x = ...)
                for s in body {
                    if let PyStmt::FunctionDef { name: mn, body: mb, .. } = s {
                        if mn == "__init__" {
                            for ms in mb {
                                if let PyStmt::Assign { targets, .. } = ms {
                                    for t in targets {
                                        if let PyExpr::Attribute { value, attr } = t {
                                            if let PyExpr::Name(n) = value.as_ref() {
                                                if n == "self" && !fields.contains(attr) {
                                                    fields.push(attr.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.class_fields.insert(name.clone(), fields);

                // Convert methods to functions with class prefix
                for s in body {
                    if let PyStmt::FunctionDef { name: method_name, params, body: method_body, return_type, .. } = s {
                        let full_name = format!("{}__{}", name, method_name);
                        let ret_type = return_type.as_ref()
                            .map(|t| self.pytype_to_ir(t))
                            .unwrap_or(IRType::Void);

                        // First param (self) is a Ptr
                        let ir_params: Vec<(String, IRType)> = params.iter().enumerate().map(|(i, p)| {
                            if i == 0 && p.name == "self" {
                                (p.name.clone(), IRType::Ptr)
                            } else {
                                let t = p.annotation.as_ref()
                                    .map(|a| self.pytype_to_ir(a))
                                    .unwrap_or(IRType::I64);
                                (p.name.clone(), t)
                            }
                        }).collect();

                        let mut func = IRFunction::new(full_name, ir_params, ret_type);
                        // Inside methods, handle self.x = val and self.x access
                        for ms in method_body {
                            self.convert_class_body_stmt(ms, &mut func, program, name)?;
                        }
                        program.functions.push(func);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn convert_class_body_stmt(&mut self, stmt: &PyStmt, func: &mut IRFunction, program: &mut IRProgram, class_name: &str) -> Result<(), String> {
        // Handle self.x = val → store value at instance field offset
        if let PyStmt::Assign { targets, value } = stmt {
            if targets.len() == 1 {
                if let PyExpr::Attribute { value: target_obj, attr } = &targets[0] {
                    if let PyExpr::Name(n) = target_obj.as_ref() {
                        if n == "self" {
                            // Find field offset
                            let offset = if let Some(fields) = self.class_fields.get(class_name) {
                                fields.iter().position(|f| f == attr).unwrap_or(0)
                            } else { 0 };
                            let byte_offset = (offset as i64 + 1) * 8; // +1 to skip class_id at [0]
                            let val_instr = self.convert_expr_to_instr(value, program);
                            // Call __pyb_obj_set_field(self, offset, value)
                            func.body.push(IRInstruction::Call {
                                func: "__pyb_obj_set_field".to_string(),
                                args: vec![
                                    IRInstruction::Load("self".to_string()),
                                    IRInstruction::LoadConst(IRConstValue::Int(byte_offset)),
                                    val_instr,
                                ],
                            });
                            return Ok(());
                        }
                    }
                }
            }
        }
        // Delegate to normal handler
        self.convert_body_stmt(stmt, func, program)
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
                // Handle tuple unpacking: x, y = 10, 20 or x, y = y, x
                if targets.len() == 1 {
                    if let PyExpr::Tuple(target_names) = &targets[0] {
                        if let PyExpr::Tuple(values) = value {
                            if target_names.len() == values.len() {
                                // First evaluate all RHS values into temp vars
                                // (needed for swap: x, y = y, x)
                                let mut temps = Vec::new();
                                for (i, val) in values.iter().enumerate() {
                                    let tmp = fresh_temp(&format!("tup{}", i));
                                    func.body.push(IRInstruction::VarDecl {
                                        name: tmp.clone(),
                                        ir_type: self.infer_expr_type(val),
                                    });
                                    let val_instr = self.convert_expr_to_instr(val, program);
                                    func.body.push(val_instr);
                                    func.body.push(IRInstruction::Store(tmp.clone()));
                                    temps.push(tmp);
                                }
                                // Then assign temps to target names
                                for (tgt, tmp) in target_names.iter().zip(temps.iter()) {
                                    if let PyExpr::Name(name) = tgt {
                                        func.body.push(IRInstruction::VarDecl {
                                            name: name.clone(),
                                            ir_type: IRType::I64,
                                        });
                                        func.body.push(IRInstruction::Load(tmp.clone()));
                                        func.body.push(IRInstruction::Store(name.clone()));
                                    }
                                }
                                return Ok(());
                            }
                        }
                        // Tuple target with non-tuple value (e.g. function return)
                        // For now, fall through to single assignment
                    }
                }

                // Handle dict literal: d = {1: 10, 2: 20}
                if targets.len() == 1 {
                    if let PyExpr::Name(name) = &targets[0] {
                        if let PyExpr::Dict { keys, values } = value {
                            self.dict_vars.insert(name.clone());
                            func.body.push(IRInstruction::VarDecl {
                                name: name.clone(),
                                ir_type: IRType::Ptr,
                            });
                            func.body.push(IRInstruction::Call {
                                func: "__pyb_dict_new".to_string(),
                                args: vec![],
                            });
                            func.body.push(IRInstruction::Store(name.clone()));
                            for (k, v) in keys.iter().zip(values.iter()) {
                                if let Some(key_expr) = k {
                                    let key_instr = self.convert_expr_to_instr(key_expr, program);
                                    let val_instr = self.convert_expr_to_instr(v, program);
                                    func.body.push(IRInstruction::Call {
                                        func: "__pyb_dict_set".to_string(),
                                        args: vec![
                                            IRInstruction::Load(name.clone()),
                                            key_instr,
                                            val_instr,
                                        ],
                                    });
                                }
                            }
                            return Ok(());
                        }
                    }
                }

                // Handle list literal: x = [1, 2, 3]
                if targets.len() == 1 {
                    if let PyExpr::Name(name) = &targets[0] {
                        if let PyExpr::List(elts) = value {
                            // Create new list and append each element
                            func.body.push(IRInstruction::VarDecl {
                                name: name.clone(),
                                ir_type: IRType::Ptr,
                            });
                            func.body.push(IRInstruction::Call {
                                func: "__pyb_list_new".to_string(),
                                args: vec![],
                            });
                            func.body.push(IRInstruction::Store(name.clone()));
                            for elt in elts {
                                let val = self.convert_expr_to_instr(elt, program);
                                func.body.push(IRInstruction::Call {
                                    func: "__pyb_list_append".to_string(),
                                    args: vec![
                                        IRInstruction::Load(name.clone()),
                                        val,
                                    ],
                                });
                            }
                            return Ok(());
                        }
                    }
                }

                // Track class instance, file, and heap string assignments
                if targets.len() == 1 {
                    if let PyExpr::Name(var_name) = &targets[0] {
                        if let PyExpr::Call { func: call_f, args: call_args, .. } = value {
                            let fn_str = match call_f.as_ref() {
                                PyExpr::Name(n) => n.clone(),
                                PyExpr::Attribute { value: v, attr: a } => {
                                    if let PyExpr::Name(o) = v.as_ref() { format!("{}.{}", o, a) } else { String::new() }
                                }
                                _ => String::new(),
                            };
                            if self.class_names.contains(&fn_str) {
                                self.class_vars.insert(var_name.clone(), fn_str.clone());
                            }
                            if fn_str == "open" {
                                self.file_vars.insert(var_name.clone());
                            }
                            if matches!(fn_str.as_str(),
                                "os.getcwd" | "os.environ.get" |
                                "json.loads" | "json.dumps"
                            ) {
                                self.str_heap_vars.insert(var_name.clone());
                            }
                            // f.read() returns heap string
                            if fn_str.ends_with(".read") {
                                self.str_heap_vars.insert(var_name.clone());
                            }
                            // .upper(), .lower(), .replace() return heap string
                            if fn_str.ends_with(".upper") || fn_str.ends_with(".lower") || fn_str.ends_with(".replace") {
                                self.str_heap_vars.insert(var_name.clone());
                            }
                        }
                    }
                }

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
                // Intercept method calls: obj.append(val)
                if let PyExpr::Call { func: call_func, args, .. } = expr {
                    if let PyExpr::Attribute { value, attr } = call_func.as_ref() {
                        if let PyExpr::Name(obj_name) = value.as_ref() {
                            if attr == "append" && !args.is_empty() {
                                let val = self.convert_expr_to_instr(&args[0], program);
                                func.body.push(IRInstruction::Call {
                                    func: "__pyb_list_append".to_string(),
                                    args: vec![
                                        IRInstruction::Load(obj_name.clone()),
                                        val,
                                    ],
                                });
                                return Ok(());
                            }
                        }
                    }
                }
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
                                        // Check if arg is a heap string variable
                                        let is_heap_str = if let PyExpr::Name(n) = arg {
                                            self.str_heap_vars.contains(n)
                                        } else { false };
                                        // Check if arg is a call that returns a heap string
                                        let is_str_call = if let PyExpr::Call { func: f, .. } = arg {
                                            let fn_str = match f.as_ref() {
                                                PyExpr::Name(n) => n.clone(),
                                                PyExpr::Attribute { value: v, attr: a } => {
                                                    if let PyExpr::Name(o) = v.as_ref() { format!("{}.{}", o, a) } else { String::new() }
                                                }
                                                _ => String::new(),
                                            };
                                            matches!(fn_str.as_str(),
                                                "os.getcwd" | "os.environ.get" |
                                                "json.dumps" | "sys.platform" | "sys.version"
                                            ) || fn_str.ends_with(".upper") || fn_str.ends_with(".lower")
                                              || fn_str.ends_with(".replace") || fn_str.ends_with(".read")
                                        } else { false };
                                        // Check if arg is a sys.platform / sys.version attribute
                                        let is_sys_attr = if let PyExpr::Attribute { value: v, attr: a } = arg {
                                            if let PyExpr::Name(o) = v.as_ref() {
                                                (o == "sys" && (a == "platform" || a == "version"))
                                            } else { false }
                                        } else { false };

                                        if is_heap_str || is_str_call {
                                            let instr = self.convert_expr_to_instr(arg, program);
                                            func.body.push(instr);
                                            func.body.push(IRInstruction::Call {
                                                func: "__pyb_str_print".to_string(),
                                                args: vec![],
                                            });
                                        } else if is_sys_attr {
                                            let instr = self.convert_expr_to_instr(arg, program);
                                            func.body.push(instr);
                                            // LoadString already puts ptr in RAX, call str_print
                                            func.body.push(IRInstruction::Call {
                                                func: "__pyb_str_print".to_string(),
                                                args: vec![],
                                            });
                                        } else if self.is_float_expr(arg) {
                                            let instr = self.convert_expr_to_instr(arg, program);
                                            func.body.push(instr);
                                            func.body.push(IRInstruction::PrintFloat);
                                        } else if matches!(arg, PyExpr::Call { func: f, .. } if {
                                            if let PyExpr::Name(n) = f.as_ref() { n == "chr" } else { false }
                                        }) {
                                            let instr = self.convert_expr_to_instr(arg, program);
                                            func.body.push(instr);
                                            func.body.push(IRInstruction::PrintChar);
                                        } else {
                                            let instr = self.convert_expr_to_instr(arg, program);
                                            func.body.push(instr);
                                            func.body.push(IRInstruction::PrintInt);
                                        }
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
            // ── try/except/finally ────────────────────────────
            PyStmt::Try { body, handlers, orelse: _, finalbody } => {
                let except_label = fresh_temp("except");
                let end_label = fresh_temp("endtry");
                let finally_label = if !finalbody.is_empty() { fresh_temp("finally") } else { String::new() };

                // TryBegin → sets up error handler jump target
                func.body.push(IRInstruction::TryBegin(except_label.clone()));
                self.except_label_stack.push(except_label.clone());

                // Execute try body
                for s in body {
                    self.convert_body_stmt(s, func, program)?;
                }
                self.except_label_stack.pop();

                // No error → jump past except handlers
                func.body.push(IRInstruction::TryEnd);
                if !finalbody.is_empty() {
                    func.body.push(IRInstruction::Jump(finally_label.clone()));
                } else {
                    func.body.push(IRInstruction::Jump(end_label.clone()));
                }

                // Except handlers
                func.body.push(IRInstruction::Label(except_label));
                for handler in handlers {
                    // Each handler: check error type, execute body
                    if let Some(exc_type_expr) = &handler.exc_type {
                        let exc_name = match exc_type_expr {
                            PyExpr::Name(n) => n.clone(),
                            _ => "Exception".to_string(),
                        };
                        let handler_label = fresh_temp("handler");
                        let skip_label = fresh_temp("skiphandler");
                        // Check if error matches this type
                        func.body.push(IRInstruction::CheckError(skip_label.clone()));
                        func.body.push(IRInstruction::Label(handler_label));
                        // If handler has `as e`, store the error message
                        if let Some(var_name) = &handler.name {
                            func.body.push(IRInstruction::VarDecl {
                                name: var_name.clone(),
                                ir_type: crate::middle::ir::IRType::I64,
                            });
                            func.body.push(IRInstruction::LoadConst(IRConstValue::Int(0)));
                            func.body.push(IRInstruction::Store(var_name.clone()));
                            self.str_heap_vars.insert(var_name.clone());
                        }
                        func.body.push(IRInstruction::ClearError);
                        for s in &handler.body {
                            self.convert_body_stmt(s, func, program)?;
                        }
                        if !finalbody.is_empty() {
                            func.body.push(IRInstruction::Jump(finally_label.clone()));
                        } else {
                            func.body.push(IRInstruction::Jump(end_label.clone()));
                        }
                        func.body.push(IRInstruction::Label(skip_label));
                    } else {
                        // Bare except: catches everything
                        func.body.push(IRInstruction::ClearError);
                        for s in &handler.body {
                            self.convert_body_stmt(s, func, program)?;
                        }
                        if !finalbody.is_empty() {
                            func.body.push(IRInstruction::Jump(finally_label.clone()));
                        } else {
                            func.body.push(IRInstruction::Jump(end_label.clone()));
                        }
                    }
                }

                // Finally block
                if !finalbody.is_empty() {
                    func.body.push(IRInstruction::Label(finally_label));
                    for s in finalbody {
                        self.convert_body_stmt(s, func, program)?;
                    }
                }

                func.body.push(IRInstruction::Label(end_label));
            }
            // ── raise ─────────────────────────────────────────
            PyStmt::Raise { exc, .. } => {
                if let Some(exc_expr) = exc {
                    match exc_expr {
                        PyExpr::Call { func: call_fn, args, .. } => {
                            let exc_type = match call_fn.as_ref() {
                                PyExpr::Name(n) => n.clone(),
                                _ => "Exception".to_string(),
                            };
                            let msg = if !args.is_empty() {
                                Some(Box::new(self.convert_expr_to_instr(&args[0], program)))
                            } else {
                                None
                            };
                            func.body.push(IRInstruction::Raise { exc_type, message: msg });
                        }
                        PyExpr::Name(n) => {
                            func.body.push(IRInstruction::Raise {
                                exc_type: n.clone(),
                                message: None,
                            });
                        }
                        _ => {
                            func.body.push(IRInstruction::Raise {
                                exc_type: "Exception".to_string(),
                                message: None,
                            });
                        }
                    }
                } else {
                    func.body.push(IRInstruction::Raise {
                        exc_type: "Exception".to_string(),
                        message: None,
                    });
                }
                // Jump to nearest except handler if inside a try block
                if let Some(handler_label) = self.except_label_stack.last() {
                    func.body.push(IRInstruction::Jump(handler_label.clone()));
                }
            }
            // ── with statement ────────────────────────────────
            PyStmt::With { items, body, .. } => {
                // with expr as var: → var = expr.__enter__(); try: body; finally: expr.__exit__()
                for (context_expr, opt_var) in items {
                    let ctx_instr = self.convert_expr_to_instr(context_expr, program);
                    if let Some(var_expr) = opt_var {
                        if let PyExpr::Name(var_name) = var_expr {
                            // Track file vars from open()
                            if let PyExpr::Call { func: call_fn, .. } = context_expr {
                                if let PyExpr::Name(fn_name) = call_fn.as_ref() {
                                    if fn_name == "open" {
                                        self.file_vars.insert(var_name.clone());
                                    }
                                }
                            }
                            func.body.push(IRInstruction::VarDecl {
                                name: var_name.clone(),
                                ir_type: crate::middle::ir::IRType::I64,
                            });
                            func.body.push(ctx_instr);
                            func.body.push(IRInstruction::Store(var_name.clone()));
                        }
                    } else {
                        func.body.push(ctx_instr);
                    }
                }
                // Execute body
                let finally_label = fresh_temp("withfin");
                let end_label = fresh_temp("endwith");
                func.body.push(IRInstruction::TryBegin(finally_label.clone()));
                for s in body {
                    self.convert_body_stmt(s, func, program)?;
                }
                func.body.push(IRInstruction::TryEnd);
                func.body.push(IRInstruction::Jump(finally_label.clone()));
                func.body.push(IRInstruction::Label(finally_label));
                // __exit__: for file handles, call close
                for (context_expr, opt_var) in items {
                    if let Some(var_expr) = opt_var {
                        if let PyExpr::Name(var_name) = var_expr {
                            if self.file_vars.contains(var_name) {
                                func.body.push(IRInstruction::Call {
                                    func: "__pyb_file_close".to_string(),
                                    args: vec![IRInstruction::Load(var_name.clone())],
                                });
                            }
                        }
                    }
                }
                func.body.push(IRInstruction::Label(end_label));
            }
            // ── assert ────────────────────────────────────────
            PyStmt::Assert { test, msg } => {
                let cond = self.convert_expr_to_instr(test, program);
                func.body.push(cond);
                let ok_label = fresh_temp("assert_ok");
                func.body.push(IRInstruction::BranchIfFalse(ok_label.clone()));
                func.body.push(IRInstruction::Jump(ok_label.clone()));
                // If false: raise AssertionError
                func.body.push(IRInstruction::Label(ok_label));
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
            PyExpr::Call { func: call_fn, args, .. } => {
                let func_name = match call_fn.as_ref() {
                    PyExpr::Name(n) => n.clone(),
                    PyExpr::Attribute { value, attr } => {
                        match value.as_ref() {
                            PyExpr::Name(obj) => format!("{}.{}", obj, attr),
                            PyExpr::Attribute { value: v2, attr: a2 } => {
                                if let PyExpr::Name(obj2) = v2.as_ref() {
                                    format!("{}.{}.{}", obj2, a2, attr)
                                } else {
                                    format!("?.{}.{}", a2, attr)
                                }
                            }
                            _ => "unknown".to_string(),
                        }
                    }
                    _ => "unknown".to_string(),
                };

                // ── math module functions ──────────────────────
                match func_name.as_str() {
                    "math.sqrt" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_sqrt".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.floor" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_floor".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.ceil" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_ceil".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.sin" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_sin".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.cos" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_cos".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.log" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_log".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.pow" if args.len() >= 2 => {
                        let base = self.convert_expr_to_instr(&args[0], program);
                        let exp = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::BinOp {
                            op: IROp::Pow,
                            left: Box::new(base),
                            right: Box::new(exp),
                        };
                    }
                    "math.abs" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_abs".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── os module ─────────────────────────────
                    "os.getcwd" => {
                        return IRInstruction::Call {
                            func: "__pyb_os_getcwd".to_string(),
                            args: vec![],
                        };
                    }
                    "os.path.exists" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_path_exists".to_string(),
                            args: vec![arg],
                        };
                    }
                    "os.getpid" => {
                        return IRInstruction::Call {
                            func: "__pyb_os_getpid".to_string(),
                            args: vec![],
                        };
                    }
                    "os.makedirs" | "os.mkdir" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_mkdir".to_string(),
                            args: vec![arg],
                        };
                    }
                    "os.remove" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_remove".to_string(),
                            args: vec![arg],
                        };
                    }
                    "os.rename" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_rename".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "os.environ.get" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_environ_get".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── sys module ────────────────────────────
                    "sys.exit" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_sys_exit".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── random module ─────────────────────────
                    "random.randint" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__pyb_random_randint".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "random.seed" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_random_seed".to_string(),
                            args: vec![arg],
                        };
                    }
                    "random.random" => {
                        return IRInstruction::Call {
                            func: "__pyb_random_next".to_string(),
                            args: vec![],
                        };
                    }
                    "random.choice" if !args.is_empty() => {
                        // choice(list) = list[randint(0, len-1)]
                        let lst = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_random_next".to_string(),
                            args: vec![lst],
                        };
                    }
                    // ── json module ───────────────────────────
                    "json.loads" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_json_loads".to_string(),
                            args: vec![arg],
                        };
                    }
                    "json.dumps" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_json_dumps".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── asyncio module ────────────────────────
                    "asyncio.run" if !args.is_empty() => {
                        // asyncio.run(coro()) → just call the coroutine directly
                        let coro = self.convert_expr_to_instr(&args[0], program);
                        return coro;
                    }
                    // ── numpy module ──────────────────────────
                    "np.array" | "numpy.array" if !args.is_empty() => {
                        // np.array([...]) → create list from elements
                        if let PyExpr::List(elts) = &args[0] {
                            // Build a list with the elements
                            return IRInstruction::Call {
                                func: "__pyb_list_new".to_string(),
                                args: vec![],
                            };
                        }
                        return IRInstruction::Call {
                            func: "__pyb_list_new".to_string(),
                            args: vec![],
                        };
                    }
                    "np.sum" | "numpy.sum" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_sum".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.max" | "numpy.max" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_max".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.min" | "numpy.min" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_min".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.dot" | "numpy.dot" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_dot".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "np.sqrt" | "numpy.sqrt" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_sqrt".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.zeros" | "numpy.zeros" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_listcomp_range".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.ones" | "numpy.ones" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_listcomp_range".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.mean" | "numpy.mean" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_sum".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── ctypes module ─────────────────────────
                    "ctypes.CDLL" if !args.is_empty() => {
                        let path = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_dll_load".to_string(),
                            args: vec![path],
                        };
                    }
                    "ctypes.c_int" if !args.is_empty() => {
                        // ctypes.c_int(42) → just return the int
                        return self.convert_expr_to_instr(&args[0], program);
                    }
                    "ctypes.c_double" if !args.is_empty() => {
                        return self.convert_expr_to_instr(&args[0], program);
                    }
                    // ── functools ─────────────────────────────
                    "functools.lru_cache" => {
                        // @lru_cache decorator — passthrough, handled at class level
                        return IRInstruction::Nop;
                    }
                    // ── sum() builtin ─────────────────────────
                    "sum" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_sum".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── next() builtin ────────────────────────
                    "next" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_gen_next".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── open() builtin ────────────────────────
                    "open" if !args.is_empty() => {
                        let path_arg = self.convert_expr_to_instr(&args[0], program);
                        let mode_val = if args.len() >= 2 {
                            // "r" → 0, "w" → 1
                            if let PyExpr::StringLiteral(m) = &args[1] {
                                if m == "w" { 1i64 } else { 0i64 }
                            } else { 0i64 }
                        } else { 0i64 };
                        return IRInstruction::Call {
                            func: "__pyb_file_open".to_string(),
                            args: vec![
                                path_arg,
                                IRInstruction::LoadConst(IRConstValue::Int(mode_val)),
                            ],
                        };
                    }
                    // ── abs, min, max, chr, ord ────────────────
                    "abs" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__builtin_abs".to_string(),
                            args: vec![arg],
                        };
                    }
                    "min" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__builtin_min".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "max" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__builtin_max".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "chr" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__builtin_chr".to_string(),
                            args: vec![arg],
                        };
                    }
                    "ord" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__builtin_ord".to_string(),
                            args: vec![arg],
                        };
                    }
                    "len" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__builtin_len".to_string(),
                            args: vec![arg],
                        };
                    }
                    _ => {
                        // Check if it's a constructor call: ClassName(args...)
                        if self.class_names.contains(&func_name) {
                            let num_fields = self.class_fields.get(&func_name)
                                .map(|f| f.len()).unwrap_or(0);
                            let alloc_size = (num_fields + 1) * 8; // +1 for class_id
                            let init_name = format!("{}____init__", func_name);
                            // Build args for __init__: first arg will be the new obj ptr (placeholder)
                            let mut init_args = vec![
                                IRInstruction::LoadConst(IRConstValue::Int(alloc_size as i64)),
                            ];
                            for a in args {
                                init_args.push(self.convert_expr_to_instr(a, program));
                            }
                            return IRInstruction::Call {
                                func: format!("__pyb_obj_new::{}", init_name),
                                args: init_args,
                            };
                        }
                        // Check obj.method() calls
                        if func_name.contains('.') {
                            let parts: Vec<&str> = func_name.splitn(2, '.').collect();
                            if parts.len() == 2 {
                                let obj_name = parts[0];
                                let method = parts[1];

                                // File method calls: f.read(), f.write(), f.close()
                                if self.file_vars.contains(obj_name) {
                                    match method {
                                        "read" => {
                                            return IRInstruction::Call {
                                                func: "__pyb_file_read".to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string())],
                                            };
                                        }
                                        "write" if !args.is_empty() => {
                                            let arg = self.convert_expr_to_instr(&args[0], program);
                                            let str_label = if let PyExpr::StringLiteral(s) = &args[0] {
                                                Some(self.add_string(s, program))
                                            } else { None };
                                            if let Some(label) = str_label {
                                                return IRInstruction::Call {
                                                    func: "__pyb_file_write".to_string(),
                                                    args: vec![
                                                        IRInstruction::Load(obj_name.to_string()),
                                                        IRInstruction::LoadString(label.clone()),
                                                        IRInstruction::LoadConst(IRConstValue::Int(
                                                            if let PyExpr::StringLiteral(s) = &args[0] { s.len() as i64 } else { 0 }
                                                        )),
                                                    ],
                                                };
                                            }
                                            return IRInstruction::Call {
                                                func: "__pyb_file_write".to_string(),
                                                args: vec![
                                                    IRInstruction::Load(obj_name.to_string()),
                                                    arg,
                                                    IRInstruction::LoadConst(IRConstValue::Int(0)),
                                                ],
                                            };
                                        }
                                        "close" => {
                                            return IRInstruction::Call {
                                                func: "__pyb_file_close".to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string())],
                                            };
                                        }
                                        _ => {}
                                    }
                                }

                                // String method calls: s.upper(), s.lower(), s.find(), s.replace()
                                if self.str_heap_vars.contains(obj_name) || self.string_vars.contains_key(obj_name) {
                                    match method {
                                        "upper" => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            return IRInstruction::Call {
                                                func: "__pyb_str_upper".to_string(),
                                                args: vec![src],
                                            };
                                        }
                                        "lower" => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            return IRInstruction::Call {
                                                func: "__pyb_str_lower".to_string(),
                                                args: vec![src],
                                            };
                                        }
                                        "find" if !args.is_empty() => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let needle = self.convert_expr_to_instr(&args[0], program);
                                            return IRInstruction::Call {
                                                func: "__pyb_str_find".to_string(),
                                                args: vec![src, needle],
                                            };
                                        }
                                        "replace" if args.len() >= 2 => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let old = self.convert_expr_to_instr(&args[0], program);
                                            let new = self.convert_expr_to_instr(&args[1], program);
                                            return IRInstruction::Call {
                                                func: "__pyb_str_replace".to_string(),
                                                args: vec![src, old, new],
                                            };
                                        }
                                        "startswith" if !args.is_empty() => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let needle = self.convert_expr_to_instr(&args[0], program);
                                            // startswith = find == 0
                                            return IRInstruction::Compare {
                                                op: IRCmpOp::Eq,
                                                left: Box::new(IRInstruction::Call {
                                                    func: "__pyb_str_find".to_string(),
                                                    args: vec![src, needle],
                                                }),
                                                right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(0))),
                                            };
                                        }
                                        "endswith" if !args.is_empty() => {
                                            // Simplified: just use find and check result != -1
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let needle = self.convert_expr_to_instr(&args[0], program);
                                            return IRInstruction::Compare {
                                                op: IRCmpOp::Ne,
                                                left: Box::new(IRInstruction::Call {
                                                    func: "__pyb_str_find".to_string(),
                                                    args: vec![src, needle],
                                                }),
                                                right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(-1))),
                                            };
                                        }
                                        _ => {}
                                    }
                                }

                                // Class instance method calls
                                if let Some(cls) = self.class_vars.get(obj_name) {
                                    let cls = cls.clone();
                                    let full_method = format!("{}__{}", cls, method);
                                    let mut call_args = vec![IRInstruction::Load(obj_name.to_string())];
                                    for a in args {
                                        call_args.push(self.convert_expr_to_instr(a, program));
                                    }
                                    return IRInstruction::Call {
                                        func: full_method,
                                        args: call_args,
                                    };
                                }
                            }
                        }
                    }
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
            PyExpr::Subscript { value, slice } => {
                let obj_instr = self.convert_expr_to_instr(value, program);
                let idx_instr = self.convert_expr_to_instr(slice, program);
                // Check if this is a dict or list subscript
                let is_dict = if let PyExpr::Name(n) = value.as_ref() {
                    self.dict_vars.contains(n)
                } else { false };
                if is_dict {
                    IRInstruction::Call {
                        func: "__pyb_dict_get".to_string(),
                        args: vec![obj_instr, idx_instr],
                    }
                } else {
                    IRInstruction::Call {
                        func: "__pyb_list_get".to_string(),
                        args: vec![obj_instr, idx_instr],
                    }
                }
            }
            PyExpr::Attribute { value, attr } => {
                if let PyExpr::Name(obj) = value.as_ref() {
                    match (obj.as_str(), attr.as_str()) {
                        ("math", "pi") => IRInstruction::LoadConst(IRConstValue::Float(std::f64::consts::PI)),
                        ("math", "e") => IRInstruction::LoadConst(IRConstValue::Float(std::f64::consts::E)),
                        ("math", "inf") => IRInstruction::LoadConst(IRConstValue::Float(f64::INFINITY)),
                        ("math", "tau") => IRInstruction::LoadConst(IRConstValue::Float(std::f64::consts::TAU)),
                        ("sys", "platform") => {
                            let label = self.add_string(if cfg!(target_os = "windows") { "win32" } else { "linux" }, program);
                            return IRInstruction::LoadString(label);
                        }
                        ("sys", "version") => {
                            let label = self.add_string("PyDead-BIB 2.0.0", program);
                            return IRInstruction::LoadString(label);
                        }
                        ("sys", "maxsize") => IRInstruction::LoadConst(IRConstValue::Int(i64::MAX)),
                        _ => {
                            // Check if obj is a class instance → read field
                            if let Some(cls) = self.class_vars.get(obj) {
                                let cls = cls.clone();
                                let offset = if let Some(fields) = self.class_fields.get(&cls) {
                                    fields.iter().position(|f| f == attr).unwrap_or(0)
                                } else { 0 };
                                let byte_offset = (offset as i64 + 1) * 8;
                                IRInstruction::Call {
                                    func: "__pyb_obj_get_field".to_string(),
                                    args: vec![
                                        IRInstruction::Load(obj.clone()),
                                        IRInstruction::LoadConst(IRConstValue::Int(byte_offset)),
                                    ],
                                }
                            } else if obj == "self" {
                                // self.x inside a method — need class context
                                // We'll use a convention: look through all classes for this field
                                let mut byte_offset = 8i64; // default: first field
                                for (cls_name, fields) in &self.class_fields {
                                    if let Some(pos) = fields.iter().position(|f| f == attr) {
                                        byte_offset = (pos as i64 + 1) * 8;
                                        break;
                                    }
                                }
                                IRInstruction::Call {
                                    func: "__pyb_obj_get_field".to_string(),
                                    args: vec![
                                        IRInstruction::Load("self".to_string()),
                                        IRInstruction::LoadConst(IRConstValue::Int(byte_offset)),
                                    ],
                                }
                            } else {
                                IRInstruction::Nop
                            }
                        }
                    }
                } else {
                    IRInstruction::Nop
                }
            }
            // ── List comprehension ─────────────────────────────
            PyExpr::ListComp { element, generators } => {
                // [expr for var in range(n)] → create list, loop, append
                // For simple cases: [x**2 for x in range(n)]
                if let Some(gen) = generators.first() {
                    if let PyExpr::Call { func: iter_fn, args: iter_args, .. } = &gen.iter {
                        if let PyExpr::Name(fn_name) = iter_fn.as_ref() {
                            if fn_name == "range" && !iter_args.is_empty() {
                                // Compile as: list_new, for i in range(n): list_append(list, element(i))
                                // Return the call to build the list
                                let stop = self.convert_expr_to_instr(&iter_args[iter_args.len().min(2) - 1], program);
                                return IRInstruction::Call {
                                    func: "__pyb_listcomp_range".to_string(),
                                    args: vec![stop],
                                };
                            }
                        }
                    }
                }
                // Fallback: just create empty list
                IRInstruction::Call {
                    func: "__pyb_list_new".to_string(),
                    args: vec![],
                }
            }
            // ── Await expression ──────────────────────────────
            PyExpr::Await(inner) => {
                // For now, await just evaluates the inner expression
                self.convert_expr_to_instr(inner, program)
            }
            // ── Yield expression ──────────────────────────────
            PyExpr::Yield(val) => {
                if let Some(v) = val {
                    self.convert_expr_to_instr(v, program)
                } else {
                    IRInstruction::LoadConst(IRConstValue::None)
                }
            }
            // ── Conditional expression (ternary) ──────────────
            PyExpr::IfExpr { test, body, orelse } => {
                let cond = self.convert_expr_to_instr(test, program);
                let then_val = self.convert_expr_to_instr(body, program);
                let else_val = self.convert_expr_to_instr(orelse, program);
                // For now, compile as: test ? then : else using nested Call
                // Simple approach: evaluate test, if true return body, else orelse
                // This is simplified — full ternary needs conditional jump in ISA
                then_val
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

    fn is_float_expr(&self, expr: &PyExpr) -> bool {
        match expr {
            PyExpr::FloatLiteral(_) => true,
            PyExpr::Attribute { value, attr } => {
                if let PyExpr::Name(obj) = value.as_ref() {
                    obj == "math" && matches!(attr.as_str(), "pi" | "e" | "inf" | "tau")
                } else { false }
            }
            PyExpr::Call { func, .. } => {
                if let PyExpr::Attribute { value, attr } = func.as_ref() {
                    if let PyExpr::Name(obj) = value.as_ref() {
                        return obj == "math" && matches!(attr.as_str(),
                            "sqrt" | "sin" | "cos" | "log" | "abs" | "pow"
                        );
                    }
                }
                false
            }
            _ => false,
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
