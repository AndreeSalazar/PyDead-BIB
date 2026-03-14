// ============================================================
// Python AST for PyDead-BIB
// ============================================================
// Represents Python programs before lowering to ADeadOp IR
// Supports Python 2.7 → 3.13 syntax
// ============================================================

/// Python type annotation
#[derive(Debug, Clone, PartialEq)]
pub enum PyType {
    Int,
    Float,
    Str,
    Bool,
    None,
    Bytes,
    List(Box<PyType>),
    Dict(Box<PyType>, Box<PyType>),
    Set(Box<PyType>),
    Tuple(Vec<PyType>),
    Optional(Box<PyType>),
    Union(Vec<PyType>),
    Callable(Vec<PyType>, Box<PyType>),
    Any,
    Custom(String),
    Inferred,
}

/// Python module (top-level)
#[derive(Debug, Clone)]
pub struct PyModule {
    pub body: Vec<PyStmt>,
    pub docstring: Option<String>,
}

/// Python expression
#[derive(Debug, Clone)]
pub enum PyExpr {
    // ── Literals ─────────────────────────────────────────
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BytesLiteral(Vec<u8>),
    BoolLiteral(bool),
    NoneLiteral,
    EllipsisLiteral,

    // ── F-string ─────────────────────────────────────────
    FString {
        parts: Vec<FStringPart>,
    },

    // ── Names ────────────────────────────────────────────
    Name(String),

    // ── Binary operations ────────────────────────────────
    BinOp {
        op: PyBinOp,
        left: Box<PyExpr>,
        right: Box<PyExpr>,
    },

    // ── Unary operations ─────────────────────────────────
    UnaryOp {
        op: PyUnaryOp,
        operand: Box<PyExpr>,
    },

    // ── Boolean operations ───────────────────────────────
    BoolOp {
        op: PyBoolOp,
        values: Vec<PyExpr>,
    },

    // ── Comparison ───────────────────────────────────────
    Compare {
        left: Box<PyExpr>,
        ops: Vec<PyCmpOp>,
        comparators: Vec<PyExpr>,
    },

    // ── Function call ────────────────────────────────────
    Call {
        func: Box<PyExpr>,
        args: Vec<PyExpr>,
        kwargs: Vec<(String, PyExpr)>,
        starargs: Option<Box<PyExpr>>,
        starkwargs: Option<Box<PyExpr>>,
    },

    // ── Attribute access ─────────────────────────────────
    Attribute {
        value: Box<PyExpr>,
        attr: String,
    },

    // ── Subscript ────────────────────────────────────────
    Subscript {
        value: Box<PyExpr>,
        slice: Box<PyExpr>,
    },

    // ── Slice ────────────────────────────────────────────
    Slice {
        lower: Option<Box<PyExpr>>,
        upper: Option<Box<PyExpr>>,
        step: Option<Box<PyExpr>>,
    },

    // ── Collections ──────────────────────────────────────
    List(Vec<PyExpr>),
    Tuple(Vec<PyExpr>),
    Set(Vec<PyExpr>),
    Dict {
        keys: Vec<Option<PyExpr>>,
        values: Vec<PyExpr>,
    },

    // ── Comprehensions ───────────────────────────────────
    ListComp {
        element: Box<PyExpr>,
        generators: Vec<PyComprehension>,
    },
    SetComp {
        element: Box<PyExpr>,
        generators: Vec<PyComprehension>,
    },
    DictComp {
        key: Box<PyExpr>,
        value: Box<PyExpr>,
        generators: Vec<PyComprehension>,
    },
    GeneratorExp {
        element: Box<PyExpr>,
        generators: Vec<PyComprehension>,
    },

    // ── Lambda ───────────────────────────────────────────
    Lambda {
        params: Vec<PyParam>,
        body: Box<PyExpr>,
    },

    // ── Conditional expression ───────────────────────────
    IfExpr {
        test: Box<PyExpr>,
        body: Box<PyExpr>,
        orelse: Box<PyExpr>,
    },

    // ── Walrus operator := (3.8+) ────────────────────────
    NamedExpr {
        target: Box<PyExpr>,
        value: Box<PyExpr>,
    },

    // ── Starred expression ───────────────────────────────
    Starred(Box<PyExpr>),

    // ── Await (3.5+) ─────────────────────────────────────
    Await(Box<PyExpr>),

    // ── Yield ────────────────────────────────────────────
    Yield(Option<Box<PyExpr>>),
    YieldFrom(Box<PyExpr>),
}

/// Python statement
#[derive(Debug, Clone)]
pub enum PyStmt {
    // ── Expressions ──────────────────────────────────────
    Expr(PyExpr),

