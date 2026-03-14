// ============================================================
// PyDead-BIB IR (Intermediate Representation)
// ============================================================
// ADeadOp SSA-form — heredado de ADead-BIB v8.0
// Tipos explícitos en cada instrucción
// BasicBlocks — sin ambigüedad semántica
// GIL eliminado: cada objeto tiene ownership ✓
// ============================================================

/// IR Type — maps Python types to machine types
#[derive(Debug, Clone, PartialEq)]
pub enum IRType {
    Void,
    I8,      // bool
    I16,
    I32,
    I64,     // int (default)
    I128,
    F32,
    F64,     // float (default)
    Ptr,     // str, list, dict, object references
    Vec256,  // YMM 256-bit (SIMD)
}

impl IRType {
    pub fn byte_size(&self) -> usize {
        match self {
            IRType::Void => 0,
            IRType::I8 => 1,
            IRType::I16 => 2,
            IRType::I32 => 4,
            IRType::I64 => 8,
            IRType::I128 => 16,
            IRType::F32 => 4,
            IRType::F64 => 8,
            IRType::Ptr => 8,
            IRType::Vec256 => 32,
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, IRType::I8 | IRType::I16 | IRType::I32 | IRType::I64 | IRType::I128)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, IRType::F32 | IRType::F64)
    }
}

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

/// IR Instruction — SSA-form operations
#[derive(Debug, Clone)]
pub enum IRInstruction {
    // Constants
    LoadConst(IRConstValue),
    LoadString(String),     // label in .data
    Load(String),           // load variable
    Store(String),          // store to variable

    // Variable declaration
    VarDecl { name: String, ir_type: IRType },

    // Arithmetic
    BinOp { op: IROp, left: Box<IRInstruction>, right: Box<IRInstruction> },
    Compare { op: IRCmpOp, left: Box<IRInstruction>, right: Box<IRInstruction> },

    // Control flow
    Label(String),
    Jump(String),
    BranchIfFalse(String),
    Return,
    ReturnVoid,
    Break,
    Continue,

    // Function call
    Call { func: String, args: Vec<IRInstruction> },

    // Iterator
    IterNext { target: String, end_label: String },

    // No-op
    Nop,
}

/// Constant value in IR
#[derive(Debug, Clone)]
pub enum IRConstValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    None,
}

/// IR binary operation
#[derive(Debug, Clone, Copy)]
pub enum IROp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    Shl,
    Shr,
    And,
    Or,
    Xor,
    MatMul,
}

/// IR comparison operation
#[derive(Debug, Clone, Copy)]
pub enum IRCmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
}
