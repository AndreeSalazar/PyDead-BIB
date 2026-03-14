// ============================================================
// Python Preprocessor for PyDead-BIB
// ============================================================
// Encoding detection, __future__ handling, decorator expansion
// Line normalization, trailing whitespace, stats collection
// fastos.bib cache integration
// ============================================================

/// Preprocessing statistics collected during source processing
#[derive(Debug, Clone, Default)]
pub struct PreprocessStats {
    pub line_count: usize,
    pub import_count: usize,
    pub function_count: usize,
    pub class_count: usize,
    pub has_print_statements: bool,
}

/// Python source preprocessor
pub struct PyPreprocessor {
    encoding: String,
    future_annotations: bool,
    future_features: Vec<String>,
    stats: PreprocessStats,
}

impl PyPreprocessor {
    pub fn new() -> Self {
        Self {
            encoding: "utf-8".to_string(),
            future_annotations: false,
            future_features: Vec::new(),
            stats: PreprocessStats::default(),
        }
    }

    /// Process Python source — detect encoding, handle __future__,
    /// normalize line endings, strip trailing whitespace, collect stats
    pub fn process(&mut self, source: &str) -> String {
        // Reset state for each process call
        self.future_features.clear();
        self.future_annotations = false;
        self.stats = PreprocessStats::default();

        // Normalize \r\n → \n
        let normalized = source.replace("\r\n", "\n");

        let mut result = String::new();
        let mut lines = normalized.lines().peekable();

        // Check for shebang (must be first line)
        if let Some(first_line) = lines.peek() {
            if first_line.starts_with("#!") {
                lines.next();
            }
        }

        // Check encoding declaration (PEP 263) — can be line 1 or 2
        // # -*- coding: utf-8 -*-
        if let Some(line) = lines.peek() {
            if line.contains("coding") {
                // Encoding line — skip in output, we always use UTF-8
                lines.next();
            }
        }

        for line in lines {
            let trimmed = line.trim();
            self.stats.line_count += 1;

            // Handle __future__ imports (compile-time directives)
            if trimmed.starts_with("from __future__") {
                self.extract_future_features(trimmed);
                // Process but don't emit — these are compile-time only
                continue;
            }

            // Count imports
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                self.stats.import_count += 1;
            }

            // Count function definitions
            if trimmed.starts_with("def ") {
                self.stats.function_count += 1;
            }

            // Count class definitions
            if trimmed.starts_with("class ") {
                self.stats.class_count += 1;
            }

            // Detect Python 2 print statements (bare `print` followed by a space and no paren)
            if Self::is_print_statement(trimmed) {
                self.stats.has_print_statements = true;
            }

            // Strip trailing whitespace
            let stripped = line.trim_end();
            result.push_str(stripped);
            result.push('\n');
        }

