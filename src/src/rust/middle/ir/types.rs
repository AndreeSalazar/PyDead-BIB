// ============================================================
// ADead-BIB IR Type System
// ============================================================
// Inspired by LLVM Type System
// ============================================================

use std::fmt;

/// IR Type - Represents all types in the intermediate representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // Primitive types
    Void,
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,

    // Pointer type
    Ptr(Box<Type>),

    // Array type: [N x T]
    Array {
        element: Box<Type>,
        size: usize,
    },

    // Vector type (SIMD): <N x T>
    Vector {
        element: Box<Type>,
        size: usize,
    },

    // Struct type: { T1, T2, ... }
    Struct {
        name: Option<String>,
        fields: Vec<Type>,
        packed: bool,
    },

    // Function type: T (T1, T2, ...)
    Function {
        return_type: Box<Type>,
        params: Vec<Type>,
        variadic: bool,
    },

    // Label type (for basic blocks)
    Label,

    // Metadata type
    Metadata,
}

impl Type {
    // ============================================================
    // Constructors
    // ============================================================

    pub fn void() -> Self {
        Type::Void
    }
    pub fn bool() -> Self {
        Type::Bool
    }
    pub fn i8() -> Self {
        Type::I8
    }
    pub fn i16() -> Self {
        Type::I16
    }
    pub fn i32() -> Self {
        Type::I32
    }
    pub fn i64() -> Self {
        Type::I64
    }
    pub fn i128() -> Self {
        Type::I128
    }
    pub fn f32() -> Self {
        Type::F32
    }
    pub fn f64() -> Self {
        Type::F64
    }

    pub fn ptr(pointee: Type) -> Self {
        Type::Ptr(Box::new(pointee))
    }

    pub fn array(element: Type, size: usize) -> Self {
        Type::Array {
            element: Box::new(element),
            size,
        }
    }

    pub fn vector(element: Type, size: usize) -> Self {
        Type::Vector {
            element: Box::new(element),
            size,
        }
    }

    pub fn structure(fields: Vec<Type>) -> Self {
        Type::Struct {
            name: None,
            fields,
            packed: false,
        }
    }

    pub fn named_struct(name: &str, fields: Vec<Type>) -> Self {
        Type::Struct {
            name: Some(name.to_string()),
            fields,
            packed: false,
        }
    }

    pub fn packed_struct(fields: Vec<Type>) -> Self {
        Type::Struct {
            name: None,
            fields,
            packed: true,
        }
    }

    pub fn function(return_type: Type, params: Vec<Type>, variadic: bool) -> Self {
        Type::Function {
            return_type: Box::new(return_type),
            params,
            variadic,
        }
    }

    // ============================================================
    // Type queries
    // ============================================================

    /// Returns true if this is a void type
    pub fn is_void(&self) -> bool {
        matches!(self, Type::Void)
    }

