// ============================================================
// Python Type Inferencer for PyDead-BIB
// ============================================================
// Duck typing → concrete static types
// PEP 484 type hints → guaranteed types
// Type propagation: a = 1 → a: int64 inferred
// Return type inference: def f() → return type
// Container types: list[int], dict[str, float]
// Gradual typing: typed + untyped coexist
// ============================================================

use super::py_ast::*;
use std::collections::HashMap;

/// Concrete type after inference (maps to IR types)
#[derive(Debug, Clone, PartialEq)]
pub enum ConcreteType {
    Int64,
    Float64,
    Bool,
    Str,
    Bytes,
    NoneType,
    List(Box<ConcreteType>),
    Dict(Box<ConcreteType>, Box<ConcreteType>),
    Set(Box<ConcreteType>),
    Tuple(Vec<ConcreteType>),
    Object(String),    // class instance
    Function {
        params: Vec<ConcreteType>,
        ret: Box<ConcreteType>,
    },
    Dynamic,           // could not infer — fallback
}

/// Type inference result for a scope
#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub bindings: HashMap<String, ConcreteType>,
    pub functions: HashMap<String, ConcreteType>,
}

/// Python type inferencer
pub struct PyTypeInferencer {
    env_stack: Vec<TypeEnv>,
}

impl PyTypeInferencer {
    pub fn new() -> Self {
        let mut global = TypeEnv {
            bindings: HashMap::new(),
            functions: HashMap::new(),
        };

        // Built-in functions
        global.functions.insert("print".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::NoneType),
        });
        global.functions.insert("len".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Int64),
        });
        global.functions.insert("range".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Int64],
            ret: Box::new(ConcreteType::List(Box::new(ConcreteType::Int64))),
        });
        global.functions.insert("int".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Int64),
        });
        global.functions.insert("float".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Float64),
        });
        global.functions.insert("str".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Str),
        });
        global.functions.insert("bool".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Bool),
        });

        Self {
            env_stack: vec![global],
        }
    }

    /// Infer types for entire module — returns typed AST (passthrough for now)
    pub fn infer(&mut self, module: &PyModule) -> PyModule {
        let mut typed_module = module.clone();
        for stmt in &typed_module.body {
            self.infer_stmt(stmt);
        }
        typed_module
    }

    /// Infer types in a statement
    fn infer_stmt(&mut self, stmt: &PyStmt) {
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
            PyStmt::ClassDef { name, body, .. } => {
                self.current_env_mut().bindings.insert(name.clone(), ConcreteType::Object(name.clone()));
                for s in body { self.infer_stmt(s); }
            }
            _ => {}
        }
    }

    /// Infer type of expression
    fn infer_expr(&self, expr: &PyExpr) -> ConcreteType {
        match expr {
            PyExpr::IntLiteral(_) => ConcreteType::Int64,
            PyExpr::FloatLiteral(_) => ConcreteType::Float64,
            PyExpr::BoolLiteral(_) => ConcreteType::Bool,
            PyExpr::StringLiteral(_) | PyExpr::FString { .. } => ConcreteType::Str,
            PyExpr::BytesLiteral(_) => ConcreteType::Bytes,
            PyExpr::NoneLiteral => ConcreteType::NoneType,
            PyExpr::Name(name) => {
                self.lookup_var(name).unwrap_or(ConcreteType::Dynamic)
            }
            PyExpr::BinOp { op, left, right } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                self.infer_binop(op, &lt, &rt)
            }
            PyExpr::UnaryOp { op: _, operand } => self.infer_expr(operand),
            PyExpr::BoolOp { .. } => ConcreteType::Bool,
            PyExpr::Compare { .. } => ConcreteType::Bool,
            PyExpr::Call { func, .. } => {
                if let PyExpr::Name(name) = func.as_ref() {
                    self.lookup_function_return(name).unwrap_or(ConcreteType::Dynamic)
                } else {
                    ConcreteType::Dynamic
                }
            }
            PyExpr::List(elts) => {
                let elem_type = elts.first()
                    .map(|e| self.infer_expr(e))
                    .unwrap_or(ConcreteType::Dynamic);
                ConcreteType::List(Box::new(elem_type))
            }
            PyExpr::Dict { keys, values } => {
                let kt = keys.first().and_then(|k| k.as_ref())
                    .map(|k| self.infer_expr(k))
                    .unwrap_or(ConcreteType::Dynamic);
                let vt = values.first()
                    .map(|v| self.infer_expr(v))
                    .unwrap_or(ConcreteType::Dynamic);
                ConcreteType::Dict(Box::new(kt), Box::new(vt))
            }
            PyExpr::Tuple(elts) => {
                let types: Vec<ConcreteType> = elts.iter().map(|e| self.infer_expr(e)).collect();
                ConcreteType::Tuple(types)
            }
            PyExpr::Subscript { value, .. } => {
                let vt = self.infer_expr(value);
                match vt {
                    ConcreteType::List(inner) => *inner,
                    ConcreteType::Dict(_, val) => *val,
                    ConcreteType::Str => ConcreteType::Str,
                    _ => ConcreteType::Dynamic,
                }
            }
            PyExpr::Attribute { .. } => ConcreteType::Dynamic,
            _ => ConcreteType::Dynamic,
        }
    }

    fn infer_binop(&self, op: &PyBinOp, left: &ConcreteType, right: &ConcreteType) -> ConcreteType {
        match (left, right) {
            (ConcreteType::Int64, ConcreteType::Int64) => match op {
                PyBinOp::Div => ConcreteType::Float64,
                _ => ConcreteType::Int64,
            },
            (ConcreteType::Float64, _) | (_, ConcreteType::Float64) => ConcreteType::Float64,
            (ConcreteType::Str, ConcreteType::Str) if *op == PyBinOp::Add => ConcreteType::Str,
            (ConcreteType::Str, ConcreteType::Int64) if *op == PyBinOp::Mul => ConcreteType::Str,
            (ConcreteType::List(_), ConcreteType::List(_)) if *op == PyBinOp::Add => left.clone(),
            _ => ConcreteType::Dynamic,
        }
    }

    fn annotation_to_concrete(&self, ann: &PyType) -> ConcreteType {
        match ann {
            PyType::Int => ConcreteType::Int64,
            PyType::Float => ConcreteType::Float64,
            PyType::Str => ConcreteType::Str,
            PyType::Bool => ConcreteType::Bool,
            PyType::None => ConcreteType::NoneType,
            PyType::Bytes => ConcreteType::Bytes,
            PyType::List(inner) => ConcreteType::List(Box::new(self.annotation_to_concrete(inner))),
            PyType::Dict(k, v) => ConcreteType::Dict(
                Box::new(self.annotation_to_concrete(k)),
                Box::new(self.annotation_to_concrete(v)),
            ),
            PyType::Set(inner) => ConcreteType::Set(Box::new(self.annotation_to_concrete(inner))),
            PyType::Tuple(elts) => ConcreteType::Tuple(elts.iter().map(|e| self.annotation_to_concrete(e)).collect()),
            PyType::Optional(inner) => self.annotation_to_concrete(inner),
            PyType::Any | PyType::Inferred => ConcreteType::Dynamic,
            PyType::Custom(name) => ConcreteType::Object(name.clone()),
            _ => ConcreteType::Dynamic,
        }
    }

    fn lookup_var(&self, name: &str) -> Option<ConcreteType> {
        for env in self.env_stack.iter().rev() {
            if let Some(t) = env.bindings.get(name) {
                return Some(t.clone());
            }
        }
        std::option::Option::None
    }

    fn lookup_function_return(&self, name: &str) -> Option<ConcreteType> {
        for env in self.env_stack.iter().rev() {
            if let Some(ConcreteType::Function { ret, .. }) = env.functions.get(name) {
                return Some(*ret.clone());
            }
        }
        std::option::Option::None
    }

    fn current_env_mut(&mut self) -> &mut TypeEnv {
        self.env_stack.last_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_int_literal() {
        let inf = PyTypeInferencer::new();
        let t = inf.infer_expr(&PyExpr::IntLiteral(42));
        assert_eq!(t, ConcreteType::Int64);
    }

    #[test]
    fn test_infer_float_promotion() {
        let inf = PyTypeInferencer::new();
        let t = inf.infer_binop(
            &PyBinOp::Add,
            &ConcreteType::Int64,
            &ConcreteType::Float64,
        );
        assert_eq!(t, ConcreteType::Float64);
    }

    #[test]
    fn test_infer_div_returns_float() {
        let inf = PyTypeInferencer::new();
        let t = inf.infer_binop(
            &PyBinOp::Div,
            &ConcreteType::Int64,
            &ConcreteType::Int64,
        );
        assert_eq!(t, ConcreteType::Float64);
    }

    #[test]
    fn test_infer_string_concat() {
        let inf = PyTypeInferencer::new();
        let t = inf.infer_binop(
            &PyBinOp::Add,
            &ConcreteType::Str,
            &ConcreteType::Str,
        );
        assert_eq!(t, ConcreteType::Str);
    }
}
