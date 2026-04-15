use super::PyToIR;
use crate::frontend::python::ast::*;
use crate::middle::ir::*;
use super::{IRProgram, IRGlobal, IRConstant, fresh_temp};

impl PyToIR {
    pub fn convert_body_stmt(&mut self, stmt: &PyStmt, func: &mut IRFunction, program: &mut IRProgram) -> Result<(), String> {
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
                                    let set_func = if self.is_str_expr(key_expr) { "__pyb_dict_str_set" } else { "__pyb_dict_set" };
                                    func.body.push(IRInstruction::Call {
                                        func: set_func.to_string(),
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
                            self.list_vars.insert(name.clone());
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
                            // .split() returns list
                            if fn_str.ends_with(".split") {
                                self.list_vars.insert(var_name.clone());
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
                        // v4.3: Track string concatenation results as heap strings
                        if let PyExpr::BinOp { op: PyBinOp::Add, left, right } = value {
                            let left_is_str = self.is_str_expr(left);
                            let right_is_str = self.is_str_expr(right);
                            if left_is_str || right_is_str {
                                self.str_heap_vars.insert(name.clone());
                            }
                        }
                        if self.global_vars.contains(name) || self.all_globals.contains(name) {
                            // Global variable → use GlobalStore (no VarDecl needed)
                            func.body.push(val_instr.clone());
                            func.body.push(IRInstruction::GlobalStore(name.clone()));
                        } else {
                            func.body.push(IRInstruction::VarDecl {
                                name: name.clone(),
                                ir_type: self.infer_expr_type(value),
                            });
                            func.body.push(val_instr.clone());
                            func.body.push(IRInstruction::Store(name.clone()));
                        }
                    } else if let PyExpr::Subscript { value: dict_expr, slice: key_expr } = target {
                        // Dictionary subscript assignment: d[k] = v
                        let dict_instr = self.convert_expr_to_instr(dict_expr, program);
                        let key_instr = self.convert_expr_to_instr(key_expr, program);
                        let set_func = if self.is_str_expr(key_expr) { "__pyb_dict_str_set" } else { "__pyb_dict_set" };
                        func.body.push(IRInstruction::Call {
                            func: set_func.to_string(),
                            args: vec![
                                dict_instr,
                                key_instr,
                                val_instr.clone(),
                            ],
                        });
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
                
                // Track list variables for `for x in my_list:`
                let is_list = if let PyExpr::Name(n) = iter {
                    self.list_vars.contains(n)
                } else { false };

                if is_list {
                    // List iteration: 
                    // __idx = 0
                    // __len = __pyb_list_len(list)
                    // label loop:
                    // if __idx >= __len goto end
                    // target = __pyb_list_get(list, __idx)
                    // body
                    // __idx = __idx + 1
                    // goto loop
                    // label end:
                    let idx_var = fresh_temp("idx");
                    let len_var = fresh_temp("len");
                    
                    func.body.push(IRInstruction::VarDecl { name: idx_var.clone(), ir_type: IRType::I64 });
                    func.body.push(IRInstruction::VarDecl { name: len_var.clone(), ir_type: IRType::I64 });
                    
                    if let PyExpr::Name(target_name) = target {
                        func.body.push(IRInstruction::VarDecl { name: target_name.clone(), ir_type: IRType::I64 });
                        
                        // init __idx = 0
                        func.body.push(IRInstruction::LoadConst(IRConstValue::Int(0)));
                        func.body.push(IRInstruction::Store(idx_var.clone()));
                        
                        // init __len = __pyb_list_len(list)
                        let list_instr = self.convert_expr_to_instr(iter, program);
                        func.body.push(IRInstruction::Call {
                            func: "__pyb_list_len".to_string(),
                            args: vec![list_instr.clone()],
                        });
                        func.body.push(IRInstruction::Store(len_var.clone()));
                        
                        let loop_label = fresh_temp("for_list");
                        let end_label = fresh_temp("endfor_list");
                        
                        func.body.push(IRInstruction::Label(loop_label.clone()));
                        
                        // if __idx < __len is false goto end
                        func.body.push(IRInstruction::Compare {
                            op: IRCmpOp::Lt,
                            left: Box::new(IRInstruction::Load(idx_var.clone())),
                            right: Box::new(IRInstruction::Load(len_var.clone())),
                        });
                        func.body.push(IRInstruction::BranchIfFalse(end_label.clone()));
                        
                        // target = __pyb_list_get(list, __idx)
                        func.body.push(IRInstruction::Call {
                            func: "__pyb_list_get".to_string(),
                            args: vec![list_instr, IRInstruction::Load(idx_var.clone())],
                        });
                        func.body.push(IRInstruction::Store(target_name.clone()));
                        
                        // body
                        for s in body {
                            self.convert_body_stmt(s, func, program)?;
                        }
                        
                        // __idx = __idx + 1
                        func.body.push(IRInstruction::BinOp {
                            op: IROp::Add,
                            left: Box::new(IRInstruction::Load(idx_var.clone())),
                            right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(1))),
                        });
                        func.body.push(IRInstruction::Store(idx_var.clone()));
                        
                        func.body.push(IRInstruction::Jump(loop_label));
                        func.body.push(IRInstruction::Label(end_label));
                    }
                } else {
                    // Fallback: generic for (IterNext)
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
                                    // v4.3: Check if arg is a Name that refers to a string variable
                                    PyExpr::Name(var_name) => {
                                        // First check if it's a known string literal variable
                                        if let Some(label) = self.string_vars.get(var_name).cloned() {
                                            func.body.push(IRInstruction::PrintStr(label));
                                        } else if self.str_heap_vars.contains(var_name) {
                                            // Heap string variable
                                            let instr = self.convert_expr_to_instr(arg, program);
                                            func.body.push(instr);
                                            func.body.push(IRInstruction::Call {
                                                func: "__pyb_str_print".to_string(),
                                                args: vec![],
                                            });
                                        } else if self.list_vars.contains(var_name) {
                                            // List variable
                                            let instr = self.convert_expr_to_instr(arg, program);
                                            func.body.push(instr);
                                            func.body.push(IRInstruction::Call {
                                                func: "__pyb_list_print".to_string(),
                                                args: vec![],
                                            });
                                        } else {
                                            // Assume integer variable
                                            let instr = self.convert_expr_to_instr(arg, program);
                                            func.body.push(instr);
                                            func.body.push(IRInstruction::PrintInt);
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
                                                "json.dumps" | "sys.platform" | "sys.version" |
                                                "str"
                                            ) || fn_str.ends_with(".upper") || fn_str.ends_with(".lower")
                                              || fn_str.ends_with(".replace") || fn_str.ends_with(".read")
                                              || fn_str.ends_with(".strip")
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
                                            // v4.5: Check if arg is a list variable → print via __pyb_list_print
                                            let is_list_var = if let PyExpr::Name(n) = arg {
                                                self.list_vars.contains(n)
                                            } else { false };
                                            // Check if arg is a bool-producing call (startswith, endswith, etc)
                                            let is_bool_call = if let PyExpr::Call { func: f, .. } = arg {
                                                let fn_str = match f.as_ref() {
                                                    PyExpr::Attribute { value: _, attr: a } => {
                                                        matches!(a.as_str(), "startswith" | "endswith")
                                                    }
                                                    _ => false,
                                                };
                                                fn_str
                                            } else { false };
                                            
                                            if is_list_var {
                                                let instr = self.convert_expr_to_instr(arg, program);
                                                func.body.push(instr);
                                                func.body.push(IRInstruction::Call {
                                                    func: "__pyb_list_print".to_string(),
                                                    args: vec![],
                                                });
                                            } else if is_bool_call {
                                                let instr = self.convert_expr_to_instr(arg, program);
                                                func.body.push(instr);
                                                // Print True or False based on RAX value
                                                let true_label = fresh_temp("bool_true");
                                                let done_label = fresh_temp("bool_done");
                                                func.body.push(IRInstruction::BranchIfFalse(true_label.clone()));
                                                let t_lbl = self.add_string("True", program);
                                                func.body.push(IRInstruction::PrintStr(t_lbl));
                                                func.body.push(IRInstruction::Jump(done_label.clone()));
                                                func.body.push(IRInstruction::Label(true_label));
                                                let f_lbl = self.add_string("False", program);
                                                func.body.push(IRInstruction::PrintStr(f_lbl));
                                                func.body.push(IRInstruction::Label(done_label));
                                            } else {
                                                let instr = self.convert_expr_to_instr(arg, program);
                                                func.body.push(instr);
                                                func.body.push(IRInstruction::PrintInt);
                                            }
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
                    if self.global_vars.contains(name) || self.all_globals.contains(name) {
                        // Global variable → use GlobalLoad/GlobalStore
                        func.body.push(IRInstruction::BinOp {
                            op: self.convert_binop(op),
                            left: Box::new(IRInstruction::GlobalLoad(name.clone())),
                            right: Box::new(val_instr),
                        });
                        func.body.push(IRInstruction::GlobalStore(name.clone()));
                    } else {
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
            }
            PyStmt::Global(names) => {
                // Already pre-scanned in convert_stmt; register globals in .data
                for gn in names {
                    self.global_vars.insert(gn.clone());
                    self.all_globals.insert(gn.clone());
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

}
