use super::types::{PythonUB, UBSeverity, UBReport};
/// Python UB Detector — compile-time error detection
pub struct PyUBDetector {
    reports: Vec<UBReport>,
    file: String,
    strict_mode: bool,
}

impl PyUBDetector {
    /// PyDead-BIB es ESTRICTO por defecto — UB NO EXISTE
    /// Cualquier UB detectado bloquea la compilación
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
            file: String::new(),
            strict_mode: true,  // ESTRICTO por defecto — UB NO PERMITIDO
        }
    }

    pub fn with_file(mut self, file: String) -> Self {
        self.file = file;
        self
    }

    pub fn with_strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Analyze IR program for undefined behavior
    pub fn analyze(&mut self, program: &crate::frontend::python::to_ir::IRProgram) -> &[UBReport] {
        // Check each function
        for func in &program.functions {
            self.check_function(func);
        }

        // Check globals
        for global in &program.globals {
            self.check_global(global);
        }

        &self.reports
    }

    fn check_function(&mut self, func: &crate::middle::ir::IRFunction) {
        use crate::middle::ir::{IRInstruction, IROp, IRConstValue, IRType};

        // ── Check for empty return in non-void function ─────────
        if func.return_type != IRType::Void {
            for (i, instr) in func.body.iter().enumerate() {
                if matches!(instr, IRInstruction::ReturnVoid) {
                    self.reports.push(UBReport {
                        kind: PythonUB::TypeMismatch,
                        severity: UBSeverity::Warning,
                        message: format!(
                            "Empty return in non-void function '{}' (expected {:?})",
                            func.name, func.return_type
                        ),
                        line: i,
                        col: 0,
                        file: self.file.clone(),
                        suggestion: Some("Return a value matching the declared return type".to_string()),
                    });
                }
            }
        }

        for (i, instr) in func.body.iter().enumerate() {
            match instr {
                // ── Division by zero detection ──────────────────
                IRInstruction::BinOp { op: IROp::Div, right, .. }
                | IRInstruction::BinOp { op: IROp::FloorDiv, right, .. }
                | IRInstruction::BinOp { op: IROp::Mod, right, .. } => {
                    if self.is_zero_constant(right) {
                        self.reports.push(UBReport {
                            kind: PythonUB::DivisionByZero,
                            severity: UBSeverity::Error,
                            message: format!("Division by zero detected in function '{}'", func.name),
                            line: i,
                            col: 0,
                            file: self.file.clone(),
                            suggestion: Some("Check divisor is not zero before dividing".to_string()),
                        });
                    }
                }

                // ── Type mismatch: str + int ────────────────────
                IRInstruction::BinOp { op: IROp::Add, left, right, .. } => {
                    let left_is_str = matches!(left.as_ref(), IRInstruction::LoadString(_));
                    let right_is_str = matches!(right.as_ref(), IRInstruction::LoadString(_));
                    let left_is_int = matches!(left.as_ref(), IRInstruction::LoadConst(IRConstValue::Int(_)));
                    let right_is_int = matches!(right.as_ref(), IRInstruction::LoadConst(IRConstValue::Int(_)));
                    let left_is_float = matches!(left.as_ref(), IRInstruction::LoadConst(IRConstValue::Float(_)));
                    let right_is_float = matches!(right.as_ref(), IRInstruction::LoadConst(IRConstValue::Float(_)));

                    // str + int → ERROR
                    if (left_is_str && right_is_int) || (left_is_int && right_is_str) {
                        self.reports.push(UBReport {
                            kind: PythonUB::TypeMismatch,
                            severity: UBSeverity::Error,
                            message: format!(
                                "Type mismatch in function '{}': cannot add str and int",
                                func.name
                            ),
                            line: i,
                            col: 0,
                            file: self.file.clone(),
                            suggestion: Some("Use str() to convert the integer or use f-strings".to_string()),
                        });
                    }

                    // ── v4.3 Tipos Estrictos: int + float → ERROR (como Fortran) ──
                    if (left_is_int && right_is_float) || (left_is_float && right_is_int) {
                        self.reports.push(UBReport {
                            kind: PythonUB::MixedArithmetic,
                            severity: UBSeverity::Error,
                            message: format!(
                                "Mixed arithmetic in function '{}': int + float requires explicit conversion",
                                func.name
                            ),
                            line: i,
                            col: 0,
                            file: self.file.clone(),
                            suggestion: Some("Use float(x) or int(x) for explicit type conversion. PyDead-BIB respects bit-level types like Fortran.".to_string()),
                        });
                    }
                }

                // ── v4.3 Tipos Estrictos: Mul, Sub, Div con tipos mixtos ──
                IRInstruction::BinOp { op, left, right, .. } 
                    if matches!(op, IROp::Mul | IROp::Sub) => {
                    let left_is_int = matches!(left.as_ref(), IRInstruction::LoadConst(IRConstValue::Int(_)));
                    let right_is_int = matches!(right.as_ref(), IRInstruction::LoadConst(IRConstValue::Int(_)));
                    let left_is_float = matches!(left.as_ref(), IRInstruction::LoadConst(IRConstValue::Float(_)));
                    let right_is_float = matches!(right.as_ref(), IRInstruction::LoadConst(IRConstValue::Float(_)));

                    if (left_is_int && right_is_float) || (left_is_float && right_is_int) {
                        self.reports.push(UBReport {
                            kind: PythonUB::MixedArithmetic,
                            severity: UBSeverity::Error,
                            message: format!(
                                "Mixed arithmetic in function '{}': {:?} with int and float requires explicit conversion",
                                func.name, op
                            ),
                            line: i,
                            col: 0,
                            file: self.file.clone(),
                            suggestion: Some("Use float(x) or int(x) for explicit type conversion. INT + INT, Float + Float only.".to_string()),
                        });
                    }
                }

                // ── Integer overflow: large Pow operands ────────
                IRInstruction::BinOp {
                    op: IROp::Pow,
                    left,
                    right,
                    ..
                } => {
                    if let (
                        IRInstruction::LoadConst(IRConstValue::Int(base)),
                        IRInstruction::LoadConst(IRConstValue::Int(exp)),
                    ) = (left.as_ref(), right.as_ref())
                    {
                        if Self::pow_may_overflow(*base, *exp) {
                            self.reports.push(UBReport {
                                kind: PythonUB::IntegerOverflow,
                                severity: UBSeverity::Warning,
                                message: format!(
                                    "Potential integer overflow in function '{}': {}**{} produces a very large value",
                                    func.name, base, exp
                                ),
                                line: i,
                                col: 0,
                                file: self.file.clone(),
                                suggestion: Some("Consider whether this large exponentiation is intentional".to_string()),
                            });
                        }
                    }
                }

                // ── Mutable default argument heuristic ──────────
                IRInstruction::Call { func: callee, args } => {
                    if self.is_mutable_constructor(callee) {
                        // A bare list()/dict()/set() as a default arg pattern
                        self.reports.push(UBReport {
                            kind: PythonUB::MutableDefaultArg,
                            severity: UBSeverity::Warning,
                            message: format!(
                                "Call to mutable constructor '{}()' in function '{}' — if used as default argument, this is a classic Python bug",
                                callee, func.name
                            ),
                            line: i,
                            col: 0,
                            file: self.file.clone(),
                            suggestion: Some("Use None as default and create the mutable object inside the function body".to_string()),
                        });
                    }

                    // ── NoneDeref heuristic: calling method on None ──
                    for arg in args {
                        if matches!(arg, IRInstruction::LoadConst(IRConstValue::None)) {
                            self.reports.push(UBReport {
                                kind: PythonUB::NoneDeref,
                                severity: UBSeverity::Error,
                                message: format!(
                                    "Possible None dereference in call to '{}' in function '{}'",
                                    callee, func.name
                                ),
                                line: i,
                                col: 0,
                                file: self.file.clone(),
                                suggestion: Some("Check for None before calling methods or accessing attributes".to_string()),
                            });
                        }
                    }
                }

                _ => {}
            }
        }

        // ── NoneDeref: Load followed by Call on a None-named variable ──
        for (i, window) in func.body.windows(2).enumerate() {
            if let (
                IRInstruction::LoadConst(IRConstValue::None),
                IRInstruction::Call { func: callee, .. },
            ) = (&window[0], &window[1])
            {
                self.reports.push(UBReport {
                    kind: PythonUB::NoneDeref,
                    severity: UBSeverity::Error,
                    message: format!(
                        "None value used before call to '{}' in function '{}' — likely AttributeError at runtime",
                        callee, func.name
                    ),
                    line: i + 1,
                    col: 0,
                    file: self.file.clone(),
                    suggestion: Some("Add a None check before this call".to_string()),
                });
            }
        }
    }

    fn check_global(&mut self, global: &crate::frontend::python::to_ir::IRGlobal) {
        // Check for uninitialized variables
        if global.init_value.is_none() {
            self.reports.push(UBReport {
                kind: PythonUB::UninitializedVariable,
                severity: UBSeverity::Warning,
                message: format!("Global '{}' declared without initialization", global.name),
                line: 0,
                col: 0,
                file: self.file.clone(),
                suggestion: Some(format!("Initialize '{}' with a default value", global.name)),
            });
        }
    }

    fn is_zero_constant(&self, instr: &crate::middle::ir::IRInstruction) -> bool {
        match instr {
            crate::middle::ir::IRInstruction::LoadConst(crate::middle::ir::IRConstValue::Int(0)) => true,
            crate::middle::ir::IRInstruction::LoadConst(crate::middle::ir::IRConstValue::Float(f)) if *f == 0.0 => true,
            _ => false,
        }
    }

    fn is_mutable_constructor(&self, name: &str) -> bool {
        matches!(name, "list" | "dict" | "set" | "bytearray")
    }

    fn pow_may_overflow(base: i64, exp: i64) -> bool {
        if exp < 0 || base == 0 || base == 1 {
            return false;
        }
        let abs_base = base.unsigned_abs();
        // Heuristic: if base >= 2 and exp >= 64, the result is astronomically large
        if abs_base >= 2 && exp >= 64 {
            return true;
        }
        // For larger bases, even smaller exponents can overflow i64
        if abs_base >= 10 && exp >= 19 {
            return true;
        }
        false
    }

    /// Get all reports
    pub fn reports(&self) -> &[UBReport] {
        &self.reports
    }

    /// Check if any errors (not just warnings) were found
    pub fn has_errors(&self) -> bool {
        self.reports.iter().any(|r| r.severity == UBSeverity::Error)
    }

    /// PyDead-BIB ESTRICTO: Verificar y bloquear compilación si hay UB
    /// Retorna Err con mensaje de error si hay UB detectado
    pub fn verify_no_ub(&self) -> Result<(), String> {
        if !self.strict_mode {
            return Ok(());
        }

        let errors: Vec<&UBReport> = self.reports
            .iter()
            .filter(|r| r.severity == UBSeverity::Error)
            .collect();

        if errors.is_empty() {
            return Ok(());
        }

        let mut msg = String::from("\n╔═══════════════════════════════════════════════════════════════╗\n");
        msg.push_str("║  PyDead-BIB: COMPILACIÓN BLOQUEADA — UB DETECTADO                             ║\n");
        msg.push_str("║  PyDead-BIB es implícitamente ESTRICTO: UB NO EXISTE                          ║\n");
        msg.push_str("╚═══════════════════════════════════════════════════════════════════════════════╝\n\n");

        for (i, err) in errors.iter().enumerate() {
            msg.push_str(&format!("Error #{}: {:?}\n", i + 1, err.kind));
            msg.push_str(&format!("  Archivo: {}:{}\n", err.file, err.line));
            msg.push_str(&format!("  Mensaje: {}\n", err.message));
            if let Some(ref suggestion) = err.suggestion {
                msg.push_str(&format!("  Sugerencia: {}\n", suggestion));
            }
            msg.push('\n');
        }

        msg.push_str("Corrija los errores anteriores para compilar.\n");
        msg.push_str("PyDead-BIB NO permite comportamiento indefinido.\n");

        Err(msg)
    }

    /// Verificar programa completo y bloquear si hay UB
    pub fn verify_program(&mut self, program: &crate::frontend::python::to_ir::IRProgram) -> Result<(), String> {
        self.analyze(program);
        self.verify_no_ub()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ub_kinds() {
        let ub = PythonUB::DivisionByZero;
        assert_eq!(ub, PythonUB::DivisionByZero);
    }

    #[test]
    fn test_detector_creation() {
        let detector = PyUBDetector::new().with_file("test.py".to_string());
        assert!(!detector.has_errors());
    }

    // Helper to build an IRFunction with a given body and return type
    fn make_func(name: &str, return_type: crate::middle::ir::IRType, body: Vec<crate::middle::ir::IRInstruction>) -> crate::middle::ir::IRFunction {
        crate::middle::ir::IRFunction {
            name: name.to_string(),
            params: vec![],
            return_type,
            body,
        }
    }

    #[test]
    fn test_division_by_zero() {
        use crate::middle::ir::{IRInstruction, IROp, IRConstValue, IRType};
        let func = make_func("div_zero", IRType::I64, vec![
            IRInstruction::BinOp {
                op: IROp::Div,
                left: Box::new(IRInstruction::LoadConst(IRConstValue::Int(10))),
                right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(0))),
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(det.has_errors());
        assert_eq!(det.reports()[0].kind, PythonUB::DivisionByZero);
    }

    #[test]
    fn test_type_mismatch_str_plus_int() {
        use crate::middle::ir::{IRInstruction, IROp, IRConstValue, IRType};
        let func = make_func("str_add", IRType::Void, vec![
            IRInstruction::BinOp {
                op: IROp::Add,
                left: Box::new(IRInstruction::LoadString("hello".to_string())),
                right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(42))),
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(det.has_errors());
        assert_eq!(det.reports()[0].kind, PythonUB::TypeMismatch);
    }

    #[test]
    fn test_type_mismatch_int_plus_str() {
        use crate::middle::ir::{IRInstruction, IROp, IRConstValue, IRType};
        let func = make_func("int_add_str", IRType::Void, vec![
            IRInstruction::BinOp {
                op: IROp::Add,
                left: Box::new(IRInstruction::LoadConst(IRConstValue::Int(1))),
                right: Box::new(IRInstruction::LoadString("world".to_string())),
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(det.has_errors());
        assert_eq!(det.reports()[0].kind, PythonUB::TypeMismatch);
    }

    #[test]
    fn test_no_type_mismatch_int_plus_int() {
        use crate::middle::ir::{IRInstruction, IROp, IRConstValue, IRType};
        let func = make_func("int_add", IRType::Void, vec![
            IRInstruction::BinOp {
                op: IROp::Add,
                left: Box::new(IRInstruction::LoadConst(IRConstValue::Int(1))),
                right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(2))),
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(!det.has_errors());
    }

    #[test]
    fn test_integer_overflow_large_pow() {
        use crate::middle::ir::{IRInstruction, IROp, IRConstValue, IRType};
        let func = make_func("big_pow", IRType::I64, vec![
            IRInstruction::BinOp {
                op: IROp::Pow,
                left: Box::new(IRInstruction::LoadConst(IRConstValue::Int(2))),
                right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(64))),
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(!det.reports().is_empty());
        assert_eq!(det.reports()[0].kind, PythonUB::IntegerOverflow);
        assert_eq!(det.reports()[0].severity, UBSeverity::Warning);
    }

    #[test]
    fn test_no_overflow_small_pow() {
        use crate::middle::ir::{IRInstruction, IROp, IRConstValue, IRType};
        let func = make_func("small_pow", IRType::I64, vec![
            IRInstruction::BinOp {
                op: IROp::Pow,
                left: Box::new(IRInstruction::LoadConst(IRConstValue::Int(2))),
                right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(10))),
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(det.reports().is_empty());
    }

    #[test]
    fn test_mutable_default_arg() {
        use crate::middle::ir::{IRInstruction, IRType};
        let func = make_func("mut_default", IRType::Void, vec![
            IRInstruction::Call {
                func: "list".to_string(),
                args: vec![],
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(!det.reports().is_empty());
        assert_eq!(det.reports()[0].kind, PythonUB::MutableDefaultArg);
    }

    #[test]
    fn test_mutable_default_arg_dict() {
        use crate::middle::ir::{IRInstruction, IRType};
        let func = make_func("mut_dict", IRType::Void, vec![
            IRInstruction::Call {
                func: "dict".to_string(),
                args: vec![],
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(!det.reports().is_empty());
        assert_eq!(det.reports()[0].kind, PythonUB::MutableDefaultArg);
    }

    #[test]
    fn test_no_mutable_default_for_normal_call() {
        use crate::middle::ir::{IRInstruction, IRType};
        let func = make_func("normal_call", IRType::Void, vec![
            IRInstruction::Call {
                func: "print".to_string(),
                args: vec![],
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(det.reports().is_empty());
    }

    #[test]
    fn test_none_deref_in_call_args() {
        use crate::middle::ir::{IRInstruction, IRConstValue, IRType};
        let func = make_func("none_arg", IRType::Void, vec![
            IRInstruction::Call {
                func: "process".to_string(),
                args: vec![IRInstruction::LoadConst(IRConstValue::None)],
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(det.has_errors());
        assert_eq!(det.reports()[0].kind, PythonUB::NoneDeref);
    }

    #[test]
    fn test_none_deref_load_then_call() {
        use crate::middle::ir::{IRInstruction, IRConstValue, IRType};
        let func = make_func("none_call", IRType::Void, vec![
            IRInstruction::LoadConst(IRConstValue::None),
            IRInstruction::Call {
                func: "method".to_string(),
                args: vec![],
            },
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(det.has_errors());
        assert!(det.reports().iter().any(|r| r.kind == PythonUB::NoneDeref));
    }

    #[test]
    fn test_empty_return_in_nonvoid_function() {
        use crate::middle::ir::{IRInstruction, IRType};
        let func = make_func("bad_return", IRType::I64, vec![
            IRInstruction::ReturnVoid,
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(!det.reports().is_empty());
        let report = &det.reports()[0];
        assert_eq!(report.kind, PythonUB::TypeMismatch);
        assert_eq!(report.severity, UBSeverity::Warning);
        assert!(report.message.contains("Empty return"));
    }

    #[test]
    fn test_return_void_in_void_function_ok() {
        use crate::middle::ir::{IRInstruction, IRType};
        let func = make_func("void_func", IRType::Void, vec![
            IRInstruction::ReturnVoid,
        ]);
        let mut det = PyUBDetector::new().with_file("test.py".to_string());
        det.check_function(&func);
        assert!(det.reports().is_empty());
    }
}
