use super::stmt::PyParam;

#[derive(Debug, Clone)]
pub enum PyExpr {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BytesLiteral(Vec<u8>),
    BoolLiteral(bool),
    NoneLiteral,
    EllipsisLiteral,
    FString { parts: Vec<FStringPart> },
    Name(String),
    BinOp { op: PyBinOp, left: Box<PyExpr>, right: Box<PyExpr> },
    UnaryOp { op: PyUnaryOp, operand: Box<PyExpr> },
    BoolOp { op: PyBoolOp, values: Vec<PyExpr> },
    Compare { left: Box<PyExpr>, ops: Vec<PyCmpOp>, comparators: Vec<PyExpr> },
    Call {
        func: Box<PyExpr>,
        args: Vec<PyExpr>,
        kwargs: Vec<(String, PyExpr)>,
        starargs: Option<Box<PyExpr>>,
        starkwargs: Option<Box<PyExpr>>,
    },
    Attribute { value: Box<PyExpr>, attr: String },
    Subscript { value: Box<PyExpr>, slice: Box<PyExpr> },
    Slice { lower: Option<Box<PyExpr>>, upper: Option<Box<PyExpr>>, step: Option<Box<PyExpr>> },
    List(Vec<PyExpr>),
    Tuple(Vec<PyExpr>),
    Set(Vec<PyExpr>),
    Dict { keys: Vec<Option<PyExpr>>, values: Vec<PyExpr> },
    ListComp { element: Box<PyExpr>, generators: Vec<PyComprehension> },
    SetComp { element: Box<PyExpr>, generators: Vec<PyComprehension> },
    DictComp { key: Box<PyExpr>, value: Box<PyExpr>, generators: Vec<PyComprehension> },
    GeneratorExp { element: Box<PyExpr>, generators: Vec<PyComprehension> },
    Lambda { params: Vec<PyParam>, body: Box<PyExpr> },
    IfExpr { test: Box<PyExpr>, body: Box<PyExpr>, orelse: Box<PyExpr> },
    NamedExpr { target: Box<PyExpr>, value: Box<PyExpr> },
    Starred(Box<PyExpr>),
    Await(Box<PyExpr>),
    Yield(Option<Box<PyExpr>>),
    YieldFrom(Box<PyExpr>),
}

#[derive(Debug, Clone)]
pub enum FStringPart {
    Literal(String),
    Expression(PyExpr, Option<String>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyBinOp {
    Add, Sub, Mul, Div, FloorDiv, Mod, Pow, LShift, RShift, BitOr, BitXor, BitAnd, MatMul,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyUnaryOp {
    Pos, Neg, Invert, Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyBoolOp {
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyCmpOp {
    Eq, NotEq, Lt, LtE, Gt, GtE, Is, IsNot, In, NotIn,
}

#[derive(Debug, Clone)]
pub struct PyComprehension {
    pub target: PyExpr,
    pub iter: PyExpr,
    pub ifs: Vec<PyExpr>,
    pub is_async: bool,
}
