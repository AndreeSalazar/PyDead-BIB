/// Concrete type after inference (maps to IR types)
#[derive(Debug, Clone, PartialEq)]
pub enum ConcreteType {
    Int64,
    Float64,
    Bool,
    Str,
    Bytes,
    NoneType,
    List(Box<ConcreteType>),
    Dict(Box<ConcreteType>, Box<ConcreteType>),
    Set(Box<ConcreteType>),
    Tuple(Vec<ConcreteType>),
    Object(String),    // class instance
    Function {
        params: Vec<ConcreteType>,
        ret: Box<ConcreteType>,
    },
    Dynamic,           // could not infer — fallback
}
