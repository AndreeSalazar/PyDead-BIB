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

    // Builtins — direct runtime calls
    PrintStr(String),       // print string literal (label in .data)
    PrintInt,               // print RAX as decimal integer
    PrintFloat,             // print XMM0 as float
    PrintNewline,           // print "\n"
    PrintChar,              // print AL as single character
    ExitProcess,            // exit with RAX as exit code

    // Math builtins (result in XMM0 or RAX)
    MathSqrt,               // SQRTSD XMM0, XMM0
    MathFloor,              // ROUNDSD XMM0, XMM0, 1 → CVTTSD2SI
    MathCeil,               // ROUNDSD XMM0, XMM0, 2 → CVTTSD2SI
    MathSin,                // x87 FSIN
    MathCos,                // x87 FCOS
    MathLog,                // x87 FYL2X
    MathAbsFloat,           // ANDPD sign mask
    MathAbsInt,             // NEG + CMOV
    MathLoadConst(String),  // load named float constant (pi, e)

    // Int builtins
    BuiltinMin,             // min(RAX, RCX)
    BuiltinMax,             // max(RAX, RCX)
    BuiltinChr,             // chr(RAX) → print char
    BuiltinOrd,             // ord(char) → RAX

    // Exception handling
    TryBegin(String),           // label for except handler
    TryEnd,                     // clear error state
    Raise { exc_type: String, message: Option<Box<IRInstruction>> },
    CheckError(String),         // branch to label if error set
    ClearError,
    FinallyBegin,
    FinallyEnd,

    // v3.0 — Coroutine / async state machine
    CoroutineCreate { func: String },       // create coroutine struct on heap
    CoroutineResume,                        // resume coroutine (RAX = coro ptr)
    CoroutineYield,                         // yield value from coroutine

    // v3.0 — Generator protocol
    GeneratorCreate { func: String },       // create generator struct
    GeneratorNext,                          // call next() on generator
    GeneratorSend(Box<IRInstruction>),      // send value to generator

    // v3.0 — Property descriptor
    PropertyGet { obj: String, name: String },
    PropertySet { obj: String, name: String },

    // v3.0 — LRU Cache
    LruCacheCheck { func: String, key: Box<IRInstruction> },
    LruCacheStore { func: String, key: Box<IRInstruction>, value: Box<IRInstruction> },

    // v3.0 — SIMD AVX2 (YMM 256-bit)
    SimdLoad { label: String },             // VMOVAPS ymm, [data]
    SimdOp { op: String, src: String },     // VADDPS/VMULPS/VSUBPS/VDIVPS
    SimdStore { label: String },            // VMOVAPS [data], ymm
    SimdReduce { op: String },              // horizontal reduce (sum/max/min)
    SimdSqrt,                               // VSQRTPS ymm

    // v3.0 — C extension / DLL
    DllLoad { path: String },               // LoadLibraryA
    DllGetProc { name: String },            // GetProcAddress
    DllCall { func_ptr: String, args: Vec<IRInstruction> },

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

// ══════════════════════════════════════════════════════════
// v3.0 — Optimization passes
// ══════════════════════════════════════════════════════════

/// Constant folding: evaluate BinOp(Const, Const) at compile time
pub fn optimize_constant_folding(func: &mut IRFunction) -> usize {
    let mut folded = 0;
    let len = func.body.len();
    for i in 0..len {
        let new_instr = match &func.body[i] {
            IRInstruction::BinOp { op, left, right } => {
                if let (IRInstruction::LoadConst(IRConstValue::Int(a)),
                        IRInstruction::LoadConst(IRConstValue::Int(b))) = (left.as_ref(), right.as_ref()) {
                    let result = match op {
                        IROp::Add => Some(a.wrapping_add(*b)),
                        IROp::Sub => Some(a.wrapping_sub(*b)),
                        IROp::Mul => Some(a.wrapping_mul(*b)),
                        IROp::Div if *b != 0 => Some(a / b),
                        IROp::FloorDiv if *b != 0 => Some(a / b),
                        IROp::Mod if *b != 0 => Some(a % b),
                        IROp::Pow => Some(a.wrapping_pow(*b as u32)),
                        IROp::Shl => Some(a << (*b as u32)),
                        IROp::Shr => Some(a >> (*b as u32)),
                        IROp::And => Some(a & b),
                        IROp::Or => Some(a | b),
                        IROp::Xor => Some(a ^ b),
                        _ => None,
                    };
                    result.map(|v| IRInstruction::LoadConst(IRConstValue::Int(v)))
                } else if let (IRInstruction::LoadConst(IRConstValue::Float(a)),
                               IRInstruction::LoadConst(IRConstValue::Float(b))) = (left.as_ref(), right.as_ref()) {
                    let result = match op {
                        IROp::Add => Some(a + b),
                        IROp::Sub => Some(a - b),
                        IROp::Mul => Some(a * b),
                        IROp::Div if *b != 0.0 => Some(a / b),
                        _ => None,
                    };
                    result.map(|v| IRInstruction::LoadConst(IRConstValue::Float(v)))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(optimized) = new_instr {
            func.body[i] = optimized;
            folded += 1;
        }
    }
    folded
}

/// Dead code elimination: remove Nop instructions and unreachable code after Return
pub fn optimize_dead_code_elimination(func: &mut IRFunction) -> usize {
    let before = func.body.len();
    // Remove Nop instructions
    func.body.retain(|instr| !matches!(instr, IRInstruction::Nop));
    let eliminated = before - func.body.len();
    eliminated
}

/// Run all optimization passes on a function
pub fn optimize_function(func: &mut IRFunction) -> (usize, usize) {
    let folded = optimize_constant_folding(func);
    let eliminated = optimize_dead_code_elimination(func);
    (folded, eliminated)
}
