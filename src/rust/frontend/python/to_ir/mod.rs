pub mod core;
pub mod stmt;
pub mod expr;

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

use super::ast::*;
use crate::middle::ir::{IRCmpOp, IRConstValue, IRFunction, IRInstruction, IRModule, IROp, IRType};

use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn fresh_temp(prefix: &str) -> String {
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
    list_vars: std::collections::HashSet<String>, // variables that are lists
    file_vars: std::collections::HashSet<String>, // variables that are file handles
    str_heap_vars: std::collections::HashSet<String>, // variables that are heap strings
    except_label_stack: Vec<String>, // stack of except handler labels for raise
    class_fields: std::collections::HashMap<String, Vec<String>>, // class_name → ordered field names
    class_vars: std::collections::HashMap<String, String>, // var_name → class_name
    class_names: std::collections::HashSet<String>, // known class names
    global_vars: std::collections::HashSet<String>, // variables declared 'global' in current function
    all_globals: std::collections::HashSet<String>, // all global variables across all functions
}

impl PyToIR {
    pub fn new() -> Self {
        Self {
            string_counter: 0,
            string_vars: std::collections::HashMap::new(),
            dict_vars: std::collections::HashSet::new(),
            list_vars: std::collections::HashSet::new(),
            file_vars: std::collections::HashSet::new(),
            str_heap_vars: std::collections::HashSet::new(),
            except_label_stack: Vec::new(),
            class_fields: std::collections::HashMap::new(),
            class_vars: std::collections::HashMap::new(),
            class_names: std::collections::HashSet::new(),
            global_vars: std::collections::HashSet::new(),
            all_globals: std::collections::HashSet::new(),
        }
    }
}

/// Main entry: Convert Python module → IR program
pub fn compile_python_to_ir(module: &PyModule) -> Result<IRProgram, Box<dyn std::error::Error>> {
    let mut converter = PyToIR::new();
    converter.convert(module).map_err(|e| e.into())
}
