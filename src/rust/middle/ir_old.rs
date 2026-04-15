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

    // v3.0 — C extension / DLL (C ABI real)
    DllLoad { path: String },                           // LoadLibraryA → RAX = HMODULE
    DllGetProc { module: String, name: String },        // GetProcAddress(hModule, name) → RAX = func_ptr
    DllCall { func_ptr: String, args: Vec<IRInstruction> }, // CALL func_ptr with args
    DllFree { module: String },                         // FreeLibrary(hModule)

    // v4.1 — ctypes C ABI types
    CStructAlloc { name: String, size: usize },         // HeapAlloc(size) → RAX = struct ptr
    CStructSetField { offset: usize, value: Box<IRInstruction> }, // MOV [RAX+offset], value
    CStructGetField { offset: usize },                  // MOV RAX, [RCX+offset]
    CPointerAlloc { inner_size: usize },                // HeapAlloc(8) → RAX = ptr to ptr
    CPointerDeref,                                      // MOV RAX, [RAX]
    CPointerSet { value: Box<IRInstruction> },          // MOV [RAX], value
    CByRef { var: String },                             // LEA RAX, [var] — pass by reference

    // v4.2 — ctypes extended types
    CCharP { value: Box<IRInstruction> },               // c_char_p — string → null-terminated ptr
    CVoidP { value: Box<IRInstruction> },               // c_void_p — generic pointer
    CArrayAlloc { elem_size: usize, count: usize },     // Array allocation
    CArraySet { elem_size: usize, index: Box<IRInstruction>, value: Box<IRInstruction> },
    CArrayGet { elem_size: usize, index: Box<IRInstruction> },

    // v4.2 — struct module (pack/unpack)
    StructPack { format: String, values: Vec<IRInstruction> },   // struct.pack(fmt, v1, v2, ...)
    StructUnpack { format: String, data: Box<IRInstruction> },   // struct.unpack(fmt, data)

    // v4.0 — Global State Tracker (FASE 1)
    GlobalLoad(String),             // MOV RAX, [__global_name] from .data
    GlobalStore(String),            // MOV [__global_name], RAX to .data

    // v4.0 — GPU Dispatch (FASE 4)
    GpuInit,                                            // cuInit(0) via nvcuda.dll
    GpuDeviceGet,                                       // cuDeviceGet(&dev, 0)
    GpuCtxCreate,                                       // cuCtxCreate(&ctx, 0, dev)
    GpuMalloc { size: Box<IRInstruction> },             // cuMemAlloc(&dptr, size)
    GpuMemcpyHtoD { dst: String, src: String, size: Box<IRInstruction> }, // cuMemcpyHtoD
    GpuMemcpyDtoH { dst: String, src: String, size: Box<IRInstruction> }, // cuMemcpyDtoH
    GpuLaunch { kernel: String, args: Vec<IRInstruction> },              // cuLaunchKernel
    GpuFree { ptr: String },                            // cuMemFree(dptr)
    GpuCtxDestroy,                                      // cuCtxDestroy(ctx)
    GpuAvxToCuda { avx_label: String, gpu_ptr: String, count: Box<IRInstruction> }, // AVX2→CUDA handoff

    // v4.0 — Vulkan/SPIR-V Dispatch
    VkInit,                                             // vkCreateInstance
    VkDeviceGet,                                        // vkEnumeratePhysicalDevices
    VkDeviceCreate,                                     // vkCreateDevice + queue
    VkBufferCreate { size: Box<IRInstruction> },        // vkCreateBuffer + vkAllocateMemory
    VkBufferWrite { dst: String, src: String, size: Box<IRInstruction> },  // vkMapMemory + memcpy
    VkBufferRead { dst: String, src: String, size: Box<IRInstruction> },   // vkMapMemory read back
    VkShaderLoad { spirv_path: String },                // vkCreateShaderModule from SPIR-V
    VkDispatch { shader: String, x: Box<IRInstruction>, y: Box<IRInstruction>, z: Box<IRInstruction> }, // vkCmdDispatch
    VkBufferFree { ptr: String },                       // vkFreeMemory + vkDestroyBuffer
    VkDestroy,                                          // vkDestroyDevice + vkDestroyInstance

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

