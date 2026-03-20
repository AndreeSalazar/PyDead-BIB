// ============================================================
// ADead-BIB C Preprocessor
// ============================================================
// Resolves #include directives by injecting built-in headers
// Handles: #include <header.h>, #include "header.h"
// Skips: #define, #ifdef, #ifndef, #endif, #else, #if, #pragma
//
// No GCC. No Clang. ADead-BIB owns the headers. 💀🦈
// ============================================================

use super::c_stdlib;
use std::collections::{HashMap, HashSet};

/// A #define macro: either object-like or function-like
#[derive(Debug, Clone)]
enum Macro {
    /// #define NAME value
    Object(String),
    /// #define NAME(a,b) body
    Function { params: Vec<String>, body: String },
}

pub struct CPreprocessor {
    /// Track included headers to prevent double inclusion
    included: HashSet<String>,
    /// Whether the common prologue has been injected
    prologue_injected: bool,
    /// Defined macros
    macros: HashMap<String, Macro>,
}

impl CPreprocessor {
    pub fn new() -> Self {
        Self {
            included: HashSet::new(),
            prologue_injected: false,
            macros: HashMap::new(),
        }
    }

    /// Process C source code, resolving #include directives
    /// Returns preprocessed source with declarations injected
    pub fn process(&mut self, source: &str) -> String {
        let mut output = String::new();
        let mut skip_mode = false;
        let mut skip_depth: i32 = 0;
        let mut skip_else_ok = false;

        for (i, line) in source.lines().enumerate() {
            let source_line = i + 1;
            let trimmed = line.trim();

            // Handle conditional compilation skip mode
            if skip_mode {
                if trimmed.starts_with("#ifdef")
                    || trimmed.starts_with("#ifndef")
                    || trimmed.starts_with("#if ")
                {
                    skip_depth += 1;
                } else if trimmed == "#endif"
                    || trimmed.starts_with("#endif ")
                    || trimmed.starts_with("#endif/")
                {
                    skip_depth -= 1;
                    if skip_depth <= 0 {
                        skip_mode = false;
                        skip_depth = 0;
                    }
                } else if (trimmed == "#else"
                    || trimmed.starts_with("#else ")
                    || trimmed.starts_with("#else/"))
                    && skip_depth == 1
                    && skip_else_ok
                {
                    skip_mode = false;
                    skip_depth = 0;
                    // But we need to mark that the next #else/#elif should skip
                    skip_else_ok = false;
                }
                output.push('\n');
                continue;
            }

            // Handle #endif when not in skip mode (from #else branch)
            if trimmed == "#endif"
                || trimmed.starts_with("#endif ")
                || trimmed.starts_with("#endif/")
            {
                output.push('\n');
                continue;
            }
            // Handle #else when not skipping (we were in the true branch, now skip)
            if trimmed == "#else" || trimmed.starts_with("#else ") || trimmed.starts_with("#else/")
            {
                skip_mode = true;
                skip_depth = 1;
                skip_else_ok = false;
                output.push('\n');
                continue;
            }

            if trimmed.starts_with("#include") {
                // Extract header name from #include <header.h> or #include "header.h"
                if let Some(header_name) = self.extract_include(trimmed) {
                    // Skip if already included
                    if self.included.contains(&header_name) {
                        output.push('\n'); // keep line count stable
                        continue;
                    }
                    self.included.insert(header_name.clone());

                    // Wait, track if we injected lines
                    let mut lines_injected = false;

                    // Inject common prologue on first include
                    if !self.prologue_injected {
                        self.prologue_injected = true;
                        output.push_str(&format!("# 1 \"<common_prologue>\"\n"));
                        output.push_str(c_stdlib::COMMON_PROLOGUE);
                        output.push('\n');
                        lines_injected = true;
                    }

                    // Look up header declarations
                    if let Some(declarations) = c_stdlib::get_header(&header_name) {
                        output.push_str(&format!("# 1 \"{}\"\n", header_name));
                        output.push_str(declarations);
                        output.push('\n');
                        lines_injected = true;
                    } else {
                        // Unknown header — skip with warning
                        eprintln!("ADead-BIB: unknown header <{}> — skipped", header_name);
                        output.push('\n');
                    }

                    if lines_injected {
                        // Resync to main file line
                        output.push_str(&format!("# {} \"main\"\n", source_line + 1));
                    }
                } else {
                    output.push('\n'); // malformed include
                }
            } else if trimmed.starts_with("#define ") || trimmed.starts_with("#define\t") {
                self.parse_define(trimmed);
                output.push('\n');
            } else if trimmed.starts_with("#undef ") {
                let name = trimmed[7..].trim().to_string();
                self.macros.remove(&name);
                output.push('\n');
            } else if trimmed.starts_with("#ifdef ") {
                let name = trimmed[7..].trim();
                if !self.macros.contains_key(name) {
                    // Skip until #else or #endif
                    skip_mode = true;
                    skip_depth = 1;
                    skip_else_ok = true;
                }
                output.push('\n');
            } else if trimmed.starts_with("#ifndef ") {
                let name = trimmed[8..].trim();
                if self.macros.contains_key(name) {
                    skip_mode = true;
                    skip_depth = 1;
                    skip_else_ok = true;
                }
                output.push('\n');
            } else if trimmed.starts_with("#if ") {
                // Simple: #if 0 → skip, #if 1 → keep, #if DEFINED → check
                let cond = trimmed[4..].trim();
                let active = if cond == "0" {
                    false
                } else if cond == "1" {
                    true
                } else if cond.starts_with("defined(") {
                    let name = cond.trim_start_matches("defined(").trim_end_matches(')');
                    self.macros.contains_key(name)
                } else {
                    self.macros.contains_key(cond)
                };
                if !active {
                    skip_mode = true;
                    skip_depth = 1;
                    skip_else_ok = true;
                }
                output.push('\n');
            } else if trimmed.starts_with('#') {
                // Skip other preprocessor directives: #pragma, #error, #warning, #line, etc.
                output.push('\n');
            } else {
                // Expand macros in regular code lines
                let expanded = self.expand_macros(line);
                output.push_str(&expanded);
                output.push('\n');
            }
        }

        output
    }

