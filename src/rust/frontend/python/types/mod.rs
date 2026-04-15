pub mod concrete;
pub mod layout;
pub mod infer_stmt;
pub mod infer_expr;
pub mod strict;

pub use concrete::ConcreteType;
pub use layout::{StructField, StructLayout};
pub use strict::*;
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

use super::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub bindings: HashMap<String, ConcreteType>,
    pub functions: HashMap<String, ConcreteType>,
}

/// Python type inferencer
pub struct PyTypeInferencer {
    env_stack: Vec<TypeEnv>,
    // v4.0 FASE 3: Class registry
    pub class_layouts: HashMap<String, StructLayout>,
    pub inheritance_chain: HashMap<String, Vec<String>>, // class → [parent, grandparent, ...]
    pub dynamic_fallbacks: Vec<String>, // compile-time warnings for Dynamic fields
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
            class_layouts: HashMap::new(),
            inheritance_chain: HashMap::new(),
            dynamic_fallbacks: Vec::new(),
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

}
