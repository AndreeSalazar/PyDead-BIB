/// Python-specific undefined behavior types
#[derive(Debug, Clone, PartialEq)]
pub enum PythonUB {
    // ── Heredados de C (aplicables) ──────────────────────
    DivisionByZero,
    IntegerOverflow,
    UninitializedVariable,

    // ── Python-specific ──────────────────────────────────
    NoneDeref,                 // None.atributo → AttributeError pre-detectado
    IndexOutOfBounds,          // lista[100] con lista de 10 → pre-detectado
    KeyNotFound,               // dict["x"] sin "x" → pre-detectado
    TypeMismatch,              // "hola" + 42 → TypeError pre-detectado
    InfiniteRecursion,         // recursión sin base case → detectado
    CircularImport,            // A importa B, B importa A → detectado
    MutableDefaultArg,         // def f(x=[]) → bug clásico Python → warning
    GlobalWithoutDeclaration,  // modifica global sin 'global' → warning
    IteratorExhausted,         // reusar generator ya consumido → detectado
    UnpackMismatch,            // a, b = [1, 2, 3] → demasiados valores

    // ── v4.2 — Memory Safety (C ABI) ─────────────────────
    UseAfterFree,              // Usar memoria después de liberar
    BufferOverflow,            // Escribir fuera de bounds de array/struct
    DoubleFree,                // Liberar memoria ya liberada
    NullPointerDeref,          // Dereferenciar puntero nulo en C ABI

    // ── v4.3 — Tipos Estrictos (como Fortran) ────────────
    MixedArithmetic,           // int + float → ERROR (debe ser explícito)
    ImplicitCoercion,          // Conversión implícita de tipos → ERROR
}

/// Severity level for UB reports
#[derive(Debug, Clone, PartialEq)]
pub enum UBSeverity {
    Error,
    Warning,
    Info,
}

/// UB detection report
#[derive(Debug, Clone)]
pub struct UBReport {
    pub kind: PythonUB,
    pub severity: UBSeverity,
    pub message: String,
    pub line: usize,
    pub col: usize,
    pub file: String,
    pub suggestion: Option<String>,
}
