use super::expr::{PyExpr, PyBinOp};
use super::types::PyType;

#[derive(Debug, Clone)]
pub struct PyModule {
    pub body: Vec<PyStmt>,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PyStmt {
    Expr(PyExpr),
    Assign { targets: Vec<PyExpr>, value: PyExpr },
    AugAssign { target: PyExpr, op: PyBinOp, value: PyExpr },
    AnnAssign { target: PyExpr, annotation: PyType, value: Option<PyExpr> },
    If { test: PyExpr, body: Vec<PyStmt>, elif_clauses: Vec<(PyExpr, Vec<PyStmt>)>, orelse: Vec<PyStmt> },
    While { test: PyExpr, body: Vec<PyStmt>, orelse: Vec<PyStmt> },
    For { target: PyExpr, iter: PyExpr, body: Vec<PyStmt>, orelse: Vec<PyStmt>, is_async: bool },
    Break,
    Continue,
    Pass,
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
    ClassDef {
        name: String,
        bases: Vec<PyExpr>,
        body: Vec<PyStmt>,
        decorators: Vec<PyExpr>,
        docstring: Option<String>,
    },
    Import { names: Vec<PyAlias> },
    ImportFrom { module: Option<String>, names: Vec<PyAlias>, level: usize },
    Try { body: Vec<PyStmt>, handlers: Vec<PyExceptHandler>, orelse: Vec<PyStmt>, finalbody: Vec<PyStmt> },
    Raise { exc: Option<PyExpr>, cause: Option<PyExpr> },
    With { items: Vec<(PyExpr, Option<PyExpr>)>, body: Vec<PyStmt>, is_async: bool },
    Assert { test: PyExpr, msg: Option<PyExpr> },
    Delete(Vec<PyExpr>),
    Global(Vec<String>),
    Nonlocal(Vec<String>),
    Match { subject: PyExpr, cases: Vec<PyMatchCase> },
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