        result
    }

    /// Extract feature names from a `from __future__ import ...` line
    fn extract_future_features(&mut self, line: &str) {
        // from __future__ import annotations, division
        if let Some(imports_part) = line.strip_prefix("from __future__ import") {
            for feature in imports_part.split(',') {
                let feature = feature.trim();
                if !feature.is_empty() {
                    if feature == "annotations" {
                        self.future_annotations = true;
                    }
                    self.future_features.push(feature.to_string());
                }
            }
        }
    }

    /// Detect Python 2 style print statements: `print "hello"` or `print 'hello'`
    /// but not `print(...)` or `print_something`
    fn is_print_statement(line: &str) -> bool {
        if !line.starts_with("print") {
            return false;
        }
        let rest = &line[5..];
        if rest.is_empty() {
            return false;
        }
        let first_char = rest.chars().next().unwrap();
        // `print(` is a function call, `print_` or `print1` is an identifier
        first_char == ' ' && !rest.trim_start().starts_with('(')
    }

    /// Get detected encoding
    pub fn encoding(&self) -> &str {
        &self.encoding
    }

    /// Get the list of __future__ features enabled in the processed source
    pub fn future_features(&self) -> &[String] {
        &self.future_features
    }

    /// Get preprocessing statistics from the last `process()` call
    pub fn stats(&self) -> &PreprocessStats {
        &self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_preprocessing() {
        let mut pp = PyPreprocessor::new();
        let result = pp.process("x = 42\n");
        assert!(result.contains("x = 42"));
    }

    #[test]
    fn test_future_removal() {
        let mut pp = PyPreprocessor::new();
        let result = pp.process("from __future__ import annotations\nx = 42\n");
        assert!(!result.contains("__future__"));
        assert!(result.contains("x = 42"));
    }

    #[test]
    fn test_shebang_removal() {
        let mut pp = PyPreprocessor::new();
        let result = pp.process("#!/usr/bin/env python3\nx = 42\n");
        assert!(!result.contains("#!/usr/bin"));
        assert!(result.contains("x = 42"));
    }

    // ---- New tests ----

    #[test]
    fn test_future_features_tracked() {
        let mut pp = PyPreprocessor::new();
        pp.process("from __future__ import annotations, division\nx = 1\n");
        let features = pp.future_features();
        assert!(features.contains(&"annotations".to_string()));
        assert!(features.contains(&"division".to_string()));
        assert_eq!(features.len(), 2);
        assert!(pp.future_annotations);
    }

    #[test]
    fn test_future_print_function() {
        let mut pp = PyPreprocessor::new();
        pp.process("from __future__ import print_function\nprint('hi')\n");
        assert!(pp.future_features().contains(&"print_function".to_string()));
        assert!(!pp.future_annotations);
    }

    #[test]
    fn test_crlf_normalization() {
        let mut pp = PyPreprocessor::new();
        let result = pp.process("x = 1\r\ny = 2\r\n");
        assert!(!result.contains("\r"));
        assert!(result.contains("x = 1\n"));
        assert!(result.contains("y = 2\n"));
    }

    #[test]
    fn test_trailing_whitespace_stripped() {
        let mut pp = PyPreprocessor::new();
        let result = pp.process("x = 1   \ny = 2\t\n");
        assert!(result.contains("x = 1\n"));
        assert!(result.contains("y = 2\n"));
        assert!(!result.contains("   \n"));
        assert!(!result.contains("\t\n"));
    }

    #[test]
    fn test_print_statement_detection() {
        let mut pp = PyPreprocessor::new();
        pp.process("print \"hello\"\n");
        assert!(pp.stats().has_print_statements);
    }

    #[test]
    fn test_print_function_not_flagged() {
        let mut pp = PyPreprocessor::new();
        pp.process("print(\"hello\")\n");
        assert!(!pp.stats().has_print_statements);
    }

    #[test]
    fn test_print_identifier_not_flagged() {
        let mut pp = PyPreprocessor::new();
        pp.process("print_result = 42\n");
        assert!(!pp.stats().has_print_statements);
    }

    #[test]
    fn test_stats_line_count() {
        let mut pp = PyPreprocessor::new();
        pp.process("a = 1\nb = 2\nc = 3\n");
        assert_eq!(pp.stats().line_count, 3);
    }

    #[test]
    fn test_stats_import_count() {
        let mut pp = PyPreprocessor::new();
        pp.process("import os\nfrom sys import argv\nx = 1\n");
        assert_eq!(pp.stats().import_count, 2);
    }

    #[test]
    fn test_stats_function_count() {
        let mut pp = PyPreprocessor::new();
        pp.process("def foo():\n    pass\ndef bar():\n    pass\n");
        assert_eq!(pp.stats().function_count, 2);
    }

    #[test]
    fn test_stats_class_count() {
        let mut pp = PyPreprocessor::new();
        pp.process("class Foo:\n    pass\nclass Bar:\n    pass\n");
        assert_eq!(pp.stats().class_count, 2);
    }

    #[test]
    fn test_encoding_removal() {
        let mut pp = PyPreprocessor::new();
        let result = pp.process("# -*- coding: utf-8 -*-\nx = 42\n");
        assert!(!result.contains("coding"));
        assert!(result.contains("x = 42"));
    }

    #[test]
    fn test_stats_reset_between_calls() {
        let mut pp = PyPreprocessor::new();
        pp.process("def foo():\n    pass\n");
        assert_eq!(pp.stats().function_count, 1);
        pp.process("x = 1\n");
        assert_eq!(pp.stats().function_count, 0);
        assert_eq!(pp.stats().line_count, 1);
    }

    #[test]
    fn test_combined_preprocessing() {
        let mut pp = PyPreprocessor::new();
        let source = "#!/usr/bin/env python3\r\n# -*- coding: utf-8 -*-\r\nfrom __future__ import annotations\r\nimport os   \r\ndef main():  \r\n    print \"hello\"  \r\n";
        let result = pp.process(source);
        assert!(!result.contains("\r"));
        assert!(!result.contains("#!/usr/bin"));
        assert!(!result.contains("coding"));
        assert!(!result.contains("__future__"));
        assert!(result.contains("import os\n"));
        assert!(result.contains("def main():\n"));
        assert!(pp.future_features().contains(&"annotations".to_string()));
        assert_eq!(pp.stats().import_count, 1);
        assert_eq!(pp.stats().function_count, 1);
        assert!(pp.stats().has_print_statements);
    }
}
