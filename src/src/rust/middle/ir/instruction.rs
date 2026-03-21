// ============================================================
// ADead-BIB IR Instructions
// ============================================================
// SSA Instructions - Inspired by LLVM IR
// Each instruction produces at most one value
// ============================================================

use super::{Type, Value, ValueId};
use std::fmt;

/// Binary arithmetic/logical operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    SDiv, // Signed division
    UDiv, // Unsigned division
    SRem, // Signed remainder
    URem, // Unsigned remainder

    // Floating point
    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,

    // Bitwise
    And,
    Or,
    Xor,
    Shl,  // Shift left
    LShr, // Logical shift right
    AShr, // Arithmetic shift right
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "sub",
            BinaryOp::Mul => "mul",
            BinaryOp::SDiv => "sdiv",
            BinaryOp::UDiv => "udiv",
            BinaryOp::SRem => "srem",
            BinaryOp::URem => "urem",
            BinaryOp::FAdd => "fadd",
            BinaryOp::FSub => "fsub",
            BinaryOp::FMul => "fmul",
            BinaryOp::FDiv => "fdiv",
            BinaryOp::FRem => "frem",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
            BinaryOp::Xor => "xor",
            BinaryOp::Shl => "shl",
            BinaryOp::LShr => "lshr",
            BinaryOp::AShr => "ashr",
        };
        write!(f, "{}", name)
    }
}

/// Comparison predicates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    // Integer comparisons
    Eq,  // Equal
    Ne,  // Not equal
    Slt, // Signed less than
    Sle, // Signed less or equal
    Sgt, // Signed greater than
    Sge, // Signed greater or equal
    Ult, // Unsigned less than
    Ule, // Unsigned less or equal
    Ugt, // Unsigned greater than
    Uge, // Unsigned greater or equal

    // Floating point comparisons (ordered)
    FOeq, // Ordered equal
    FOne, // Ordered not equal
    FOlt, // Ordered less than
    FOle, // Ordered less or equal
    FOgt, // Ordered greater than
    FOge, // Ordered greater or equal

    // Floating point comparisons (unordered)
    FUeq, // Unordered equal
    FUne, // Unordered not equal
    FUlt, // Unordered less than
    FUle, // Unordered less or equal
    FUgt, // Unordered greater than
    FUge, // Unordered greater or equal
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            CompareOp::Eq => "eq",
            CompareOp::Ne => "ne",
            CompareOp::Slt => "slt",
            CompareOp::Sle => "sle",
            CompareOp::Sgt => "sgt",
            CompareOp::Sge => "sge",
            CompareOp::Ult => "ult",
            CompareOp::Ule => "ule",
            CompareOp::Ugt => "ugt",
            CompareOp::Uge => "uge",
            CompareOp::FOeq => "oeq",
            CompareOp::FOne => "one",
            CompareOp::FOlt => "olt",
            CompareOp::FOle => "ole",
            CompareOp::FOgt => "ogt",
            CompareOp::FOge => "oge",
            CompareOp::FUeq => "ueq",
            CompareOp::FUne => "une",
            CompareOp::FUlt => "ult",
            CompareOp::FUle => "ule",
            CompareOp::FUgt => "ugt",
            CompareOp::FUge => "uge",
        };
        write!(f, "{}", name)
    }
}

/// Cast operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastOp {
    Trunc,    // Truncate to smaller integer
    ZExt,     // Zero extend to larger integer
    SExt,     // Sign extend to larger integer
    FPTrunc,  // Truncate floating point
    FPExt,    // Extend floating point
    FPToUI,   // Float to unsigned int
    FPToSI,   // Float to signed int
    UIToFP,   // Unsigned int to float
    SIToFP,   // Signed int to float
    PtrToInt, // Pointer to integer
    IntToPtr, // Integer to pointer
    Bitcast,  // Reinterpret bits
}

