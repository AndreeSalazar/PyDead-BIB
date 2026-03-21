// ============================================================
// Python Type Inferencer for PyDead-BIB
// ============================================================
// Duck typing → concrete static types
// PEP 484 type hints → guaranteed types
// Type propagation: a = 1 → a: int64 inferred
// Return type inference: def f() → return type
// Container types: list[int], dict[str, float]
// Gradual typing: typed + untyped coexist
// ============================================================

use super::py_ast::*;
use std::collections::HashMap;

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

// ══════════════════════════════════════════════════════════
// v4.0 — FASE 3: StructLayout & Deep Type Inference
// ══════════════════════════════════════════════════════════

/// Field in a struct layout — name, type, byte offset
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: ConcreteType,
    pub byte_offset: usize,
}

/// Struct layout for a class — ordered fields with offsets
#[derive(Debug, Clone)]
pub struct StructLayout {
    pub class_name: String,
    pub parent: Option<String>,
    pub fields: Vec<StructField>,
    pub total_size: usize,
    pub dynamic_warnings: Vec<String>,
}

impl StructLayout {
    pub fn new(name: &str) -> Self {
        Self {
            class_name: name.to_string(),
            parent: None,
            fields: Vec::new(),
            total_size: 8, // 8 bytes for class_id at offset 0
            dynamic_warnings: Vec::new(),
        }
    }

    /// Add a field to the layout, computing byte offset
    pub fn add_field(&mut self, name: &str, field_type: ConcreteType) {
        // Skip if field already exists (from parent)
        if self.fields.iter().any(|f| f.name == name) {
            return;
        }
        let offset = self.total_size;
        self.fields.push(StructField {
            name: name.to_string(),
            field_type: field_type.clone(),
            byte_offset: offset,
        });
        let field_size = match &field_type {
            ConcreteType::Int64 | ConcreteType::Float64 | ConcreteType::Str
            | ConcreteType::Bool | ConcreteType::NoneType => 8,
            ConcreteType::Object(_) => 8, // pointer
            ConcreteType::List(_) | ConcreteType::Dict(_, _) => 8, // pointer
            ConcreteType::Dynamic => 8, // pointer-sized fallback
            _ => 8,
        };
        self.total_size += field_size;
    }

    /// Get field offset by name
    pub fn field_offset(&self, name: &str) -> Option<usize> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.byte_offset)
    }

    /// Get field type by name
    pub fn field_type(&self, name: &str) -> Option<&ConcreteType> {
        self.fields.iter().find(|f| f.name == name).map(|f| &f.field_type)
    }
}

/// Type inference result for a scope
#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub bindings: HashMap<String, ConcreteType>,
    pub functions: HashMap<String, ConcreteType>,
}

/// Python type inferencer
pub struct PyTypeInferencer {
    env_stack: Vec<TypeEnv>,
    // v4.0 FASE 3: Class registry
    pub class_layouts: HashMap<String, StructLayout>,
    pub inheritance_chain: HashMap<String, Vec<String>>, // class → [parent, grandparent, ...]
    pub dynamic_fallbacks: Vec<String>, // compile-time warnings for Dynamic fields
}