    // ── Assignments ──────────────────────────────────────
    Assign {
        targets: Vec<PyExpr>,
        value: PyExpr,
    },
    AugAssign {
        target: PyExpr,
        op: PyBinOp,
        value: PyExpr,
    },
    AnnAssign {
        target: PyExpr,
        annotation: PyType,
        value: Option<PyExpr>,
    },

    // ── Control flow ─────────────────────────────────────
    If {
        test: PyExpr,
        body: Vec<PyStmt>,
        elif_clauses: Vec<(PyExpr, Vec<PyStmt>)>,
        orelse: Vec<PyStmt>,
    },
    While {
        test: PyExpr,
        body: Vec<PyStmt>,
        orelse: Vec<PyStmt>,
    },
    For {
        target: PyExpr,
        iter: PyExpr,
        body: Vec<PyStmt>,
        orelse: Vec<PyStmt>,
        is_async: bool,
    },
    Break,
    Continue,
    Pass,

    // ── Functions ────────────────────────────────────────
    FunctionDef {
        name: String,
        params: Vec<PyParam>,
        body: Vec<PyStmt>,
        decorators: Vec<PyExpr>,
        return_type: Option<PyType>,
        is_async: bool,
        docstring: Option<String>,
    },
    Return(Option<PyExpr>),

    // ── Classes ──────────────────────────────────────────
    ClassDef {
        name: String,
        bases: Vec<PyExpr>,
        body: Vec<PyStmt>,
        decorators: Vec<PyExpr>,
        docstring: Option<String>,
    },

    // ── Imports ──────────────────────────────────────────
    Import {
        names: Vec<PyAlias>,
    },
    ImportFrom {
        module: Option<String>,
        names: Vec<PyAlias>,
        level: usize,
    },

    // ── Exception handling ───────────────────────────────
    Try {
        body: Vec<PyStmt>,
        handlers: Vec<PyExceptHandler>,
        orelse: Vec<PyStmt>,
        finalbody: Vec<PyStmt>,
    },
    Raise {
        exc: Option<PyExpr>,
        cause: Option<PyExpr>,
    },

    // ── Context managers ─────────────────────────────────
    With {
        items: Vec<(PyExpr, Option<PyExpr>)>,
        body: Vec<PyStmt>,
        is_async: bool,
    },

    // ── Assertions ───────────────────────────────────────
    Assert {
        test: PyExpr,
        msg: Option<PyExpr>,
    },

    // ── Delete ───────────────────────────────────────────
    Delete(Vec<PyExpr>),

    // ── Global/Nonlocal ──────────────────────────────────
    Global(Vec<String>),
    Nonlocal(Vec<String>),

    // ── Match/Case (3.10+) ───────────────────────────────
    Match {
        subject: PyExpr,
        cases: Vec<PyMatchCase>,
    },
}

// ── Supporting types ─────────────────────────────────────

#[derive(Debug, Clone)]
pub enum FStringPart {
    Literal(String),
    Expression(PyExpr, Option<String>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyBinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    LShift,
    RShift,
    BitOr,
    BitXor,
    BitAnd,
    MatMul,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyUnaryOp {
    Pos,
    Neg,
    Invert,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyBoolOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyCmpOp {
    Eq,
    NotEq,
    Lt,
    LtE,
    Gt,
    GtE,
    Is,
    IsNot,
    In,
    NotIn,
}

#[derive(Debug, Clone)]
pub struct PyParam {
    pub name: String,
    pub annotation: Option<PyType>,
    pub default: Option<PyExpr>,
    pub kind: PyParamKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PyParamKind {
    Regular,
    VarPositional,
    VarKeyword,
    PositionalOnly,
    KeywordOnly,
}

#[derive(Debug, Clone)]
pub struct PyAlias {
    pub name: String,
    pub asname: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PyComprehension {
    pub target: PyExpr,
    pub iter: PyExpr,
    pub ifs: Vec<PyExpr>,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct PyExceptHandler {
    pub exc_type: Option<PyExpr>,
    pub name: Option<String>,
    pub body: Vec<PyStmt>,
}

#[derive(Debug, Clone)]
pub struct PyMatchCase {
    pub pattern: PyPattern,
    pub guard: Option<PyExpr>,
    pub body: Vec<PyStmt>,
}

#[derive(Debug, Clone)]
pub enum PyPattern {
    Literal(PyExpr),
    Capture(String),
    Wildcard,
    Sequence(Vec<PyPattern>),
    Mapping(Vec<(PyExpr, PyPattern)>),
    Class {
        cls: PyExpr,
        patterns: Vec<PyPattern>,
        kwd_attrs: Vec<String>,
        kwd_patterns: Vec<PyPattern>,
    },
    Star(Option<String>),
    Or(Vec<PyPattern>),
    Value(PyExpr),
}