impl fmt::Display for CastOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            CastOp::Trunc => "trunc",
            CastOp::ZExt => "zext",
            CastOp::SExt => "sext",
            CastOp::FPTrunc => "fptrunc",
            CastOp::FPExt => "fpext",
            CastOp::FPToUI => "fptoui",
            CastOp::FPToSI => "fptosi",
            CastOp::UIToFP => "uitofp",
            CastOp::SIToFP => "sitofp",
            CastOp::PtrToInt => "ptrtoint",
            CastOp::IntToPtr => "inttoptr",
            CastOp::Bitcast => "bitcast",
        };
        write!(f, "{}", name)
    }
}

/// Instruction opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    // Memory
    Alloca,
    Load,
    Store,
    GetElementPtr,

    // Arithmetic/Logic
    Binary(BinaryOp),

    // Comparison
    ICmp(CompareOp),
    FCmp(CompareOp),

    // Conversion
    Cast(CastOp),

    // Control flow (terminators)
    Ret,
    Br,
    CondBr,
    Switch,
    Unreachable,

    // Function call
    Call,

    // Phi node (SSA)
    Phi,

    // Select (ternary)
    Select,

    // Intrinsics
    Intrinsic,
}

/// IR Instruction
#[derive(Debug, Clone)]
pub struct Instruction {
    /// Unique ID of the result value (None for void instructions)
    pub result: Option<ValueId>,

    /// The operation
    pub opcode: Opcode,

    /// Result type
    pub ty: Type,

    /// Operands
    pub operands: Vec<Value>,

    /// For GEP: indices
    pub indices: Vec<Value>,

    /// For Phi: incoming blocks
    pub phi_blocks: Vec<u32>,

    /// For Call: function name
    pub call_target: Option<String>,

    /// For Intrinsic: intrinsic name
    pub intrinsic_name: Option<String>,

    /// Metadata (debug info, etc.)
    pub metadata: Vec<(String, String)>,
}

impl Instruction {
    // ============================================================
    // Constructors
    // ============================================================

