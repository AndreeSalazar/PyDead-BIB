// ============================================================
// UB Report — Sistema de reportes de Undefined Behavior
// ============================================================

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UBSeverity {
    Info,    // Advertencia informativa
    Warning, // Posible UB, no bloquea compilación
    Error,   // UB confirmado, bloquea compilación
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UBKind {
    NullPointerDereference,
    UseAfterFree,
    DoubleFree,
    ArrayOutOfBounds,
    IntegerOverflow,
    IntegerUnderflow,
    DivisionByZero,
    UninitializedVariable,
    TypeConfusion,
    StackOverflow,
    DataRace,
    InvalidCast,
    DanglingPointer,
    ShiftOverflow,
    StrictAliasingViolation,
    UnsequencedModification,
    SignedOverflowPromotion,
    ReturnLocalAddress,
    FormatStringMismatch,
    BufferOverflow,
    AlignmentViolation,
}

impl fmt::Display for UBKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UBKind::NullPointerDereference => write!(f, "Null Pointer Dereference"),
            UBKind::UseAfterFree => write!(f, "Use After Free"),
            UBKind::DoubleFree => write!(f, "Double Free"),
            UBKind::ArrayOutOfBounds => write!(f, "Array Out of Bounds"),
            UBKind::IntegerOverflow => write!(f, "Integer Overflow"),
            UBKind::IntegerUnderflow => write!(f, "Integer Underflow"),
            UBKind::DivisionByZero => write!(f, "Division by Zero"),
            UBKind::UninitializedVariable => write!(f, "Uninitialized Variable"),
            UBKind::TypeConfusion => write!(f, "Type Confusion"),
            UBKind::StackOverflow => write!(f, "Stack Overflow"),
            UBKind::DataRace => write!(f, "Data Race"),
            UBKind::InvalidCast => write!(f, "Invalid Cast"),
            UBKind::DanglingPointer => write!(f, "Dangling Pointer"),
            UBKind::ShiftOverflow => write!(f, "Shift Overflow"),
            UBKind::StrictAliasingViolation => write!(f, "Strict Aliasing Violation"),
            UBKind::UnsequencedModification => write!(f, "Unsequenced Modification"),
            UBKind::SignedOverflowPromotion => write!(f, "Signed Overflow Promotion"),
            UBKind::ReturnLocalAddress => write!(f, "Return Local Address"),
            UBKind::FormatStringMismatch => write!(f, "Format String Mismatch"),
            UBKind::BufferOverflow => write!(f, "Buffer Overflow"),
            UBKind::AlignmentViolation => write!(f, "Alignment Violation"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UBReport {
    pub severity: UBSeverity,
    pub kind: UBKind,
    pub message: String,
    pub function: Option<String>,
    pub file_path: Option<String>,
    pub line: Option<usize>,
    pub suggestion: Option<String>,
}

impl UBReport {
    pub fn new(severity: UBSeverity, kind: UBKind, message: String) -> Self {
        Self {
            severity,
            kind,
            message,
            function: None,
            file_path: None,
            line: None,
            suggestion: None,
        }
    }

    pub fn with_location(mut self, function: String, line: usize) -> Self {
        self.function = Some(function);
        self.line = Some(line);
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_file(mut self, file_path: String) -> Self {
        self.file_path = Some(file_path);
        self
    }

    pub fn print(&self) {
        let severity_str = match self.severity {
            UBSeverity::Error => "\x1b[31mERROR\x1b[0m",  // Rojo
            UBSeverity::Warning => "\x1b[33mWARN\x1b[0m", // Amarillo
            UBSeverity::Info => "\x1b[36mINFO\x1b[0m",    // Cyan
        };

        let file_loc = if let Some(path) = &self.file_path {
            if let Some(line) = self.line {
                if line > 0 {
                    format!("{}:{}: ", path, line)
                } else {
                    format!("{}: ", path)
                }
            } else {
                format!("{}: ", path)
            }
        } else {
            String::new()
        };

        let location = if let (Some(func), Some(line)) = (&self.function, self.line) {
            format!(" in {}:{}", func, line)
        } else if let Some(func) = &self.function {
            format!(" in {}", func)
        } else {
            String::new()
        };

        println!(
            "{}[{}] {}: {}{}",
            file_loc, severity_str, self.kind, self.message, location
        );

        if let Some(suggestion) = &self.suggestion {
            println!("  → Suggestion: {}", suggestion);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ub_report_creation() {
        let report = UBReport::new(
            UBSeverity::Error,
            UBKind::NullPointerDereference,
            "Dereferencing null pointer".to_string(),
        );
        assert_eq!(report.severity, UBSeverity::Error);
        assert_eq!(report.kind, UBKind::NullPointerDereference);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(UBSeverity::Error > UBSeverity::Warning);
        assert!(UBSeverity::Warning > UBSeverity::Info);
    }
}