impl PyTypeInferencer {
    pub fn new() -> Self {
        let mut global = TypeEnv {
            bindings: HashMap::new(),
            functions: HashMap::new(),
        };

        // Built-in functions
        global.functions.insert("print".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::NoneType),
        });
        global.functions.insert("len".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Int64),
        });
        global.functions.insert("range".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Int64],
            ret: Box::new(ConcreteType::List(Box::new(ConcreteType::Int64))),
        });
        global.functions.insert("int".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Int64),
        });
        global.functions.insert("float".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Float64),
        });
        global.functions.insert("str".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Str),
        });
        global.functions.insert("bool".to_string(), ConcreteType::Function {
            params: vec![ConcreteType::Dynamic],
            ret: Box::new(ConcreteType::Bool),
        });

        Self {
            env_stack: vec![global],
            class_layouts: HashMap::new(),
            inheritance_chain: HashMap::new(),
            dynamic_fallbacks: Vec::new(),
        }
    }

    /// Infer types for entire module — returns typed AST (passthrough for now)
    pub fn infer(&mut self, module: &PyModule) -> PyModule {
        let mut typed_module = module.clone();
        for stmt in &typed_module.body {
            self.infer_stmt(stmt);
        }
        typed_module
    }

    /// Infer types in a statement
    fn infer_stmt(&mut self, stmt: &PyStmt) {
        match stmt {
            PyStmt::Assign { targets, value } => {
                let val_type = self.infer_expr(value);
                for target in targets {
                    if let PyExpr::Name(name) = target {
                        self.current_env_mut().bindings.insert(name.clone(), val_type.clone());
                    }
                }
            }
            PyStmt::AnnAssign { target, annotation, value } => {
                let ann_type = self.annotation_to_concrete(annotation);
                if let PyExpr::Name(name) = target {
                    self.current_env_mut().bindings.insert(name.clone(), ann_type);
                }
                if let Some(val) = value {
                    self.infer_expr(val);
                }
            }
            PyStmt::FunctionDef { name, params, body, return_type, .. } => {
                let param_types: Vec<ConcreteType> = params.iter().map(|p| {
                    p.annotation.as_ref()
                        .map(|a| self.annotation_to_concrete(a))
                        .unwrap_or(ConcreteType::Dynamic)
                }).collect();
                let ret = return_type.as_ref()
                    .map(|r| self.annotation_to_concrete(r))
                    .unwrap_or(ConcreteType::Dynamic);
                self.current_env_mut().functions.insert(name.clone(), ConcreteType::Function {
                    params: param_types,
                    ret: Box::new(ret),
                });

                // Push scope for function body
                self.env_stack.push(TypeEnv {
                    bindings: HashMap::new(),
                    functions: HashMap::new(),
                });
                for s in body {
                    self.infer_stmt(s);
                }
                self.env_stack.pop();
            }
            PyStmt::If { test, body, elif_clauses, orelse } => {
                self.infer_expr(test);
                for s in body { self.infer_stmt(s); }
                for (t, b) in elif_clauses {
                    self.infer_expr(t);
                    for s in b { self.infer_stmt(s); }
                }
                for s in orelse { self.infer_stmt(s); }
            }
            PyStmt::While { test, body, orelse } => {
                self.infer_expr(test);
                for s in body { self.infer_stmt(s); }
                for s in orelse { self.infer_stmt(s); }
            }
            PyStmt::For { target: _, iter, body, orelse, .. } => {
                self.infer_expr(iter);
                for s in body { self.infer_stmt(s); }
                for s in orelse { self.infer_stmt(s); }
            }
            PyStmt::ClassDef { name, bases, body, .. } => {
                self.current_env_mut().bindings.insert(name.clone(), ConcreteType::Object(name.clone()));

                // v4.0 FASE 3: Build StructLayout with deep __init__ inference
                let mut layout = StructLayout::new(name);

                // Resolve inheritance chain
                let mut chain = Vec::new();
                for base in bases {
                    if let PyExpr::Name(base_name) = base {
                        chain.push(base_name.clone());
                        layout.parent = Some(base_name.clone());
                        // Copy parent fields (deep inheritance)
                        if let Some(parent_layout) = self.class_layouts.get(base_name).cloned() {
                            for field in &parent_layout.fields {
                                layout.add_field(&field.name, field.field_type.clone());
                            }
                            // Also inherit grandparent chain
                            if let Some(parent_chain) = self.inheritance_chain.get(base_name).cloned() {
                                chain.extend(parent_chain);
                            }
                        }
                    }
                }
                self.inheritance_chain.insert(name.clone(), chain);

                // Deep __init__ inference: scan self.x = val assignments
                for s in body {
                    if let PyStmt::FunctionDef { name: method_name, body: method_body, .. } = s {
                        if method_name == "__init__" {
                            for ms in method_body {
                                if let PyStmt::Assign { targets, value } = ms {
                                    for t in targets {
                                        if let PyExpr::Attribute { value: target_obj, attr } = t {
                                            if let PyExpr::Name(n) = target_obj.as_ref() {
                                                if n == "self" {
                                                    // Infer field type from RHS
                                                    let field_type = self.infer_expr(value);
                                                    if field_type == ConcreteType::Dynamic {
                                                        self.dynamic_fallbacks.push(
                                                            format!("{}::{}: type could not be inferred, using Dynamic", name, attr)
                                                        );
                                                    }
                                                    layout.add_field(attr, field_type);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.class_layouts.insert(name.clone(), layout);

                for s in body { self.infer_stmt(s); }
            }
            _ => {}
        }
    }

    /// Infer type of expression
    fn infer_expr(&self, expr: &PyExpr) -> ConcreteType {
        match expr {
            PyExpr::IntLiteral(_) => ConcreteType::Int64,
            PyExpr::FloatLiteral(_) => ConcreteType::Float64,
            PyExpr::BoolLiteral(_) => ConcreteType::Bool,
            PyExpr::StringLiteral(_) | PyExpr::FString { .. } => ConcreteType::Str,
            PyExpr::BytesLiteral(_) => ConcreteType::Bytes,
            PyExpr::NoneLiteral => ConcreteType::NoneType,
            PyExpr::Name(name) => {
                self.lookup_var(name).unwrap_or(ConcreteType::Dynamic)
            }
            PyExpr::BinOp { op, left, right } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                self.infer_binop(op, &lt, &rt)
            }
            PyExpr::UnaryOp { op: _, operand } => self.infer_expr(operand),
            PyExpr::BoolOp { .. } => ConcreteType::Bool,
            PyExpr::Compare { .. } => ConcreteType::Bool,
            PyExpr::Call { func, .. } => {
                if let PyExpr::Name(name) = func.as_ref() {
                    self.lookup_function_return(name).unwrap_or(ConcreteType::Dynamic)
                } else {
                    ConcreteType::Dynamic
                }
            }
            PyExpr::List(elts) => {
                let elem_type = elts.first()
                    .map(|e| self.infer_expr(e))
                    .unwrap_or(ConcreteType::Dynamic);
                ConcreteType::List(Box::new(elem_type))
            }
            PyExpr::Dict { keys, values } => {
                let kt = keys.first().and_then(|k| k.as_ref())
                    .map(|k| self.infer_expr(k))
                    .unwrap_or(ConcreteType::Dynamic);
                let vt = values.first()
                    .map(|v| self.infer_expr(v))
                    .unwrap_or(ConcreteType::Dynamic);
                ConcreteType::Dict(Box::new(kt), Box::new(vt))
            }
            PyExpr::Tuple(elts) => {
                let types: Vec<ConcreteType> = elts.iter().map(|e| self.infer_expr(e)).collect();
                ConcreteType::Tuple(types)
            }
            PyExpr::Subscript { value, .. } => {
                let vt = self.infer_expr(value);
                match vt {
                    ConcreteType::List(inner) => *inner,
                    ConcreteType::Dict(_, val) => *val,
                    ConcreteType::Str => ConcreteType::Str,
                    _ => ConcreteType::Dynamic,
                }
            }
            PyExpr::Attribute { .. } => ConcreteType::Dynamic,
            _ => ConcreteType::Dynamic,
        }
    }

    fn infer_binop(&self, op: &PyBinOp, left: &ConcreteType, right: &ConcreteType) -> ConcreteType {
        match (left, right) {
            (ConcreteType::Int64, ConcreteType::Int64) => match op {
                PyBinOp::Div => ConcreteType::Float64,
                _ => ConcreteType::Int64,
            },
            (ConcreteType::Float64, _) | (_, ConcreteType::Float64) => ConcreteType::Float64,
            (ConcreteType::Str, ConcreteType::Str) if *op == PyBinOp::Add => ConcreteType::Str,
            (ConcreteType::Str, ConcreteType::Int64) if *op == PyBinOp::Mul => ConcreteType::Str,
            (ConcreteType::List(_), ConcreteType::List(_)) if *op == PyBinOp::Add => left.clone(),
            _ => ConcreteType::Dynamic,
        }
    }

    fn annotation_to_concrete(&self, ann: &PyType) -> ConcreteType {
        match ann {
            PyType::Int => ConcreteType::Int64,
            PyType::Float => ConcreteType::Float64,
            PyType::Str => ConcreteType::Str,
            PyType::Bool => ConcreteType::Bool,
            PyType::None => ConcreteType::NoneType,
            PyType::Bytes => ConcreteType::Bytes,
            PyType::List(inner) => ConcreteType::List(Box::new(self.annotation_to_concrete(inner))),
            PyType::Dict(k, v) => ConcreteType::Dict(
                Box::new(self.annotation_to_concrete(k)),
                Box::new(self.annotation_to_concrete(v)),
            ),
            PyType::Set(inner) => ConcreteType::Set(Box::new(self.annotation_to_concrete(inner))),
            PyType::Tuple(elts) => ConcreteType::Tuple(elts.iter().map(|e| self.annotation_to_concrete(e)).collect()),
            PyType::Optional(inner) => self.annotation_to_concrete(inner),
            PyType::Any | PyType::Inferred => ConcreteType::Dynamic,
            PyType::Custom(name) => ConcreteType::Object(name.clone()),
            _ => ConcreteType::Dynamic,
        }
    }

    fn lookup_var(&self, name: &str) -> Option<ConcreteType> {
        for env in self.env_stack.iter().rev() {
            if let Some(t) = env.bindings.get(name) {
                return Some(t.clone());
            }
        }
        std::option::Option::None
    }

    fn lookup_function_return(&self, name: &str) -> Option<ConcreteType> {
        for env in self.env_stack.iter().rev() {
            if let Some(ConcreteType::Function { ret, .. }) = env.functions.get(name) {
                return Some(*ret.clone());
            }
        }
        std::option::Option::None
    }

    fn current_env_mut(&mut self) -> &mut TypeEnv {
        self.env_stack.last_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_int_literal() {
        let inf = PyTypeInferencer::new();
        let t = inf.infer_expr(&PyExpr::IntLiteral(42));
        assert_eq!(t, ConcreteType::Int64);
    }

    #[test]
    fn test_infer_float_promotion() {
        let inf = PyTypeInferencer::new();
        let t = inf.infer_binop(
            &PyBinOp::Add,
            &ConcreteType::Int64,
            &ConcreteType::Float64,
        );
        assert_eq!(t, ConcreteType::Float64);
    }

    #[test]
    fn test_infer_div_returns_float() {
        let inf = PyTypeInferencer::new();
        let t = inf.infer_binop(
            &PyBinOp::Div,
            &ConcreteType::Int64,
            &ConcreteType::Int64,
        );
        assert_eq!(t, ConcreteType::Float64);
    }

    #[test]
    fn test_infer_string_concat() {
        let inf = PyTypeInferencer::new();
        let t = inf.infer_binop(
            &PyBinOp::Add,
            &ConcreteType::Str,
            &ConcreteType::Str,
        );
        assert_eq!(t, ConcreteType::Str);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// v4.3 — TYPE STRICTNESS ULTRA — RESPETO DE BITS
// ══════════════════════════════════════════════════════════════════════════════
// Filosofía: FORTRAN 1957 + Ada 1983 + PyDead-BIB 2025
// "Cada tipo respeta sus bits — sin excepciones"
// Sin conversión implícita NUNCA. El dev debe ser EXPLÍCITO.
// ══════════════════════════════════════════════════════════════════════════════

/// Result of type compatibility check
#[derive(Debug, Clone)]
pub enum TypeCompatResult {
    /// Types are compatible, result type provided
    Ok(ConcreteType),
    /// Types are incompatible — compilation blocked
    Mismatch {
        left: ConcreteType,
        right: ConcreteType,
        op: String,
        suggestions: Vec<String>,
    },
}

/// Check if two types are compatible for a binary operation
/// ULTRA STRICT: int + float = ERROR, float + int = ERROR
pub fn types_compatible(
    left: &ConcreteType,
    right: &ConcreteType,
    op: &PyBinOp,
) -> TypeCompatResult {
    use ConcreteType::*;
    use PyBinOp::*;
    
    match (left, right, op) {
        // ═══════════════════════════════════════════════════════════
        // PERMITIDOS — Mismo tipo con mismo tipo
        // ═══════════════════════════════════════════════════════════
        
        // int OP int = int ✅
        (Int64, Int64, Add | Sub | Mul | FloorDiv | Mod | Pow | BitAnd | BitOr | BitXor | LShift | RShift) => 
            TypeCompatResult::Ok(Int64),
        
        // int / int = float ✅ (división siempre float)
        (Int64, Int64, Div) => TypeCompatResult::Ok(Float64),
        
        // float OP float = float ✅
        (Float64, Float64, Add | Sub | Mul | Div | FloorDiv | Mod | Pow) => 
            TypeCompatResult::Ok(Float64),
        
        // str + str = str ✅ (concatenación)
        (Str, Str, Add) => TypeCompatResult::Ok(Str),
        
        // str * int = str ✅ (repetición)
        (Str, Int64, Mul) => TypeCompatResult::Ok(Str),
        (Int64, Str, Mul) => TypeCompatResult::Ok(Str),
        
        // list + list = list ✅ (concatenación)
        (List(t1), List(t2), Add) if t1 == t2 => 
            TypeCompatResult::Ok(List(t1.clone())),
        
        // list * int = list ✅ (repetición)
        (List(t), Int64, Mul) => TypeCompatResult::Ok(List(t.clone())),
        (Int64, List(t), Mul) => TypeCompatResult::Ok(List(t.clone())),
        
        // bool + bool = int ✅ (True + True = 2)
        (Bool, Bool, Add | Sub | Mul) => TypeCompatResult::Ok(Int64),
        
        // bool OP int = int ✅ (bool es subtype de int)
        (Bool, Int64, Add | Sub | Mul | FloorDiv | Mod) => TypeCompatResult::Ok(Int64),
        (Int64, Bool, Add | Sub | Mul | FloorDiv | Mod) => TypeCompatResult::Ok(Int64),
        
        // ═══════════════════════════════════════════════════════════
        // BLOQUEADOS — Tipos incompatibles 💀
        // ═══════════════════════════════════════════════════════════
        
        // int + float = ERROR 💀
        (Int64, Float64, Add | Sub | Mul | Div | FloorDiv | Mod | Pow) => {
            TypeCompatResult::Mismatch {
                left: Int64,
                right: Float64,
                op: format!("{:?}", op),
                suggestions: vec![
                    "float(x) + y  ← convertir int a float".to_string(),
                    "x + int(y)    ← convertir float a int".to_string(),
                ],
            }
        }
        
        // float + int = ERROR 💀
        (Float64, Int64, Add | Sub | Mul | Div | FloorDiv | Mod | Pow) => {
            TypeCompatResult::Mismatch {
                left: Float64,
                right: Int64,
                op: format!("{:?}", op),
                suggestions: vec![
                    "x + float(y)  ← convertir int a float".to_string(),
                    "int(x) + y    ← convertir float a int".to_string(),
                ],
            }
        }
        
        // str + int = ERROR 💀
        (Str, Int64, Add) => {
            TypeCompatResult::Mismatch {
                left: Str,
                right: Int64,
                op: "Add".to_string(),
                suggestions: vec![
                    "x + str(y)    ← convertir int a str".to_string(),
                ],
            }
        }
        
        // int + str = ERROR 💀
        (Int64, Str, Add) => {
            TypeCompatResult::Mismatch {
                left: Int64,
                right: Str,
                op: "Add".to_string(),
                suggestions: vec![
                    "str(x) + y    ← convertir int a str".to_string(),
                ],
            }
        }
        
        // str + float = ERROR 💀
        (Str, Float64, Add) => {
            TypeCompatResult::Mismatch {
                left: Str,
                right: Float64,
                op: "Add".to_string(),
                suggestions: vec![
                    "x + str(y)    ← convertir float a str".to_string(),
                ],
            }
        }
        
        // float + str = ERROR 💀
        (Float64, Str, Add) => {
            TypeCompatResult::Mismatch {
                left: Float64,
                right: Str,
                op: "Add".to_string(),
                suggestions: vec![
                    "str(x) + y    ← convertir float a str".to_string(),
                ],
            }
        }
        
        // bool + float = ERROR 💀
        (Bool, Float64, _) | (Float64, Bool, _) => {
            TypeCompatResult::Mismatch {
                left: left.clone(),
                right: right.clone(),
                op: format!("{:?}", op),
                suggestions: vec![
                    "Usa conversión explícita: float(bool_val) o int(float_val)".to_string(),
                ],
            }
        }
        
        // list + int = ERROR 💀 (excepto multiplicación)
        (List(_), Int64, Add | Sub | Div) | (Int64, List(_), Add | Sub | Div) => {
            TypeCompatResult::Mismatch {
                left: left.clone(),
                right: right.clone(),
                op: format!("{:?}", op),
                suggestions: vec![
                    "list + list   ← concatenar listas".to_string(),
                    "list * int    ← repetir lista".to_string(),
                ],
            }
        }
        
        // str - str = ERROR 💀 (no existe resta de strings)
        (Str, Str, Sub | Mul | Div | FloorDiv | Mod | Pow) => {
            TypeCompatResult::Mismatch {
                left: Str,
                right: Str,
                op: format!("{:?}", op),
                suggestions: vec![
                    "str + str     ← concatenación permitida".to_string(),
                    "str * int     ← repetición permitida".to_string(),
                ],
            }
        }
        
        // str * float = ERROR 💀
        (Str, Float64, Mul) | (Float64, Str, Mul) => {
            TypeCompatResult::Mismatch {
                left: left.clone(),
                right: right.clone(),
                op: "Mul".to_string(),
                suggestions: vec![
                    "str * int(n)  ← convertir float a int para repetición".to_string(),
                ],
            }
        }
        
        // Dynamic fallback — permitir para compatibilidad
        (Dynamic, _, _) | (_, Dynamic, _) => TypeCompatResult::Ok(Dynamic),
        
        // Todo lo demás = ERROR 💀
        _ => {
            TypeCompatResult::Mismatch {
                left: left.clone(),
                right: right.clone(),
                op: format!("{:?}", op),
                suggestions: vec![
                    "Verifica los tipos y usa conversión explícita".to_string(),
                ],
            }
        }
    }
}

/// Check if comparison between two types is valid
pub fn types_comparable(left: &ConcreteType, right: &ConcreteType) -> TypeCompatResult {
    use ConcreteType::*;
    
    match (left, right) {
        // Mismo tipo = comparación válida ✅
        (Int64, Int64) | (Float64, Float64) | (Str, Str) | (Bool, Bool) => 
            TypeCompatResult::Ok(Bool),
        
        // bool es subtype de int ✅
        (Bool, Int64) | (Int64, Bool) => TypeCompatResult::Ok(Bool),
        
        // None comparaciones siempre permitidas ✅
        (NoneType, _) | (_, NoneType) => TypeCompatResult::Ok(Bool),
        
        // Dynamic fallback
        (Dynamic, _) | (_, Dynamic) => TypeCompatResult::Ok(Bool),
        
        // int == float = ERROR 💀
        (Int64, Float64) | (Float64, Int64) => {
            TypeCompatResult::Mismatch {
                left: left.clone(),
                right: right.clone(),
                op: "Compare".to_string(),
                suggestions: vec![
                    "float(x) == y  ← comparar como floats".to_string(),
                    "x == int(y)    ← comparar como ints".to_string(),
                ],
            }
        }
        
        // str == int = ERROR 💀
        (Str, Int64) | (Int64, Str) | (Str, Float64) | (Float64, Str) => {
            TypeCompatResult::Mismatch {
                left: left.clone(),
                right: right.clone(),
                op: "Compare".to_string(),
                suggestions: vec![
                    "Tipos incompatibles para comparación".to_string(),
                ],
            }
        }
        
        // Listas del mismo tipo ✅
        (List(t1), List(t2)) if t1 == t2 => TypeCompatResult::Ok(Bool),
        
        // Todo lo demás = ERROR 💀
        _ => {
            TypeCompatResult::Mismatch {
                left: left.clone(),
                right: right.clone(),
                op: "Compare".to_string(),
                suggestions: vec![
                    "Tipos incompatibles para comparación".to_string(),
                ],
            }
        }
    }
}

/// Check if list elements are homogeneous
pub fn check_list_homogeneity(elements: &[ConcreteType]) -> Result<ConcreteType, (ConcreteType, ConcreteType)> {
    if elements.is_empty() {
        return Ok(ConcreteType::Dynamic);
    }
    
    let first = &elements[0];
    for elem in &elements[1..] {
        if elem != first && !matches!(elem, ConcreteType::Dynamic) && !matches!(first, ConcreteType::Dynamic) {
            return Err((first.clone(), elem.clone()));
        }
    }
    
    Ok(ConcreteType::List(Box::new(first.clone())))
}

/// Check if assignment is type-compatible
pub fn check_assignment_compatible(
    target_type: Option<&ConcreteType>,
    value_type: &ConcreteType,
) -> TypeCompatResult {
    match target_type {
        Some(expected) => {
            if expected == value_type {
                TypeCompatResult::Ok(value_type.clone())
            } else if matches!(expected, ConcreteType::Dynamic) || matches!(value_type, ConcreteType::Dynamic) {
                TypeCompatResult::Ok(value_type.clone())
            } else {
                TypeCompatResult::Mismatch {
                    left: expected.clone(),
                    right: value_type.clone(),
                    op: "Assign".to_string(),
                    suggestions: vec![
                        format!("{}(valor) ← conversión explícita", type_to_conversion_fn(expected)),
                    ],
                }
            }
        }
        None => TypeCompatResult::Ok(value_type.clone()),
    }
}

/// Get conversion function name for a type
pub fn type_to_conversion_fn(t: &ConcreteType) -> &'static str {
    match t {
        ConcreteType::Int64 => "int",
        ConcreteType::Float64 => "float",
        ConcreteType::Str => "str",
        ConcreteType::Bool => "bool",
        ConcreteType::Bytes => "bytes",
        _ => "type",
    }
}

/// Format type for error messages
pub fn format_type(t: &ConcreteType) -> String {
    match t {
        ConcreteType::Int64 => "int".to_string(),
        ConcreteType::Float64 => "float".to_string(),
        ConcreteType::Str => "str".to_string(),
        ConcreteType::Bool => "bool".to_string(),
        ConcreteType::Bytes => "bytes".to_string(),
        ConcreteType::NoneType => "None".to_string(),
        ConcreteType::List(inner) => format!("List[{}]", format_type(inner)),
        ConcreteType::Dict(k, v) => format!("Dict[{}, {}]", format_type(k), format_type(v)),
        ConcreteType::Tuple(elems) => {
            let inner: Vec<_> = elems.iter().map(format_type).collect();
            format!("Tuple[{}]", inner.join(", "))
        }
        ConcreteType::Object(name) => name.clone(),
        ConcreteType::Dynamic => "Dynamic".to_string(),
        _ => "Unknown".to_string(),
    }
}
