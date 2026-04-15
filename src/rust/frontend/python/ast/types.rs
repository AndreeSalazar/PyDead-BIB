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
