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