    /// Extract header name from #include directive
    /// Handles: #include <stdio.h>, #include "myheader.h", #include <sys/types.h>
    fn extract_include(&self, line: &str) -> Option<String> {
        let after_include = line.strip_prefix("#include")?.trim();

        if after_include.starts_with('<') {
            // Angle bracket include: #include <header.h>
            let end = after_include.find('>')?;
            Some(after_include[1..end].trim().to_string())
        } else if after_include.starts_with('"') {
            // Quote include: #include "header.h"
            let rest = &after_include[1..];
            let end = rest.find('"')?;
            Some(rest[..end].trim().to_string())
        } else {
            None
        }
    }

    /// Parse a #define directive and store the macro
    fn parse_define(&mut self, line: &str) {
        let rest = line.strip_prefix("#define").unwrap().trim();
        if rest.is_empty() {
            return;
        }

        // Check for function-like macro: NAME(params) body
        if let Some(paren_pos) = rest.find('(') {
            let name = rest[..paren_pos].trim();
            // Only function-like if '(' immediately follows name (no space)
            if !name.is_empty() && !name.contains(' ') {
                let after_name = &rest[paren_pos..];
                if let Some(close) = after_name.find(')') {
                    let params_str = &after_name[1..close];
                    let params: Vec<String> = params_str
                        .split(',')
                        .map(|p| p.trim().to_string())
                        .filter(|p| !p.is_empty())
                        .collect();
                    let body = after_name[close + 1..].trim().to_string();
                    self.macros
                        .insert(name.to_string(), Macro::Function { params, body });
                    return;
                }
            }
        }

        // Object-like macro: NAME value
        let mut parts = rest.splitn(2, |c: char| c == ' ' || c == '\t');
        let name = parts.next().unwrap_or("").trim();
        if name.is_empty() {
            return;
        }
        // Check for trailing // comment
        let value = parts.next().unwrap_or("").trim();
        let value = if let Some(comment_pos) = value.find("//") {
            value[..comment_pos].trim()
        } else {
            value
        };
        self.macros
            .insert(name.to_string(), Macro::Object(value.to_string()));
    }

    /// Expand macros in a line of code
    fn expand_macros(&self, line: &str) -> String {
        if self.macros.is_empty() {
            return line.to_string();
        }

        let mut result = line.to_string();
        // Multiple passes to handle nested macros (limit to prevent infinite loops)
        for _ in 0..8 {
            let prev = result.clone();
            for (name, mac) in &self.macros {
                match mac {
                    Macro::Object(value) => {
                        // Replace whole-word occurrences only
                        result = self.replace_whole_word(&result, name, value);
                    }
                    Macro::Function { params, body } => {
                        result = self.expand_function_macro(&result, name, params, body);
                    }
                }
            }
            if result == prev {
                break;
            }
        }
        result
    }

