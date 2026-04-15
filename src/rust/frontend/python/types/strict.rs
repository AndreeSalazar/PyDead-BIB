use super::concrete::ConcreteType;
use crate::frontend::python::ast::PyBinOp;
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