    /// Returns true if this is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::Bool | Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128
        )
    }

    /// Returns true if this is a floating point type
    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64)
    }

    /// Returns true if this is a pointer type
    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Ptr(_))
    }

    /// Returns true if this is an array type
    pub fn is_array(&self) -> bool {
        matches!(self, Type::Array { .. })
    }

    /// Returns true if this is a vector type
    pub fn is_vector(&self) -> bool {
        matches!(self, Type::Vector { .. })
    }

    /// Returns true if this is a struct type
    pub fn is_struct(&self) -> bool {
        matches!(self, Type::Struct { .. })
    }

    /// Returns true if this is a function type
    pub fn is_function(&self) -> bool {
        matches!(self, Type::Function { .. })
    }

    /// Returns true if this type can be used as a first-class value
    pub fn is_first_class(&self) -> bool {
        !matches!(
            self,
            Type::Void | Type::Function { .. } | Type::Label | Type::Metadata
        )
    }

    /// Returns true if this type can be used in aggregate types
    pub fn is_valid_element(&self) -> bool {
        !matches!(self, Type::Void | Type::Label | Type::Metadata)
    }

    // ============================================================
    // Size and alignment (for x86-64)
    // ============================================================

    /// Returns the size in bits
    pub fn bit_size(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Bool => 1,
            Type::I8 => 8,
            Type::I16 => 16,
            Type::I32 => 32,
            Type::I64 => 64,
            Type::I128 => 128,
            Type::F32 => 32,
            Type::F64 => 64,
            Type::Ptr(_) => 64, // x86-64
            Type::Array { element, size } => element.bit_size() * size,
            Type::Vector { element, size } => element.bit_size() * size,
            Type::Struct { fields, packed, .. } => {
                if *packed {
                    fields.iter().map(|f| f.bit_size()).sum()
                } else {
                    // With alignment padding
                    let mut offset = 0;
                    for field in fields {
                        let align = field.alignment();
                        offset = (offset + align - 1) / align * align;
                        offset += field.byte_size();
                    }
                    offset * 8
                }
            }
            Type::Function { .. } => 0,
            Type::Label => 0,
            Type::Metadata => 0,
        }
    }

    /// Returns the size in bytes
    pub fn byte_size(&self) -> usize {
        (self.bit_size() + 7) / 8
    }

    /// Returns the alignment in bytes
    pub fn alignment(&self) -> usize {
        match self {
            Type::Void => 1,
            Type::Bool => 1,
            Type::I8 => 1,
            Type::I16 => 2,
            Type::I32 => 4,
            Type::I64 => 8,
            Type::I128 => 16,
            Type::F32 => 4,
            Type::F64 => 8,
            Type::Ptr(_) => 8,
            Type::Array { element, .. } => element.alignment(),
            Type::Vector { element, size } => {
                let elem_size = element.byte_size();
                (elem_size * size).min(32) // Max 32-byte alignment for AVX
            }
            Type::Struct { fields, packed, .. } => {
                if *packed {
                    1
                } else {
                    fields.iter().map(|f| f.alignment()).max().unwrap_or(1)
                }
            }
            Type::Function { .. } => 1,
            Type::Label => 1,
            Type::Metadata => 1,
        }
    }

    // ============================================================
    // Type access
    // ============================================================

    /// Get the pointee type for pointer types
    pub fn pointee(&self) -> Option<&Type> {
        match self {
            Type::Ptr(t) => Some(t),
            _ => None,
        }
    }

    /// Get the element type for array/vector types
    pub fn element_type(&self) -> Option<&Type> {
        match self {
            Type::Array { element, .. } => Some(element),
            Type::Vector { element, .. } => Some(element),
            _ => None,
        }
    }

    /// Get the number of elements for array/vector types
    pub fn num_elements(&self) -> Option<usize> {
        match self {
            Type::Array { size, .. } => Some(*size),
            Type::Vector { size, .. } => Some(*size),
            _ => None,
        }
    }

    /// Get struct fields
    pub fn struct_fields(&self) -> Option<&[Type]> {
        match self {
            Type::Struct { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Get function return type
    pub fn return_type(&self) -> Option<&Type> {
        match self {
            Type::Function { return_type, .. } => Some(return_type),
            _ => None,
        }
    }

    /// Get function parameter types
    pub fn param_types(&self) -> Option<&[Type]> {
        match self {
            Type::Function { params, .. } => Some(params),
            _ => None,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => write!(f, "void"),
            Type::Bool => write!(f, "i1"),
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::I128 => write!(f, "i128"),
            Type::F32 => write!(f, "float"),
            Type::F64 => write!(f, "double"),
            Type::Ptr(t) => write!(f, "{}*", t),
            Type::Array { element, size } => write!(f, "[{} x {}]", size, element),
            Type::Vector { element, size } => write!(f, "<{} x {}>", size, element),
            Type::Struct { name: Some(n), .. } => write!(f, "%{}", n),
            Type::Struct {
                name: None,
                fields,
                packed,
            } => {
                if *packed {
                    write!(f, "<")?
                }
                write!(f, "{{ ")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", field)?;
                }
                write!(f, " }}")?;
                if *packed {
                    write!(f, ">")?
                }
                Ok(())
            }
            Type::Function {
                return_type,
                params,
                variadic,
            } => {
                write!(f, "{} (", return_type)?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", param)?;
                }
                if *variadic {
                    if !params.is_empty() {
                        write!(f, ", ")?
                    }
                    write!(f, "...")?;
                }
                write!(f, ")")
            }
            Type::Label => write!(f, "label"),
            Type::Metadata => write!(f, "metadata"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_sizes() {
        assert_eq!(Type::i8().byte_size(), 1);
        assert_eq!(Type::i16().byte_size(), 2);
        assert_eq!(Type::i32().byte_size(), 4);
        assert_eq!(Type::i64().byte_size(), 8);
        assert_eq!(Type::f32().byte_size(), 4);
        assert_eq!(Type::f64().byte_size(), 8);
    }

    #[test]
    fn test_pointer_size() {
        assert_eq!(Type::ptr(Type::i32()).byte_size(), 8);
        assert_eq!(Type::ptr(Type::i8()).byte_size(), 8);
    }

    #[test]
    fn test_array_size() {
        assert_eq!(Type::array(Type::i32(), 10).byte_size(), 40);
        assert_eq!(Type::array(Type::i64(), 5).byte_size(), 40);
    }

    #[test]
    fn test_type_display() {
        assert_eq!(format!("{}", Type::i32()), "i32");
        assert_eq!(format!("{}", Type::ptr(Type::i8())), "i8*");
        assert_eq!(format!("{}", Type::array(Type::i32(), 10)), "[10 x i32]");
    }
}
