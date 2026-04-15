use super::types::IRType;
use super::opcodes::{IRInstruction, IRConstValue, IROp, IRCmpOp};

/// IR Module — top-level container
#[derive(Debug)]
pub struct IRModule {
    pub name: String,
}

impl IRModule {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

/// IR Function
#[derive(Debug)]
pub struct IRFunction {
    pub name: String,
    pub params: Vec<(String, IRType)>,
    pub return_type: IRType,
    pub body: Vec<IRInstruction>,
}

impl IRFunction {
    pub fn new(name: String, params: Vec<(String, IRType)>, return_type: IRType) -> Self {
        Self { name, params, return_type, body: Vec::new() }
    }
}

