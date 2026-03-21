// ============================================================
// UB_Detector — Undefined Behavior Detection
// ============================================================
// 21 tipos de UB detectados:
// - NullPointerDereference     - UseAfterFree
// - DoubleFree                 - ArrayOutOfBounds
// - IntegerOverflow            - IntegerUnderflow
// - DivisionByZero             - UninitializedVariable
// - TypeConfusion              - StackOverflow
// - DataRace                   - InvalidCast
// - DanglingPointer            - ShiftOverflow
// - StrictAliasingViolation    - UnsequencedModification
// - SignedOverflowPromotion    - ReturnLocalAddress
// - FormatStringMismatch       - BufferOverflow
// - AlignmentViolation
// ============================================================

pub mod analyzer;
pub mod bounds_check;
pub mod cache;
pub mod lifetime;
pub mod null_check;
pub mod overflow_check;
pub mod race_check;
pub mod report;
pub mod type_check;
pub mod uninit_check;
pub mod useafter_check;
pub mod unsequenced_check;

use crate::ast::Program;
pub use report::{UBKind, UBReport, UBSeverity};

/// UB_Detector principal — analiza un programa IR completo
pub struct UBDetector {
    reports: Vec<UBReport>,
    strict_mode: bool,
    file_path: Option<String>,
}

impl UBDetector {
    /// Crea UBDetector con modo estricto (default).
    /// En modo estricto, errores UB bloquean compilacion.
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
            strict_mode: true, // Default: modo estricto — se detiene en UB
            file_path: None,
        }
    }

    /// Desactiva modo estricto (--warn-ub): avisa y continua.
    /// Tu responsabilidad.
    pub fn with_warn_mode(mut self) -> Self {
        self.strict_mode = false;
        self
    }

    pub fn with_strict_mode(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    pub fn with_file(mut self, file_path: String) -> Self {
        self.file_path = Some(file_path);
        self
    }

    /// Analiza el programa IR y retorna reportes de UB
    pub fn analyze(&mut self, program: &Program) -> Vec<UBReport> {
        self.reports.clear();

        // 1. Análisis de null pointer dereference
        let null_reports = null_check::analyze_null_safety(program);
        self.reports.extend(null_reports);

        // 2. Análisis de array bounds
        let bounds_reports = bounds_check::analyze_bounds(program);
        self.reports.extend(bounds_reports);

        // 3. Análisis de integer overflow
        let overflow_reports = overflow_check::analyze_overflow(program);
        self.reports.extend(overflow_reports);

        // 4. Análisis de lifetime (use-after-free)
        let lifetime_reports = lifetime::analyze_lifetimes(program);
        self.reports.extend(lifetime_reports);

        // 5. Análisis de variables no inicializadas
        let uninit_reports = uninit_check::analyze_uninitialized(program);
        self.reports.extend(uninit_reports);

        // 6. Análisis de use-after-free y dangling pointers
        let useafter_reports = useafter_check::analyze_use_after_free(program);
        self.reports.extend(useafter_reports);

        // 7. Análisis de type confusion e invalid casts
        let type_reports = type_check::analyze_type_safety(program);
        self.reports.extend(type_reports);

        // 8. Análisis de data races y stack overflow
        let race_reports = race_check::analyze_concurrency(program);
        self.reports.extend(race_reports);

        // 9. Unsequenced modifications
        let unseq_reports = unsequenced_check::analyze_unsequenced(program);
        self.reports.extend(unseq_reports);

        // Ordenar por severidad (Error > Warning > Info)
        self.reports.sort_by(|a, b| b.severity.cmp(&a.severity));

        // Inject file path
        if let Some(path) = &self.file_path {
            for r in &mut self.reports {
                r.file_path = Some(path.clone());
            }
        }

        self.reports.clone()
    }

    /// Retorna true si hay errores críticos (bloquean compilación)
    pub fn has_errors(&self) -> bool {
        self.reports.iter().any(|r| r.severity == UBSeverity::Error)
    }

    /// Imprime todos los reportes
    pub fn print_reports(&self) {
        if self.reports.is_empty() {
            println!("✓ No undefined behavior detected");
            return;
        }

        println!("\n=== UB_Detector Report ===");
        for report in &self.reports {
            report.print();
        }
        println!("=========================\n");
    }
}

impl Default for UBDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ub_detector_creation() {
        let detector = UBDetector::new();
        assert_eq!(detector.reports.len(), 0);
        assert!(detector.strict_mode); // Estricto por default
    }

    #[test]
    fn test_warn_mode() {
        let detector = UBDetector::new().with_warn_mode();
        assert!(!detector.strict_mode); // --warn-ub desactiva estricto
    }
}
