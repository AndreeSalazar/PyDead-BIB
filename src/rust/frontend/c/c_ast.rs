// C99 AST for ADead-BIB C Frontend
// Represents C programs before lowering to ADead-BIB IR

/// C type representation
#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Void,
    Char,
    Short,
    Int,
    Long,
    LongLong,
    Float,
    Double,
    Bool,
    Unsigned(Box<CType>),             // unsigned int, unsigned char, etc.
    Signed(Box<CType>),               // explicit signed
    Pointer(Box<CType>),              // T*
    Array(Box<CType>, Option<usize>), // T[N] or T[]
    Struct(String),                   // struct name
    Enum(String),                     // enum name
    Typedef(String),                  // typedef'd name
    Function {
        // function pointer type
        return_type: Box<CType>,
        params: Vec<CType>,
    },
    Const(Box<CType>),    // const T
    Volatile(Box<CType>), // volatile T
    Complex(Box<CType>),  // _Complex T (e.g., double _Complex)
}

/// C expression
#[derive(Debug, Clone)]
pub enum CExpr {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),
    Identifier(String),

    // Binary operations
    BinaryOp {
        op: CBinOp,
        left: Box<CExpr>,
        right: Box<CExpr>,
    },

    // Unary operations
    UnaryOp {
        op: CUnaryOp,
        expr: Box<CExpr>,
        prefix: bool, // true for prefix (++x), false for postfix (x++)
    },

    // Function call
    Call {
        func: Box<CExpr>,
        args: Vec<CExpr>,
    },

    // Array subscript: arr[idx]
    Index {
        array: Box<CExpr>,
        index: Box<CExpr>,
    },

    // Member access: obj.field
    Member {
        object: Box<CExpr>,
        field: String,
    },

    // Arrow access: ptr->field
    ArrowMember {
        pointer: Box<CExpr>,
        field: String,
    },

    // Cast: (int)x
    Cast {
        target_type: CType,
        expr: Box<CExpr>,
    },

    // Sizeof
    SizeofType(CType),
    SizeofExpr(Box<CExpr>),

    // Ternary: a ? b : c
    Ternary {
        condition: Box<CExpr>,
        then_expr: Box<CExpr>,
        else_expr: Box<CExpr>,
    },

    // Address-of: &x
    AddressOf(Box<CExpr>),

    // Dereference: *ptr
    Deref(Box<CExpr>),

    // Assignment: x = 5, x += 1, etc.
    Assign {
        op: CAssignOp,
        target: Box<CExpr>,
        value: Box<CExpr>,
    },

    // Comma expression: (a, b, c)
    Comma(Vec<CExpr>),

    // Initializer list: {5, 3, 8, ...}
    InitList(Vec<CExpr>),

    // NULL
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    LogAnd,
    LogOr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CUnaryOp {
    Neg,     // -x
    LogNot,  // !x
    BitNot,  // ~x
    PreInc,  // ++x
    PreDec,  // --x
    PostInc, // x++
    PostDec, // x--
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CAssignOp {
    Assign,    // =
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
    ModAssign, // %=
    AndAssign, // &=
    OrAssign,  // |=
    XorAssign, // ^=
    ShlAssign, // <<=
    ShrAssign, // >>=
}

/// C statement
#[derive(Debug, Clone)]
pub enum CStmt {
    // Expression statement: expr;
    Expr(CExpr),

    // Return statement
    Return(Option<CExpr>),

    // Variable declaration: int x = 5;
    VarDecl {
        type_spec: CType,
        declarators: Vec<CDeclarator>,
        is_static: bool,
    },

    // Compound statement (block): { ... }
    Block(Vec<CStmt>),

    // If/else
    If {
        condition: CExpr,
        then_body: Box<CStmt>,
        else_body: Option<Box<CStmt>>,
    },

    // While loop
    While {
        condition: CExpr,
        body: Box<CStmt>,
    },

    // Do-while loop
    DoWhile {
        body: Box<CStmt>,
        condition: CExpr,
    },

    // For loop
    For {
        init: Option<Box<CStmt>>, // can be VarDecl or Expr
        condition: Option<CExpr>,
        update: Option<CExpr>,
        body: Box<CStmt>,
    },

    // Switch
    Switch {
        expr: CExpr,
        cases: Vec<CSwitchCase>,
    },

    // Break
    Break,

    // Continue
    Continue,

    // Goto
    Goto(String),

    // Label: name:
    Label(String, Box<CStmt>),

    // Empty statement: ;
    Empty,

    // DEBUGINFO Line tracking
    LineMarker(usize),
}

/// Switch case
#[derive(Debug, Clone)]
pub struct CSwitchCase {
    pub value: Option<CExpr>, // None = default
    pub body: Vec<CStmt>,
}

/// Variable declarator (handles: int x = 5, *y, z[10])
#[derive(Debug, Clone)]
pub struct CDeclarator {
    pub name: String,
    pub derived_type: Option<CDerivedType>, // pointer/array modifications
    pub initializer: Option<CExpr>,
}

/// Type modifications on declarators
#[derive(Debug, Clone)]
pub enum CDerivedType {
    Pointer(Option<Box<CDerivedType>>),              // *
    Array(Option<usize>, Option<Box<CDerivedType>>), // [N]
}

/// Top-level C declarations
#[derive(Debug, Clone)]
pub enum CTopLevel {
    // Function definition
    FunctionDef {
        return_type: CType,
        name: String,
        params: Vec<CParam>,
        body: Vec<CStmt>,
    },

    // Function declaration (prototype)
    FunctionDecl {
        return_type: CType,
        name: String,
        params: Vec<CParam>,
    },

    // Global variable declaration
    GlobalVar {
        type_spec: CType,
        declarators: Vec<CDeclarator>,
    },

    // Struct definition
    StructDef {
        name: String,
        fields: Vec<CStructField>,
    },

    // Enum definition
    EnumDef {
        name: String,
        values: Vec<(String, Option<i64>)>,
    },

    // Typedef
    TypedefDecl {
        original: CType,
        new_name: String,
    },
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct CParam {
    pub param_type: CType,
    pub name: Option<String>, // can be unnamed in prototypes
}

/// Struct field
#[derive(Debug, Clone)]
pub struct CStructField {
    pub field_type: CType,
    pub name: String,
}

/// Complete C translation unit (a .c file)
#[derive(Debug, Clone)]
pub struct CTranslationUnit {
    pub declarations: Vec<CTopLevel>,
}

impl CTranslationUnit {
    pub fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }
}
