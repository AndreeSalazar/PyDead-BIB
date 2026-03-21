// ============================================================
// ADead-BIB IR Values
// ============================================================
// SSA Values - Every instruction produces a unique value
// Inspired by LLVM Value system
// ============================================================

use super::Type;
use std::fmt;

/// Unique identifier for a value in the IR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub u32);

impl ValueId {
    pub fn new(id: u32) -> Self {
        ValueId(id)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

/// Constant values in the IR
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    // Integer constants
    Int { value: i64, ty: Type },

    // Floating point constants
    Float { value: f64, ty: Type },

    // Boolean constant
    Bool(bool),

    // Null pointer
    Null(Type),

    // Undefined value (for uninitialized variables)
    Undef(Type),

    // Zero initializer (for arrays/structs)
    ZeroInit(Type),

    // String constant (null-terminated)
    String(String),

    // Array constant
    Array { elements: Vec<Constant>, ty: Type },

    // Struct constant
    Struct { fields: Vec<Constant>, ty: Type },

    // Global reference
    GlobalRef { name: String, ty: Type },

    // Function reference
    FunctionRef { name: String, ty: Type },
}

impl Constant {
    // ============================================================
    // Constructors
    // ============================================================

    pub fn i8(value: i8) -> Self {
        Constant::Int {
            value: value as i64,
            ty: Type::I8,
        }
    }

    pub fn i16(value: i16) -> Self {
        Constant::Int {
            value: value as i64,
            ty: Type::I16,
        }
    }

    pub fn i32(value: i32) -> Self {
        Constant::Int {
            value: value as i64,
            ty: Type::I32,
        }
    }

    pub fn i64(value: i64) -> Self {
        Constant::Int {
            value,
            ty: Type::I64,
        }
    }

    pub fn f32(value: f32) -> Self {
        Constant::Float {
            value: value as f64,
            ty: Type::F32,
        }
    }

    pub fn f64(value: f64) -> Self {
        Constant::Float {
            value,
            ty: Type::F64,
        }
    }

    pub fn bool(value: bool) -> Self {
        Constant::Bool(value)
    }

    pub fn null(pointee: Type) -> Self {
        Constant::Null(Type::ptr(pointee))
    }

    pub fn undef(ty: Type) -> Self {
        Constant::Undef(ty)
    }

    pub fn zero(ty: Type) -> Self {
        Constant::ZeroInit(ty)
    }

    pub fn string(s: &str) -> Self {
        Constant::String(s.to_string())
    }

    pub fn global(name: &str, ty: Type) -> Self {
        Constant::GlobalRef {
            name: name.to_string(),
            ty,
        }
    }

    pub fn function(name: &str, ty: Type) -> Self {
        Constant::FunctionRef {
            name: name.to_string(),
            ty,
        }
    }

    // ============================================================
    // Type queries
    // ============================================================

    pub fn get_type(&self) -> Type {
        match self {
            Constant::Int { ty, .. } => ty.clone(),
            Constant::Float { ty, .. } => ty.clone(),
            Constant::Bool(_) => Type::Bool,
            Constant::Null(ty) => ty.clone(),
            Constant::Undef(ty) => ty.clone(),
            Constant::ZeroInit(ty) => ty.clone(),
            Constant::String(s) => Type::array(Type::I8, s.len() + 1),
            Constant::Array { ty, .. } => ty.clone(),
            Constant::Struct { ty, .. } => ty.clone(),
            Constant::GlobalRef { ty, .. } => Type::ptr(ty.clone()),
            Constant::FunctionRef { ty, .. } => Type::ptr(ty.clone()),
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Constant::Int { value, .. } => *value == 0,
            Constant::Float { value, .. } => *value == 0.0,
            Constant::Bool(b) => !*b,
            Constant::Null(_) => true,
            Constant::ZeroInit(_) => true,
            _ => false,
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            Constant::Int { value, .. } => *value == 1,
            Constant::Float { value, .. } => *value == 1.0,
            Constant::Bool(b) => *b,
            _ => false,
        }
    }

    pub fn is_all_ones(&self) -> bool {
        match self {
            Constant::Int { value, ty } => match ty {
                Type::I8 => *value as i8 == -1,
                Type::I16 => *value as i16 == -1,
                Type::I32 => *value as i32 == -1,
                Type::I64 => *value == -1,
                _ => false,
            },
            Constant::Bool(b) => *b,
            _ => false,
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Int { value, ty } => write!(f, "{} {}", ty, value),
            Constant::Float { value, ty } => write!(f, "{} {}", ty, value),
            Constant::Bool(b) => write!(f, "i1 {}", if *b { 1 } else { 0 }),
            Constant::Null(ty) => write!(f, "{} null", ty),
            Constant::Undef(ty) => write!(f, "{} undef", ty),
            Constant::ZeroInit(ty) => write!(f, "{} zeroinitializer", ty),
            Constant::String(s) => write!(f, "c\"{}\\00\"", s.escape_default()),
            Constant::Array { elements, ty } => {
                write!(f, "{} [", ty)?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Constant::Struct { fields, ty } => {
                write!(f, "{} {{ ", ty)?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", field)?;
                }
                write!(f, " }}")
            }
            Constant::GlobalRef { name, ty } => write!(f, "{}* @{}", ty, name),
            Constant::FunctionRef { name, ty } => write!(f, "{}* @{}", ty, name),
        }
    }
}

/// A value in the IR - either a computed value or a constant
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Result of an instruction
    Instruction(ValueId),

    /// A constant value
    Constant(Constant),

    /// Function argument
    Argument { index: usize, ty: Type },

    /// Basic block label
    BasicBlock(u32),
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Value::Instruction(_) => Type::Void, // Type resolved from instruction
            Value::Constant(c) => c.get_type(),
            Value::Argument { ty, .. } => ty.clone(),
            Value::BasicBlock(_) => Type::Label,
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Value::Constant(_))
    }

    pub fn as_constant(&self) -> Option<&Constant> {
        match self {
            Value::Constant(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_instruction(&self) -> Option<ValueId> {
        match self {
            Value::Instruction(id) => Some(*id),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Instruction(id) => write!(f, "{}", id),
            Value::Constant(c) => write!(f, "{}", c),
            Value::Argument { index, .. } => write!(f, "%arg{}", index),
            Value::BasicBlock(id) => write!(f, "label %bb{}", id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_types() {
        assert_eq!(Constant::i32(42).get_type(), Type::I32);
        assert_eq!(Constant::f64(3.14).get_type(), Type::F64);
        assert_eq!(Constant::bool(true).get_type(), Type::Bool);
    }

    #[test]
    fn test_constant_display() {
        assert_eq!(format!("{}", Constant::i32(42)), "i32 42");
        assert_eq!(format!("{}", Constant::bool(true)), "i1 1");
    }

    #[test]
    fn test_value_id_display() {
        assert_eq!(format!("{}", ValueId(0)), "%0");
        assert_eq!(format!("{}", ValueId(42)), "%42");
    }
}