    pub fn new(opcode: Opcode, ty: Type) -> Self {
        Instruction {
            result: None,
            opcode,
            ty,
            operands: Vec::new(),
            indices: Vec::new(),
            phi_blocks: Vec::new(),
            call_target: None,
            intrinsic_name: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_result(mut self, id: ValueId) -> Self {
        self.result = Some(id);
        self
    }

    pub fn with_operands(mut self, operands: Vec<Value>) -> Self {
        self.operands = operands;
        self
    }

    pub fn with_indices(mut self, indices: Vec<Value>) -> Self {
        self.indices = indices;
        self
    }

    // ============================================================
    // Memory instructions
    // ============================================================

    pub fn alloca(ty: Type, result: ValueId) -> Self {
        Instruction::new(Opcode::Alloca, Type::ptr(ty)).with_result(result)
    }

    pub fn load(ty: Type, ptr: Value, result: ValueId) -> Self {
        Instruction::new(Opcode::Load, ty)
            .with_result(result)
            .with_operands(vec![ptr])
    }

    pub fn store(value: Value, ptr: Value) -> Self {
        Instruction::new(Opcode::Store, Type::Void).with_operands(vec![value, ptr])
    }

    pub fn gep(base_ty: Type, ptr: Value, indices: Vec<Value>, result: ValueId) -> Self {
        let mut inst = Instruction::new(Opcode::GetElementPtr, Type::ptr(base_ty));
        inst.result = Some(result);
        inst.operands = vec![ptr];
        inst.indices = indices;
        inst
    }

    // ============================================================
    // Binary instructions
    // ============================================================

    pub fn binary(op: BinaryOp, ty: Type, lhs: Value, rhs: Value, result: ValueId) -> Self {
        Instruction::new(Opcode::Binary(op), ty)
            .with_result(result)
            .with_operands(vec![lhs, rhs])
    }

    pub fn add(ty: Type, lhs: Value, rhs: Value, result: ValueId) -> Self {
        Self::binary(BinaryOp::Add, ty, lhs, rhs, result)
    }

    pub fn sub(ty: Type, lhs: Value, rhs: Value, result: ValueId) -> Self {
        Self::binary(BinaryOp::Sub, ty, lhs, rhs, result)
    }

    pub fn mul(ty: Type, lhs: Value, rhs: Value, result: ValueId) -> Self {
        Self::binary(BinaryOp::Mul, ty, lhs, rhs, result)
    }

    pub fn sdiv(ty: Type, lhs: Value, rhs: Value, result: ValueId) -> Self {
        Self::binary(BinaryOp::SDiv, ty, lhs, rhs, result)
    }

    // ============================================================
    // Comparison instructions
    // ============================================================

    pub fn icmp(pred: CompareOp, lhs: Value, rhs: Value, result: ValueId) -> Self {
        Instruction::new(Opcode::ICmp(pred), Type::Bool)
            .with_result(result)
            .with_operands(vec![lhs, rhs])
    }

    pub fn fcmp(pred: CompareOp, lhs: Value, rhs: Value, result: ValueId) -> Self {
        Instruction::new(Opcode::FCmp(pred), Type::Bool)
            .with_result(result)
            .with_operands(vec![lhs, rhs])
    }

    // ============================================================
    // Cast instructions
    // ============================================================

    pub fn cast(op: CastOp, ty: Type, value: Value, result: ValueId) -> Self {
        Instruction::new(Opcode::Cast(op), ty)
            .with_result(result)
            .with_operands(vec![value])
    }

    // ============================================================
    // Control flow (terminators)
    // ============================================================

    pub fn ret(value: Option<Value>) -> Self {
        let mut inst = Instruction::new(Opcode::Ret, Type::Void);
        if let Some(v) = value {
            inst.operands.push(v);
        }
        inst
    }

    pub fn br(target: u32) -> Self {
        Instruction::new(Opcode::Br, Type::Void).with_operands(vec![Value::BasicBlock(target)])
    }

    pub fn cond_br(cond: Value, true_bb: u32, false_bb: u32) -> Self {
        Instruction::new(Opcode::CondBr, Type::Void).with_operands(vec![
            cond,
            Value::BasicBlock(true_bb),
            Value::BasicBlock(false_bb),
        ])
    }

    pub fn unreachable() -> Self {
        Instruction::new(Opcode::Unreachable, Type::Void)
    }

    // ============================================================
    // Call instruction
    // ============================================================

    pub fn call(ret_ty: Type, name: &str, args: Vec<Value>, result: Option<ValueId>) -> Self {
        let mut inst = Instruction::new(Opcode::Call, ret_ty);
        inst.result = result;
        inst.operands = args;
        inst.call_target = Some(name.to_string());
        inst
    }

    // ============================================================
    // Phi instruction
    // ============================================================

    pub fn phi(ty: Type, incoming: Vec<(Value, u32)>, result: ValueId) -> Self {
        let mut inst = Instruction::new(Opcode::Phi, ty);
        inst.result = Some(result);
        for (val, block) in incoming {
            inst.operands.push(val);
            inst.phi_blocks.push(block);
        }
        inst
    }

    // ============================================================
    // Select instruction
    // ============================================================

    pub fn select(
        ty: Type,
        cond: Value,
        true_val: Value,
        false_val: Value,
        result: ValueId,
    ) -> Self {
        Instruction::new(Opcode::Select, ty)
            .with_result(result)
            .with_operands(vec![cond, true_val, false_val])
    }

    // ============================================================
    // Queries
    // ============================================================

    pub fn is_terminator(&self) -> bool {
        matches!(
            self.opcode,
            Opcode::Ret | Opcode::Br | Opcode::CondBr | Opcode::Switch | Opcode::Unreachable
        )
    }

    pub fn has_side_effects(&self) -> bool {
        matches!(
            self.opcode,
            Opcode::Store
                | Opcode::Call
                | Opcode::Ret
                | Opcode::Br
                | Opcode::CondBr
                | Opcode::Switch
                | Opcode::Unreachable
        )
    }

    pub fn is_memory_op(&self) -> bool {
        matches!(
            self.opcode,
            Opcode::Alloca | Opcode::Load | Opcode::Store | Opcode::GetElementPtr
        )
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Result assignment
        if let Some(result) = &self.result {
            write!(f, "{} = ", result)?;
        }

        match &self.opcode {
            Opcode::Alloca => {
                write!(f, "alloca {}", self.ty.pointee().unwrap_or(&Type::Void))
            }
            Opcode::Load => {
                write!(f, "load {}, {}", self.ty, self.operands[0])
            }
            Opcode::Store => {
                write!(f, "store {}, {}", self.operands[0], self.operands[1])
            }
            Opcode::GetElementPtr => {
                write!(f, "getelementptr {}, {}", self.ty, self.operands[0])?;
                for idx in &self.indices {
                    write!(f, ", {}", idx)?;
                }
                Ok(())
            }
            Opcode::Binary(op) => {
                write!(
                    f,
                    "{} {} {}, {}",
                    op, self.ty, self.operands[0], self.operands[1]
                )
            }
            Opcode::ICmp(pred) => {
                write!(
                    f,
                    "icmp {} {}, {}",
                    pred, self.operands[0], self.operands[1]
                )
            }
            Opcode::FCmp(pred) => {
                write!(
                    f,
                    "fcmp {} {}, {}",
                    pred, self.operands[0], self.operands[1]
                )
            }
            Opcode::Cast(op) => {
                write!(f, "{} {} to {}", op, self.operands[0], self.ty)
            }
            Opcode::Ret => {
                if self.operands.is_empty() {
                    write!(f, "ret void")
                } else {
                    write!(f, "ret {}", self.operands[0])
                }
            }
            Opcode::Br => {
                write!(f, "br {}", self.operands[0])
            }
            Opcode::CondBr => {
                write!(
                    f,
                    "br {}, {}, {}",
                    self.operands[0], self.operands[1], self.operands[2]
                )
            }
            Opcode::Switch => {
                write!(f, "switch {} [", self.operands[0])?;
                for (i, op) in self.operands[1..].iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", op)?;
                }
                write!(f, "]")
            }
            Opcode::Unreachable => {
                write!(f, "unreachable")
            }
            Opcode::Call => {
                let name = self.call_target.as_deref().unwrap_or("unknown");
                write!(f, "call {} @{}(", self.ty, name)?;
                for (i, arg) in self.operands.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Opcode::Phi => {
                write!(f, "phi {} ", self.ty)?;
                for (i, (val, block)) in
                    self.operands.iter().zip(self.phi_blocks.iter()).enumerate()
                {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "[ {}, %bb{} ]", val, block)?;
                }
                Ok(())
            }
            Opcode::Select => {
                write!(
                    f,
                    "select {}, {}, {}",
                    self.operands[0], self.operands[1], self.operands[2]
                )
            }
            Opcode::Intrinsic => {
                let name = self.intrinsic_name.as_deref().unwrap_or("unknown");
                write!(f, "call {} @llvm.{}(", self.ty, name)?;
                for (i, arg) in self.operands.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Constant;
    use super::*;

    #[test]
    fn test_binary_instruction() {
        let lhs = Value::Constant(Constant::i32(5));
        let rhs = Value::Constant(Constant::i32(3));
        let inst = Instruction::add(Type::I32, lhs, rhs, ValueId(0));

        assert_eq!(inst.opcode, Opcode::Binary(BinaryOp::Add));
        assert_eq!(inst.ty, Type::I32);
        assert_eq!(inst.operands.len(), 2);
    }

    #[test]
    fn test_terminator_detection() {
        assert!(Instruction::ret(None).is_terminator());
        assert!(Instruction::br(0).is_terminator());
        assert!(!Instruction::alloca(Type::I32, ValueId(0)).is_terminator());
    }

    #[test]
    fn test_instruction_display() {
        let inst = Instruction::alloca(Type::I32, ValueId(0));
        assert!(format!("{}", inst).contains("alloca"));
    }
}
