// ============================================================
// ADead-BIB C++ Frontend — Abstract Syntax Tree
// ============================================================
// C++11/14/17/20 AST nodes
// Classes, templates, namespaces, lambdas, modern C++
//
// Sin GCC. Sin LLVM. Sin Clang. Solo ADead-BIB. 💀🦈
// ============================================================

// ========== Types ==========

#[derive(Debug, Clone, PartialEq)]
pub enum CppType {
    // Primitives
    Void,
    Bool,
    Char,
    WChar,
    Char8, // C++20
    Char16,
    Char32,
    Short,
    Int,
    Long,
    LongLong,
    Float,
    Double,
    LongDouble,
    Auto,                   // C++11 type inference
    Decltype(Box<CppExpr>), // C++11 decltype(expr)

    // Qualifiers
    Unsigned(Box<CppType>),
    Signed(Box<CppType>),
    Const(Box<CppType>),
    Volatile(Box<CppType>),
    Mutable(Box<CppType>),
    Constexpr(Box<CppType>), // C++11

    // Compound
    Pointer(Box<CppType>),
    Reference(Box<CppType>), // T&
    RValueRef(Box<CppType>), // T&& (C++11 move semantics)
    Array(Box<CppType>, Option<usize>),
    Function {
        return_type: Box<CppType>,
        params: Vec<CppType>,
    },

    // User-defined
    Named(String), // MyClass, std::string
    Struct(String),
    Class(String),
    Enum(String),
    Union(String),
    Typedef(String),

    // Templates
    TemplateType {
        name: String,
        args: Vec<CppType>, // vector<int>, map<string, int>
    },

    // Smart pointers (recognized by name, lowered in IR)
    UniquePtr(Box<CppType>),
    SharedPtr(Box<CppType>),
    WeakPtr(Box<CppType>),

    // STL containers (recognized)
    StdString,
    StdStringView,
    StdVector(Box<CppType>),
    StdArray(Box<CppType>, usize),
    StdMap(Box<CppType>, Box<CppType>),
    StdUnorderedMap(Box<CppType>, Box<CppType>),
    StdSet(Box<CppType>),
    StdUnorderedSet(Box<CppType>),
    StdList(Box<CppType>),
    StdForwardList(Box<CppType>),
    StdDeque(Box<CppType>),
    StdStack(Box<CppType>),
    StdQueue(Box<CppType>),
    StdPriorityQueue(Box<CppType>),
    StdOptional(Box<CppType>),
    StdVariant(Vec<CppType>),
    StdTuple(Vec<CppType>),
    StdSpan(Box<CppType>),
    StdInitializerList(Box<CppType>),
    StdAny,

    // Concurrency types
    StdThread,
    StdMutex,
    StdAtomic(Box<CppType>),
    StdFuture(Box<CppType>),
    StdPromise(Box<CppType>),

    // Other STL types
    StdRegex,
    StdFilesystemPath,

    // Special
    Nullptr, // std::nullptr_t
    SizeT,
}

// ========== Expressions ==========

#[derive(Debug, Clone, PartialEq)]
pub enum CppExpr {
    // Literals
    IntLiteral(i64),
    UIntLiteral(u64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),
    BoolLiteral(bool),
    NullptrLiteral,

    // Identifiers
    Identifier(String),
    ScopedIdentifier {
        scope: Vec<String>, // std::cout → ["std"]
        name: String,       // "cout"
    },
    This,

    // Binary operations
    BinaryOp {
        op: CppBinOp,
        left: Box<CppExpr>,
        right: Box<CppExpr>,
    },

    // Unary operations
    UnaryOp {
        op: CppUnaryOp,
        expr: Box<CppExpr>,
        is_prefix: bool,
    },

    // Assignment
    Assign {
        target: Box<CppExpr>,
        value: Box<CppExpr>,
    },
    CompoundAssign {
        op: CppBinOp,
        target: Box<CppExpr>,
        value: Box<CppExpr>,
    },

    // Function call
    Call {
        callee: Box<CppExpr>,
        args: Vec<CppExpr>,
    },

    // Member access
    MemberAccess {
        object: Box<CppExpr>,
        member: String,
    },
    ArrowAccess {
        pointer: Box<CppExpr>,
        member: String,
    },

    // Array/index
    Index {
        object: Box<CppExpr>,
        index: Box<CppExpr>,
    },

    // Pointer operations
    Deref(Box<CppExpr>),
    AddressOf(Box<CppExpr>),

    // Type operations
    Cast {
        cast_type: CppCastKind,
        target_type: CppType,
        expr: Box<CppExpr>,
    },
    SizeOf(CppSizeOfArg),
    TypeId(Box<CppExpr>),

    // Ternary
    Ternary {
        condition: Box<CppExpr>,
        then_expr: Box<CppExpr>,
        else_expr: Box<CppExpr>,
    },

