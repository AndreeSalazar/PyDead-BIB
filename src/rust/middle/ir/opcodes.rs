use super::types::IRType;

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

    // ── Runtime 2.0 — IA Native Runtime ────────────────────────
    // Tensor lifecycle
    RtTensorCreate { rows: Box<IRInstruction>, cols: Box<IRInstruction> },  // tensor_create(rows,cols) → Tensor*
    RtTensorCreateFrom { rows: Box<IRInstruction>, cols: Box<IRInstruction>, data: String }, // tensor from .data label
    RtTensorFree,                                              // tensor_free(RAX=Tensor*)
    RtTensorZeros { rows: Box<IRInstruction>, cols: Box<IRInstruction> },
    RtTensorOnes { rows: Box<IRInstruction>, cols: Box<IRInstruction> },
    RtTensorPrint,                                             // tensor_print(RAX=Tensor*)

    // Tensor element-wise ops (result = new Tensor*)
    RtTensorAdd,        // tensor_add(RCX=a, RDX=b) → RAX=Tensor*
    RtTensorSub,        // tensor_sub
    RtTensorMul,        // tensor_mul
    RtTensorDiv,        // tensor_div
    RtTensorScale,      // tensor_scale(RCX=t, XMM1=scalar) → RAX=Tensor*
    RtTensorMatmul,     // tensor_matmul(RCX=a, RDX=b) → RAX=Tensor*
    RtTensorTranspose,  // tensor_transpose(RCX=t) → RAX=Tensor*

    // Tensor activations
    RtTensorRelu,       // tensor_relu(RCX=t) → RAX=Tensor*
    RtTensorSigmoid,    // tensor_sigmoid
    RtTensorTanh,       // tensor_tanh_act
    RtTensorSoftmax,    // tensor_softmax

    // Tensor reductions (result in RAX as f64 bits)
    RtTensorSum,        // tensor_sum(RCX=t) → XMM0=f64
    RtTensorMean,       // tensor_mean
    RtTensorMax,        // tensor_max
    RtTensorMin,        // tensor_min

    // Tensor accessors
    RtTensorGet2d { row: Box<IRInstruction>, col: Box<IRInstruction> }, // tensor_get_2d(t,row,col)→f64
    RtTensorSet2d { row: Box<IRInstruction>, col: Box<IRInstruction>, val: Box<IRInstruction> },

    // NN ops
    RtLinearCreate { in_f: Box<IRInstruction>, out_f: Box<IRInstruction> }, // linear_create(in,out)→Linear*
    RtLinearForward,    // linear_forward(RCX=Linear*, RDX=input) → RAX=Tensor*
    RtMlpCreate { layers: Vec<IRInstruction>, n_layers: Box<IRInstruction> },
    RtMlpForward,       // mlp_forward(RCX=MLP*, RDX=input) → RAX=Tensor*
    RtNNFillRandom,     // nn_fill_random(RCX=Tensor*, RDX=seed)
    RtCrossEntropyLoss, // nn_cross_entropy_loss(RCX=pred, RDX=target) → XMM0=f64

    // Autograd
    RtTapeCreate,       // tape_create() → RAX=Tape*
    RtTapeFree,         // tape_free(RCX=Tape*)
    RtTapeBackward,     // tape_backward(RCX=Tape*)
    RtTapeZeroGrad,     // tape_zero_grad(RCX=Tape*)
    RtAgMatmul,         // ag_matmul(RCX=Tape*, RDX=a, R8=b) → RAX=Tensor*
    RtAgAdd,            // ag_add(RCX=Tape*, RDX=a, R8=b) → RAX=Tensor*
    RtAgRelu,           // ag_relu(RCX=Tape*, RDX=input) → RAX=Tensor*
    RtAgSoftmaxCe,      // ag_softmax_ce(RCX=Tape*, RDX=logits, R8=labels, R9=batch) → RAX=Tensor*

    // Optimizers
    RtSgdCreate { lr: Box<IRInstruction>, momentum: Box<IRInstruction>, wd: Box<IRInstruction> },
    RtAdamCreate { lr: Box<IRInstruction>, beta1: Box<IRInstruction>, beta2: Box<IRInstruction> },
    RtOptimStep,        // optim_sgd_step or optim_adam_step(RCX=Optim*, RDX=params, R8=grads, R9=n)
    RtOptimZeroGrad,    // optim_zero_grad(RCX=grads, RDX=n)

    // Tensor I/O
    RtTensorSave,       // tensor_save(RCX=path, RDX=Tensor*) → RAX=0 ok
    RtTensorLoad,       // tensor_load(RCX=path) → RAX=Tensor*
    RtModelSave,        // tensor_save_multi(RCX=path, RDX=tensors[], R8=n)
    RtModelLoad,        // tensor_load_multi(RCX=path, RDX=tensors[], R8=n)

    // Memory / Arena
    RtArenaCreate { capacity: Box<IRInstruction> }, // arena_create(cap) → Arena*
    RtArenaFree,        // arena_free(RCX=Arena*)
    RtArenaReset,       // arena_reset(RCX=Arena*)
    RtArenaAllocTensor { rows: Box<IRInstruction>, cols: Box<IRInstruction> }, // arena_alloc_tensor

    // In-place ops (no new allocation)
    RtTensorAddInplace,  // tensor_add_inplace(RCX=dst, RDX=src)
    RtTensorSubInplace,  // tensor_sub_inplace
    RtTensorScaleInplace, // tensor_scale_inplace(RCX=t, XMM1=scalar)

    // Data loading
    RtLoadCsv,          // load_csv(RCX=path, RDX=&rows, R8=&cols) → RAX=float*

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

