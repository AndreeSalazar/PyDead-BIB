use super::PyToIR;
use crate::frontend::python::ast::*;
use crate::middle::ir::*;
use super::{IRProgram, IRGlobal, IRConstant, fresh_temp};

impl PyToIR {
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

        // Pre-scan top-level assigns → register as all_globals so functions can access them
        for stmt in &toplevel_stmts {
            if let PyStmt::Assign { targets, .. } = stmt {
                for t in targets {
                    if let PyExpr::Name(name) = t {
                        self.all_globals.insert(name.clone());
                    }
                }
            }
        }

        // Second pass: generate __main__ for top-level expressions
        if !toplevel_stmts.is_empty() {
            // Pre-scan: register all top-level assignments as globals
            // This ensures variables like `p = Point(...)` use GlobalStore/GlobalLoad
            for stmt in &toplevel_stmts {
                if let PyStmt::Assign { targets, .. } = stmt {
                    for target in targets {
                        if let PyExpr::Name(name) = target {
                            self.all_globals.insert(name.clone());
                        }
                    }
                }
            }
            
            let mut main_func = IRFunction::new("__main__".to_string(), vec![], IRType::Void);
            for stmt in &toplevel_stmts {
                self.convert_body_stmt(stmt, &mut main_func, &mut program)?;
            }
            main_func.body.push(IRInstruction::ReturnVoid);
            program.functions.push(main_func);
        }

        // Populate program.globals from all_globals
        for gname in &self.all_globals {
            program.globals.push(IRGlobal {
                name: gname.clone(),
                ir_type: IRType::I64,
                init_value: Some(IRConstant::Int(0)),
            });
        }

        Ok(program)
    }

    pub fn convert_stmt(&mut self, stmt: &PyStmt, program: &mut IRProgram) -> Result<(), String> {
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

                // Save and reset per-function global_vars scope
                let saved_globals = self.global_vars.clone();
                self.global_vars.clear();

                // Pre-scan for 'global' declarations in this function
                for s in body {
                    if let PyStmt::Global(names) = s {
                        for gn in names {
                            self.global_vars.insert(gn.clone());
                            self.all_globals.insert(gn.clone());
                        }
                    }
                }

                for s in body {
                    self.convert_body_stmt(s, &mut func, program)?;
                }

                // Restore outer scope
                self.global_vars = saved_globals;

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

    pub fn convert_class_body_stmt(&mut self, stmt: &PyStmt, func: &mut IRFunction, program: &mut IRProgram, class_name: &str) -> Result<(), String> {
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

    pub fn add_string(&mut self, s: &str, program: &mut IRProgram) -> String {
        let label = format!("__str{}", self.string_counter);
        self.string_counter += 1;
        program.string_data.push((label.clone(), s.to_string()));
        label
    }

    pub fn pytype_to_ir(&self, py_type: &PyType) -> IRType {
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

}