/// v4.2 — Strength Reduction: replace expensive ops with cheaper ones
/// x * 2 → x << 1, x * 4 → x << 2, x / 2 → x >> 1, etc.
pub fn optimize_strength_reduction(func: &mut IRFunction) -> usize {
    let mut reduced = 0;
    let len = func.body.len();
    for i in 0..len {
        let new_instr = match &func.body[i] {
            IRInstruction::BinOp { op: IROp::Mul, left, right } => {
                // x * 2^n → x << n
                if let IRInstruction::LoadConst(IRConstValue::Int(n)) = right.as_ref() {
                    if *n > 0 && (*n & (*n - 1)) == 0 {
                        // n is power of 2
                        let shift = (*n as u64).trailing_zeros() as i64;
                        Some(IRInstruction::BinOp {
                            op: IROp::Shl,
                            left: left.clone(),
                            right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(shift))),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            IRInstruction::BinOp { op: IROp::Div, left, right } => {
                // x / 2^n → x >> n (for positive integers)
                if let IRInstruction::LoadConst(IRConstValue::Int(n)) = right.as_ref() {
                    if *n > 0 && (*n & (*n - 1)) == 0 {
                        let shift = (*n as u64).trailing_zeros() as i64;
                        Some(IRInstruction::BinOp {
                            op: IROp::Shr,
                            left: left.clone(),
                            right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(shift))),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            IRInstruction::BinOp { op: IROp::Mod, left, right } => {
                // x % 2^n → x & (2^n - 1)
                if let IRInstruction::LoadConst(IRConstValue::Int(n)) = right.as_ref() {
                    if *n > 0 && (*n & (*n - 1)) == 0 {
                        Some(IRInstruction::BinOp {
                            op: IROp::And,
                            left: left.clone(),
                            right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(*n - 1))),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(optimized) = new_instr {
            func.body[i] = optimized;
            reduced += 1;
        }
    }
    reduced
}

/// v4.2 — Integer Overflow Detection (UB)
pub fn detect_integer_overflow(func: &IRFunction) -> Vec<(usize, String)> {
    let mut warnings = Vec::new();
    for (i, instr) in func.body.iter().enumerate() {
        if let IRInstruction::BinOp { op, left, right } = instr {
            if let (IRInstruction::LoadConst(IRConstValue::Int(a)),
                    IRInstruction::LoadConst(IRConstValue::Int(b))) = (left.as_ref(), right.as_ref()) {
                let overflow = match op {
                    IROp::Add => a.checked_add(*b).is_none(),
                    IROp::Sub => a.checked_sub(*b).is_none(),
                    IROp::Mul => a.checked_mul(*b).is_none(),
                    IROp::Pow if *b > 63 => true,
                    IROp::Shl if *b > 63 => true,
                    _ => false,
                };
                if overflow {
                    warnings.push((i, format!("Integer overflow in {:?} operation", op)));
                }
            }
        }
    }
    warnings
}

/// v4.3 — Function Inlining: inline small functions (≤5 instructions)
pub fn optimize_inlining(func: &mut IRFunction, all_funcs: &[IRFunction]) -> usize {
    let mut inlined = 0;
    let mut i = 0;
    while i < func.body.len() {
        if let IRInstruction::Call { func: callee, args } = &func.body[i] {
            // Find the called function
            if let Some(target) = all_funcs.iter().find(|f| &f.name == callee) {
                // Only inline small functions (≤5 instructions, no recursion)
                if target.body.len() <= 5 && &target.name != &func.name {
                    // Replace call with inlined body
                    let mut inlined_body: Vec<IRInstruction> = Vec::new();
                    for instr in &target.body {
                        // Skip Return instructions, substitute args
                        if !matches!(instr, IRInstruction::Return | IRInstruction::ReturnVoid) {
                            inlined_body.push(instr.clone());
                        }
                    }
                    if !inlined_body.is_empty() {
                        func.body.splice(i..=i, inlined_body.clone());
                        inlined += 1;
                        i += inlined_body.len();
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    inlined
}

/// v4.3 — Loop Unrolling: detect small loops for unrolling hints
/// Note: Actual unrolling happens in codegen based on loop patterns
pub fn optimize_loop_unrolling(func: &mut IRFunction) -> usize {
    let mut unrolled = 0;
    // Count small loops (Label + BranchIfFalse patterns with small iteration counts)
    // This is a heuristic - actual unrolling is done in codegen
    let mut i = 0;
    while i < func.body.len() {
        if let IRInstruction::Label(label) = &func.body[i] {
            // Look for loop pattern: Label -> ... -> BranchIfFalse(same_label)
            if label.starts_with("loop_") || label.starts_with("for_") || label.starts_with("while_") {
                // Found a loop, mark as potential unroll candidate
                unrolled += 1;
            }
        }
        i += 1;
    }
    unrolled
}

/// v4.3 — Common Subexpression Elimination (CSE)
pub fn optimize_cse(func: &mut IRFunction) -> usize {
    use std::collections::HashMap;
    let mut eliminated = 0;
    let mut seen: HashMap<String, usize> = HashMap::new();
    
    for i in 0..func.body.len() {
        // Create a hash key for the instruction
        let key = match &func.body[i] {
            IRInstruction::BinOp { op, left, right } => {
                format!("{:?}:{:?}:{:?}", op, left, right)
            }
            _ => continue,
        };
        
        if let Some(&prev_idx) = seen.get(&key) {
            // Replace with reference to previous computation
            // For now, mark as Nop (will be eliminated by DCE)
            if prev_idx < i {
                func.body[i] = IRInstruction::Nop;
                eliminated += 1;
            }
        } else {
            seen.insert(key, i);
        }
    }
    eliminated
}

/// Run all optimization passes on a function
pub fn optimize_function(func: &mut IRFunction) -> (usize, usize) {
    let folded = optimize_constant_folding(func);
    let reduced = optimize_strength_reduction(func);
    let unrolled = optimize_loop_unrolling(func);
    let cse = optimize_cse(func);
    let eliminated = optimize_dead_code_elimination(func);
    (folded + reduced + unrolled + cse, eliminated)
}
