// ============================================================
// UB Cache — UB results para fastos.bib
// ============================================================
// Cachea los resultados de UB detection para headers.
// Si el header no cambio, los UB reports del cache se reutilizan.
// Nadie mas cachea el UB analysis. UNICO en el mundo.
// ============================================================

use super::report::{UBKind, UBReport, UBSeverity};

/// UB report serializable para cache
#[derive(Debug, Clone)]
pub struct CachedUBResult {
    pub kind: UBKind,
    pub severity: UBSeverity,
    pub message: String,
    pub function: Option<String>,
    pub line: Option<usize>,
    pub suggestion: Option<String>,
}

impl CachedUBResult {
    /// Convierte un UBReport a formato cacheable
    pub fn from_report(report: &UBReport) -> Self {
        Self {
            kind: report.kind,
            severity: report.severity,
            message: report.message.clone(),
            function: report.function.clone(),
            line: report.line,
            suggestion: report.suggestion.clone(),
        }
    }

    /// Restaura un UBReport desde cache
    pub fn to_report(&self) -> UBReport {
        let mut report = UBReport::new(self.severity, self.kind, self.message.clone());
        if let (Some(ref func), Some(line)) = (&self.function, self.line) {
            report = report.with_location(func.clone(), line);
        }
        if let Some(ref suggestion) = self.suggestion {
            report = report.with_suggestion(suggestion.clone());
        }
        report
    }
}

/// Cache de resultados UB para un conjunto de headers
#[derive(Debug, Clone)]
pub struct UBCache {
    /// Hash del source analizado
    pub source_hash: u64,
    /// Resultados de UB pre-analizados
    pub results: Vec<CachedUBResult>,
}

impl UBCache {
    pub fn new(source_hash: u64) -> Self {
        Self {
            source_hash,
            results: Vec::new(),
        }
    }

    /// Cachea resultados de analisis UB
    pub fn cache_results(&mut self, reports: &[UBReport]) {
        self.results = reports.iter().map(CachedUBResult::from_report).collect();
    }

    /// Restaura resultados desde cache
    pub fn restore_reports(&self) -> Vec<UBReport> {
        self.results.iter().map(|r| r.to_report()).collect()
    }

    /// Retorna true si el cache es valido para el hash dado
    pub fn is_valid_for(&self, hash: u64) -> bool {
        self.source_hash == hash
    }

    /// Numero de UB reports cacheados
    pub fn report_count(&self) -> usize {
        self.results.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ub_cache_creation() {
        let cache = UBCache::new(0x12345678);
        assert_eq!(cache.report_count(), 0);
        assert!(cache.is_valid_for(0x12345678));
        assert!(!cache.is_valid_for(0xDEADBEEF));
    }

    #[test]
    fn test_cache_roundtrip() {
        let mut cache = UBCache::new(0xABCD);
        let reports = vec![UBReport::new(
            UBSeverity::Error,
            UBKind::NullPointerDereference,
            "test null deref".to_string(),
        )
        .with_location("test_fn".to_string(), 42)];
        cache.cache_results(&reports);
        let restored = cache.restore_reports();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].kind, UBKind::NullPointerDereference);
        assert_eq!(restored[0].line, Some(42));
    }
}
