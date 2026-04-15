use super::{PyTypeInferencer, TypeEnv};
use std::collections::HashMap;
use super::concrete::ConcreteType;
use crate::frontend::python::ast::*;
use super::layout::StructLayout;

impl PyTypeInferencer {
    pub fn infer_stmt(&mut self, stmt: &PyStmt) {
        match stmt {
            PyStmt::Assign { targets, value } => {
                let val_type = self.infer_expr(value);
                for target in targets {
                    if let PyExpr::Name(name) = target {
                        self.current_env_mut().bindings.insert(name.clone(), val_type.clone());
                    }
                }
            }
            PyStmt::AnnAssign { target, annotation, value } => {
                let ann_type = self.annotation_to_concrete(annotation);
                if let PyExpr::Name(name) = target {
                    self.current_env_mut().bindings.insert(name.clone(), ann_type);
                }
                if let Some(val) = value {
                    self.infer_expr(val);
                }
            }
            PyStmt::FunctionDef { name, params, body, return_type, .. } => {
                let param_types: Vec<ConcreteType> = params.iter().map(|p| {
                    p.annotation.as_ref()
                        .map(|a| self.annotation_to_concrete(a))
                        .unwrap_or(ConcreteType::Dynamic)
                }).collect();
                let ret = return_type.as_ref()
                    .map(|r| self.annotation_to_concrete(r))
                    .unwrap_or(ConcreteType::Dynamic);
                self.current_env_mut().functions.insert(name.clone(), ConcreteType::Function {
                    params: param_types,
                    ret: Box::new(ret),
                });

                // Push scope for function body
                self.env_stack.push(TypeEnv {
                    bindings: HashMap::new(),
                    functions: HashMap::new(),
                });
                for s in body {
                    self.infer_stmt(s);
                }
                self.env_stack.pop();
            }
            PyStmt::If { test, body, elif_clauses, orelse } => {
                self.infer_expr(test);
                for s in body { self.infer_stmt(s); }
                for (t, b) in elif_clauses {
                    self.infer_expr(t);
                    for s in b { self.infer_stmt(s); }
                }
                for s in orelse { self.infer_stmt(s); }
            }
            PyStmt::While { test, body, orelse } => {
                self.infer_expr(test);
                for s in body { self.infer_stmt(s); }
                for s in orelse { self.infer_stmt(s); }
            }
            PyStmt::For { target: _, iter, body, orelse, .. } => {
                self.infer_expr(iter);
                for s in body { self.infer_stmt(s); }
                for s in orelse { self.infer_stmt(s); }
            }
            PyStmt::ClassDef { name, bases, body, .. } => {
                self.current_env_mut().bindings.insert(name.clone(), ConcreteType::Object(name.clone()));

                // v4.0 FASE 3: Build StructLayout with deep __init__ inference
                let mut layout = StructLayout::new(name);

                // Resolve inheritance chain
                let mut chain = Vec::new();
                for base in bases {
                    if let PyExpr::Name(base_name) = base {
                        chain.push(base_name.clone());
                        layout.parent = Some(base_name.clone());
                        // Copy parent fields (deep inheritance)
                        if let Some(parent_layout) = self.class_layouts.get(base_name).cloned() {
                            for field in &parent_layout.fields {
                                layout.add_field(&field.name, field.field_type.clone());
                            }
                            // Also inherit grandparent chain
                            if let Some(parent_chain) = self.inheritance_chain.get(base_name).cloned() {
                                chain.extend(parent_chain);
                            }
                        }
                    }
                }
                self.inheritance_chain.insert(name.clone(), chain);

                // Deep __init__ inference: scan self.x = val assignments
                for s in body {
                    if let PyStmt::FunctionDef { name: method_name, body: method_body, .. } = s {
                        if method_name == "__init__" {
                            for ms in method_body {
                                if let PyStmt::Assign { targets, value } = ms {
                                    for t in targets {
                                        if let PyExpr::Attribute { value: target_obj, attr } = t {
                                            if let PyExpr::Name(n) = target_obj.as_ref() {
                                                if n == "self" {
                                                    // Infer field type from RHS
                                                    let field_type = self.infer_expr(value);
                                                    if field_type == ConcreteType::Dynamic {
                                                        self.dynamic_fallbacks.push(
                                                            format!("{}::{}: type could not be inferred, using Dynamic", name, attr)
                                                        );
                                                    }
                                                    layout.add_field(attr, field_type);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.class_layouts.insert(name.clone(), layout);

                for s in body { self.infer_stmt(s); }
            }
            _ => {}
        }
    }

}
