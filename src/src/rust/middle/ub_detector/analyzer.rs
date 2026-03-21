// ============================================================
// UB Analyzer — Análisis general de patrones UB
// ============================================================

use super::report::UBReport;
use crate::ast::Program;

/// Analizador general que coordina todos los sub-analizadores
pub struct UBAnalyzer {
    pub reports: Vec<UBReport>,
}

impl UBAnalyzer {
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) {
        // Los análisis específicos se ejecutan desde UBDetector
        // Este módulo puede extenderse con análisis adicionales
        self.reports.clear();
    }
}

impl Default for UBAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
