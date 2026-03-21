/// Unified Type System for ADead-BIB
/// C-style sized types that flow from parser to codegen
///
/// Philosophy: Types determine machine code size.
/// `char x = 65` → `mov al, 65` (2 bytes) not `mov rax, 65` (10 bytes)

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Signed integers with explicit size (C-style)
    I8,  // char / int8_t       (1 byte)
    I16, // short / int16_t     (2 bytes)
    I32, // int / int32_t       (4 bytes)
    I64, // long / int64_t      (8 bytes)

    // Unsigned integers
    U8,  // unsigned char       (1 byte)
    U16, // unsigned short      (2 bytes)
    U32, // unsigned int        (4 bytes)
    U64, // unsigned long       (8 bytes)

    // Floating point
    F32, // float               (4 bytes)
    F64, // double              (8 bytes)

    // Other primitives
    Bool, // bool                (1 byte)
    Void, // void                (0 bytes)
    Str,  // string (pointer)    (8 bytes)

    // Composite types
    Pointer(Box<Type>),              // T*
    Reference(Box<Type>),            // T&
    Array(Box<Type>, Option<usize>), // T[N] or T[]
    Struct(String),                  // struct Name
    Class(String),                   // class Name
    Function(Vec<Type>, Box<Type>),  // fn(args) -> ret

    // SIMD
    Vec4,  // 4×f32 (128-bit SSE)
    Vec8,  // 8×f32 (256-bit AVX)
    Vec16, // 16×f32 (512-bit AVX-512)

    // Named / user-defined
    Named(String),

    // Inference
    Auto, // compiler deduces type

    // Unknown (for incomplete inference)
    Unknown,
}

/// Register size classification for codegen
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegSize {
    Byte,  // 8-bit: AL, BL, CL, DL
    Word,  // 16-bit: AX, BX, CX, DX
    DWord, // 32-bit: EAX, EBX, ECX, EDX
    QWord, // 64-bit: RAX, RBX, RCX, RDX
}

impl Type {
    /// Size in bytes — ESSENTIAL for correct codegen
    pub fn size_bytes(&self) -> usize {
        match self {
            Type::I8 | Type::U8 | Type::Bool => 1,
            Type::I16 | Type::U16 => 2,
            Type::I32 | Type::U32 | Type::F32 => 4,
            Type::I64 | Type::U64 | Type::F64 | Type::Pointer(_) | Type::Reference(_) => 8,
            Type::Str => 8,
            Type::Vec4 => 16,
            Type::Vec8 => 32,
            Type::Vec16 => 64,
            Type::Void => 0,
            Type::Array(t, Some(n)) => t.size_bytes() * n,
            Type::Array(_, None) => 8,
            Type::Struct(_) | Type::Class(_) => 8,
            Type::Function(_, _) => 8,
            Type::Named(_) => 8,
            Type::Auto | Type::Unknown => 8,
        }
    }

    /// What register size to use for this type
    pub fn reg_size(&self) -> RegSize {
        match self.size_bytes() {
            1 => RegSize::Byte,
            2 => RegSize::Word,
            4 => RegSize::DWord,
            _ => RegSize::QWord,
        }
    }

    /// Is this a SIMD type?
    pub fn is_simd(&self) -> bool {
        matches!(self, Type::Vec4 | Type::Vec8 | Type::Vec16)
    }

    /// Is this a pointer type?
    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer(_))
    }

    /// Is this a numeric type?
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::F32
                | Type::F64
        )
    }

    /// Is this a signed integer?
    pub fn is_signed(&self) -> bool {
        matches!(self, Type::I8 | Type::I16 | Type::I32 | Type::I64)
    }

    /// Is this an unsigned integer?
    pub fn is_unsigned(&self) -> bool {
        matches!(self, Type::U8 | Type::U16 | Type::U32 | Type::U64)
    }

    /// Is this a float type?
    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64)
    }

    /// Map C type name to ADead-BIB Type
    pub fn from_c_name(name: &str) -> Self {
        match name {
            "char" => Type::I8,
            "short" => Type::I16,
            "int" => Type::I32,
            "long" => Type::I64,
            "float" => Type::F32,
            "double" => Type::F64,
            "void" => Type::Void,
            "bool" => Type::Bool,
            "string" | "str" => Type::Str,
            "auto" => Type::Auto,
            "i8" | "int8" => Type::I8,
            "i16" | "int16" => Type::I16,
            "i32" | "int32" => Type::I32,
            "i64" | "int64" => Type::I64,
            "u8" | "uint8" => Type::U8,
            "u16" | "uint16" => Type::U16,
            "u32" | "uint32" => Type::U32,
            "u64" | "uint64" => Type::U64,
            "f32" => Type::F32,
            "f64" => Type::F64,
            other => Type::Named(other.to_string()),
        }
    }

    /// Create a pointer to this type
    pub fn pointer_to(self) -> Self {
        Type::Pointer(Box::new(self))
    }

    /// Create a reference to this type
    pub fn reference_to(self) -> Self {
        Type::Reference(Box::new(self))
    }

    /// Create an array of this type
    pub fn array_of(self, size: Option<usize>) -> Self {
        Type::Array(Box::new(self), size)
    }

    /// Dereference a pointer type, returning the pointed-to type
    pub fn deref_type(&self) -> Option<&Type> {
        match self {
            Type::Pointer(inner) => Some(inner),
            Type::Reference(inner) => Some(inner),
            _ => None,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::Void => write!(f, "void"),
            Type::Str => write!(f, "string"),
            Type::Pointer(t) => write!(f, "{}*", t),
            Type::Reference(t) => write!(f, "{}&", t),
            Type::Array(t, Some(n)) => write!(f, "{}[{}]", t, n),
            Type::Array(t, None) => write!(f, "{}[]", t),
            Type::Struct(n) => write!(f, "struct {}", n),
            Type::Class(n) => write!(f, "class {}", n),
            Type::Function(args, ret) => {
                write!(f, "fn(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Vec4 => write!(f, "vec4"),
            Type::Vec8 => write!(f, "vec8"),
            Type::Vec16 => write!(f, "vec16"),
            Type::Named(n) => write!(f, "{}", n),
            Type::Auto => write!(f, "auto"),
            Type::Unknown => write!(f, "unknown"),
        }
    }
}