    // C++11 Lambda
    Lambda {
        captures: Vec<CppCapture>,
        params: Vec<CppParam>,
        return_type: Option<CppType>,
        body: Vec<CppStmt>,
    },

    // C++11 Initializer list
    InitList(Vec<CppExpr>),

    // new / delete
    New {
        type_name: CppType,
        args: Vec<CppExpr>,
        is_array: bool,
        array_size: Option<Box<CppExpr>>,
    },
    Delete {
        expr: Box<CppExpr>,
        is_array: bool,
    },

    // C++11 range-for helper
    RangeExpr {
        start: Box<CppExpr>,
        end: Box<CppExpr>,
    },

    // Structured binding reference (C++17)
    StructuredBinding(Vec<String>),

    // Fold expression (C++17): (args op ...)
    FoldExpr {
        op: CppBinOp,
        pack: Box<CppExpr>,
        init: Option<Box<CppExpr>>,  // for binary fold: (init op ... op pack)
        is_right: bool,              // right fold vs left fold
    },

    // Pack expansion: expr...
    PackExpansion(Box<CppExpr>),

    // Co_await / co_yield (C++20)
    CoAwait(Box<CppExpr>),
    CoYield(Box<CppExpr>),

    // Throw
    Throw(Option<Box<CppExpr>>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CppBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Spaceship, // <=> (C++20)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CppUnaryOp {
    Neg,     // -x
    Not,     // !x
    BitNot,  // ~x
    PreInc,  // ++x
    PreDec,  // --x
    PostInc, // x++
    PostDec, // x--
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CppCastKind {
    CStyle,          // (int)x
    StaticCast,      // static_cast<int>(x)
    DynamicCast,     // dynamic_cast<Base*>(x)
    ConstCast,       // const_cast<int*>(x)
    ReinterpretCast, // reinterpret_cast<void*>(x)
}

#[derive(Debug, Clone, PartialEq)]
pub enum CppSizeOfArg {
    Type(CppType),
    Expr(Box<CppExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CppCapture {
    ByValue(String), // x
    ByRef(String),   // &x
    ThisByValue,     // *this (C++17)
    ThisByRef,       // this
    DefaultByValue,  // =
    DefaultByRef,    // &
}

// ========== Statements ==========

#[derive(Debug, Clone, PartialEq)]
pub enum CppStmt {
    // Sequence point / Line tracker
    LineMarker(usize),

    // Expression statement
    Expr(CppExpr),

    // Variable declaration
    VarDecl {
        type_spec: CppType,
        declarators: Vec<CppDeclarator>,
    },

    // Block
    Block(Vec<CppStmt>),

    // Control flow
    Return(Option<CppExpr>),
    If {
        init: Option<Box<CppStmt>>, // C++17 if with init
        condition: CppExpr,
        then_body: Box<CppStmt>,
        else_body: Option<Box<CppStmt>>,
        is_constexpr: bool,         // C++17 if constexpr
    },
    While {
        condition: CppExpr,
        body: Box<CppStmt>,
    },
    DoWhile {
        body: Box<CppStmt>,
        condition: CppExpr,
    },
    For {
        init: Option<Box<CppStmt>>,
        condition: Option<CppExpr>,
        increment: Option<CppExpr>,
        body: Box<CppStmt>,
    },
    RangeFor {
        type_spec: CppType,
        name: String,
        iterable: CppExpr,
        body: Box<CppStmt>,
    },
    Switch {
        expr: CppExpr,
        cases: Vec<CppSwitchCase>,
        default: Option<Vec<CppStmt>>,
    },
    Break,
    Continue,
    Goto(String),
    Label(String, Box<CppStmt>),
    Empty,

    // Exception handling
    Try {
        body: Vec<CppStmt>,
        catches: Vec<CppCatch>,
    },
    Throw(Option<CppExpr>),

    // C++20 coroutine
    CoReturn(Option<CppExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CppDeclarator {
    pub name: String,
    pub derived_type: Vec<CppDerivedType>,
    pub initializer: Option<CppExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CppDerivedType {
    Pointer,
    Reference,
    RValueRef,
    Array(Option<usize>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CppSwitchCase {
    pub value: CppExpr,
    pub body: Vec<CppStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CppCatch {
    pub param_type: Option<CppType>,
    pub param_name: Option<String>,
    pub body: Vec<CppStmt>,
}

// ========== Parameters ==========

#[derive(Debug, Clone, PartialEq)]
pub struct CppParam {
    pub param_type: CppType,
    pub name: Option<String>,
    pub default_value: Option<CppExpr>,
    pub is_variadic: bool,
}

// ========== Top-Level Declarations ==========

#[derive(Debug, Clone, PartialEq)]
pub enum CppTopLevel {
    // Functions
    FunctionDef {
        return_type: CppType,
        name: String,
        template_params: Vec<CppTemplateParam>,
        params: Vec<CppParam>,
        qualifiers: CppFuncQualifiers,
        body: Vec<CppStmt>,
    },
    FunctionDecl {
        return_type: CppType,
        name: String,
        template_params: Vec<CppTemplateParam>,
        params: Vec<CppParam>,
        qualifiers: CppFuncQualifiers,
    },

    // Classes / Structs
    ClassDef {
        name: String,
        template_params: Vec<CppTemplateParam>,
        bases: Vec<CppBaseClass>,
        members: Vec<CppClassMember>,
        is_struct: bool,
    },

    // Enums
    EnumDef {
        name: String,
        is_class: bool, // enum class (C++11)
        underlying_type: Option<CppType>,
        values: Vec<(String, Option<CppExpr>)>,
    },

    // Namespace
    Namespace {
        name: String,
        declarations: Vec<CppTopLevel>,
    },

    // Using declarations
    UsingDecl {
        name: String,
        target: String, // using cout = std::cout;
    },
    UsingNamespace(String), // using namespace std;

    // Typedef / Type alias
    TypeAlias {
        new_name: String,
        original: CppType,
        template_params: Vec<CppTemplateParam>,
    },

    // Global variable
    GlobalVar {
        type_spec: CppType,
        declarators: Vec<CppDeclarator>,
    },

    // Template explicit instantiation
    TemplateInstantiation {
        type_name: CppType,
    },

    // Template full specialization: template<> class Foo<int> { ... }
    TemplateSpecialization {
        name: String,
        specialized_args: Vec<CppType>,        // <int>, <T*>, etc.
        template_params: Vec<CppTemplateParam>, // empty for full, non-empty for partial
        members: Vec<CppClassMember>,
        is_struct: bool,
    },

    // Template function specialization: template<> int max<int>(int a, int b) { ... }
    TemplateFuncSpecialization {
        name: String,
        specialized_args: Vec<CppType>,
        template_params: Vec<CppTemplateParam>,
        return_type: CppType,
        params: Vec<CppParam>,
        body: Vec<CppStmt>,
    },

    // Static assert (C++11)
    StaticAssert {
        condition: CppExpr,
        message: Option<String>,
    },

    // Extern "C"
    ExternC {
        declarations: Vec<CppTopLevel>,
    },
}

// ========== Class Members ==========

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CppAccess {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CppClassMember {
    Field {
        access: CppAccess,
        type_spec: CppType,
        name: String,
        default_value: Option<CppExpr>,
        is_static: bool,
    },
    Method {
        access: CppAccess,
        return_type: CppType,
        name: String,
        template_params: Vec<CppTemplateParam>,
        params: Vec<CppParam>,
        qualifiers: CppFuncQualifiers,
        body: Option<Vec<CppStmt>>, // None = declaration only
    },
    Constructor {
        access: CppAccess,
        params: Vec<CppParam>,
        initializer_list: Vec<(String, CppExpr)>,
        body: Option<Vec<CppStmt>>,
        is_explicit: bool,
    },
    Destructor {
        access: CppAccess,
        is_virtual: bool,
        body: Option<Vec<CppStmt>>,
    },
    NestedClass(Box<CppTopLevel>),
    NestedEnum(Box<CppTopLevel>),
    UsingDecl(String),
    FriendDecl(String),
    AccessSpec(CppAccess),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CppFuncQualifiers {
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_final: bool,
    pub is_static: bool,
    pub is_const: bool,
    pub is_noexcept: bool,
    pub is_constexpr: bool,
    pub is_inline: bool,
    pub is_pure_virtual: bool, // = 0
    pub is_default: bool,      // = default
    pub is_delete: bool,       // = delete
}

impl Default for CppFuncQualifiers {
    fn default() -> Self {
        Self {
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_static: false,
            is_const: false,
            is_noexcept: false,
            is_constexpr: false,
            is_inline: false,
            is_pure_virtual: false,
            is_default: false,
            is_delete: false,
        }
    }
}

// ========== Templates ==========

#[derive(Debug, Clone, PartialEq)]
pub enum CppTemplateParam {
    TypeParam {
        name: String,
        default_type: Option<CppType>,
    },
    NonTypeParam {
        param_type: CppType,
        name: String,
        default_value: Option<CppExpr>,
    },
    TemplateTemplateParam {
        name: String,
    },
    VariadicType {
        name: String, // typename... Args
    },
}

// ========== Inheritance ==========

#[derive(Debug, Clone, PartialEq)]
pub struct CppBaseClass {
    pub access: CppAccess,
    pub name: String,
    pub is_virtual: bool,
    pub template_args: Vec<CppType>,
}

// ========== Struct Field (for C-style structs) ==========

#[derive(Debug, Clone, PartialEq)]
pub struct CppStructField {
    pub field_type: CppType,
    pub name: String,
}

// ========== Translation Unit ==========

#[derive(Debug, Clone)]
pub struct CppTranslationUnit {
    pub declarations: Vec<CppTopLevel>,
}

impl CppTranslationUnit {
    pub fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }
}