    /// Replace whole-word occurrences of `name` with `value`
    fn replace_whole_word(&self, text: &str, name: &str, value: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let chars: Vec<char> = text.chars().collect();
        let name_chars: Vec<char> = name.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if i + name_chars.len() <= chars.len()
                && &chars[i..i + name_chars.len()] == name_chars.as_slice()
            {
                // Check word boundary before
                let before_ok = i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_';
                // Check word boundary after
                let after_idx = i + name_chars.len();
                let after_ok = after_idx >= chars.len()
                    || !chars[after_idx].is_alphanumeric() && chars[after_idx] != '_';
                if before_ok && after_ok {
                    result.push_str(value);
                    i += name_chars.len();
                    continue;
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        result
    }

    /// Expand function-like macro invocations: NAME(arg1, arg2)
    fn expand_function_macro(
        &self,
        text: &str,
        name: &str,
        params: &[String],
        body: &str,
    ) -> String {
        let mut result = String::new();
        let chars: Vec<char> = text.chars().collect();
        let name_chars: Vec<char> = name.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Look for macro name followed by '('
            if i + name_chars.len() < chars.len()
                && &chars[i..i + name_chars.len()] == name_chars.as_slice()
            {
                let before_ok = i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_';
                let after_idx = i + name_chars.len();
                if before_ok && after_idx < chars.len() && chars[after_idx] == '(' {
                    // Extract arguments
                    if let Some((args, end_pos)) = self.extract_macro_args(&chars, after_idx) {
                        // Substitute parameters in body
                        let mut expanded = body.to_string();
                        for (pi, param) in params.iter().enumerate() {
                            if pi < args.len() {
                                expanded = self.replace_whole_word(&expanded, param, &args[pi]);
                            }
                        }
                        // Wrap in parentheses for safety
                        result.push('(');
                        result.push_str(&expanded);
                        result.push(')');
                        i = end_pos;
                        continue;
                    }
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        result
    }

    /// Extract comma-separated arguments from a function-like macro call
    /// Returns (args, position after closing paren)
    fn extract_macro_args(
        &self,
        chars: &[char],
        open_paren: usize,
    ) -> Option<(Vec<String>, usize)> {
        let mut depth = 0;
        let mut args = Vec::new();
        let mut current = String::new();
        let mut i = open_paren;

        while i < chars.len() {
            let c = chars[i];
            if c == '(' {
                depth += 1;
                if depth > 1 {
                    current.push(c);
                }
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    args.push(current.trim().to_string());
                    return Some((args, i + 1));
                }
                current.push(c);
            } else if c == ',' && depth == 1 {
                args.push(current.trim().to_string());
                current = String::new();
            } else {
                current.push(c);
            }
            i += 1;
        }
        None
    }

    /// Get list of all included headers (for debugging/analysis)
    pub fn included_headers(&self) -> &HashSet<String> {
        &self.included
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_angle_include() {
        let pp = CPreprocessor::new();
        assert_eq!(
            pp.extract_include("#include <stdio.h>"),
            Some("stdio.h".to_string())
        );
        assert_eq!(
            pp.extract_include("#include <sys/types.h>"),
            Some("sys/types.h".to_string())
        );
        assert_eq!(
            pp.extract_include("#include <vulkan/vulkan.h>"),
            Some("vulkan/vulkan.h".to_string())
        );
    }

    #[test]
    fn test_extract_quote_include() {
        let pp = CPreprocessor::new();
        assert_eq!(
            pp.extract_include("#include \"myheader.h\""),
            Some("myheader.h".to_string())
        );
    }

    #[test]
    fn test_no_double_include() {
        let mut pp = CPreprocessor::new();
        let source = "#include <stdio.h>\n#include <stdio.h>\nint main() { return 0; }\n";
        let result = pp.process(source);
        // stdio declarations should appear only once
        let count = result.matches("int printf").count();
        assert_eq!(count, 1, "printf should be declared only once");
    }

    #[test]
    fn test_preserves_code() {
        let mut pp = CPreprocessor::new();
        let source = "int main() {\n    return 0;\n}\n";
        let result = pp.process(source);
        assert!(result.contains("int main()"));
        assert!(result.contains("return 0;"));
    }

    #[test]
    fn test_skips_define() {
        let mut pp = CPreprocessor::new();
        let source = "#define MAX 100\nint x;\n";
        let result = pp.process(source);
        assert!(!result.contains("#define"));
        assert!(result.contains("int x;"));
    }

    #[test]
    fn test_multiple_headers() {
        let mut pp = CPreprocessor::new();
        let source = "#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\nint main() { return 0; }\n";
        let result = pp.process(source);
        // Should contain declarations from all three headers
        assert!(result.contains("printf"));
        assert!(result.contains("malloc"));
        assert!(result.contains("strlen"));
    }
}
