// ============================================================
// ADead-BIB C++ Preprocessor v2.0
// ============================================================
// Full preprocessor with:
//   - #include resolution (built-in C++ headers)
//   - #define object-like and function-like macros with expansion
//   - #ifdef / #ifndef / #else / #endif conditional compilation
//   - #undef
//   - Predefined macros: __cplusplus, __FILE__, __LINE__, __STDC__
//   - Stringification (#) and token pasting (##)
//   - Nested macro expansion (up to 8 passes)
//
// No GCC. No libstdc++. No libc++. ADead-BIB owns the headers. 💀🦈
// ============================================================

use super::cpp_stdlib;
use std::collections::{HashMap, HashSet};

/// A #define macro: either object-like or function-like
#[derive(Debug, Clone)]
enum CppMacro {
    /// #define NAME value
    Object(String),
    /// #define NAME(a,b) body
    Function { params: Vec<String>, body: String },
}

pub struct CppPreprocessor {
    included: HashSet<String>,
    prologue_injected: bool,
    /// Defined macros
    macros: HashMap<String, CppMacro>,
    /// Current file name (for __FILE__)
    current_file: String,
}

impl CppPreprocessor {
    pub fn new() -> Self {
        let mut macros = HashMap::new();
        // Predefined macros
        macros.insert("__cplusplus".to_string(), CppMacro::Object("201703L".to_string()));
        macros.insert("__STDC__".to_string(), CppMacro::Object("1".to_string()));
        macros.insert("__ADEAD_BIB__".to_string(), CppMacro::Object("1".to_string()));
        macros.insert("NULL".to_string(), CppMacro::Object("((void*)0)".to_string()));
        Self {
            included: HashSet::new(),
            prologue_injected: false,
            macros,
            current_file: "main.cpp".to_string(),
        }
    }

    /// Set the source file name (for __FILE__ macro)
    pub fn set_file(&mut self, name: &str) {
        self.current_file = name.to_string();
    }

    /// Process C++ source code, resolving #include, #define, conditionals
    pub fn process(&mut self, source: &str) -> String {
        // Phase 0: Join backslash-continued lines
        let source = Self::join_continuation_lines(source);

        let mut output = String::new();
        // Conditional compilation stack:
        //   Each entry: (is_active, has_been_true, depth_nesting)
        //   is_active: current branch is being emitted
        //   has_been_true: some branch in this #if/#elif/#else chain was already true
        let mut cond_stack: Vec<(bool, bool)> = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let source_line = i + 1;
            let trimmed = line.trim();

            // Determine if we are currently active (all levels must be active)
            let currently_active = cond_stack.iter().all(|(active, _)| *active);

            // --- Conditional directives are always processed ---
            if trimmed == "#endif"
                || trimmed.starts_with("#endif ")
                || trimmed.starts_with("#endif/")
            {
                cond_stack.pop();
                output.push('\n');
                continue;
            }

            if trimmed.starts_with("#ifdef ")
                || trimmed.starts_with("#ifndef ")
                || trimmed.starts_with("#if ")
            {
                if !currently_active {
                    // Parent is inactive — push inactive entry
                    cond_stack.push((false, true));
                } else if trimmed.starts_with("#ifdef ") {
                    let name = trimmed[7..].trim();
                    let active = self.macros.contains_key(name);
                    cond_stack.push((active, active));
                } else if trimmed.starts_with("#ifndef ") {
                    let name = trimmed[8..].trim();
                    let active = !self.macros.contains_key(name);
                    cond_stack.push((active, active));
                } else {
                    // #if expression
                    let cond = trimmed[4..].trim();
                    let active = self.eval_if_expression(cond);
                    cond_stack.push((active, active));
                }
                output.push('\n');
                continue;
            }

            if trimmed.starts_with("#elif ") || trimmed.starts_with("#elif(") {
                // Compute parents_active before borrowing last_mut
                let parents_active = if cond_stack.len() > 1 {
                    cond_stack[..cond_stack.len()-1].iter().all(|(a, _)| *a)
                } else {
                    true
                };
                let has_been_true = cond_stack.last().map_or(false, |e| e.1);
                if let Some(entry) = cond_stack.last_mut() {
                    if has_been_true {
                        entry.0 = false;
                    } else if parents_active {
                        let cond = if trimmed.starts_with("#elif ") {
                            trimmed[6..].trim()
                        } else {
                            trimmed[5..].trim()
                        };
                        let active = self.eval_if_expression(cond);
                        entry.0 = active;
                        if active {
                            entry.1 = true;
                        }
                    } else {
                        entry.0 = false;
                    }
                }
                output.push('\n');
                continue;
            }

            if trimmed == "#else"
                || trimmed.starts_with("#else ")
                || trimmed.starts_with("#else/")
            {
                let parents_active = if cond_stack.len() > 1 {
                    cond_stack[..cond_stack.len()-1].iter().all(|(a, _)| *a)
                } else {
                    true
                };
                let has_been_true = cond_stack.last().map_or(false, |e| e.1);
                if let Some(entry) = cond_stack.last_mut() {
                    if has_been_true {
                        entry.0 = false;
                    } else {
                        entry.0 = parents_active;
                        entry.1 = true;
                    }
                }
                output.push('\n');
                continue;
            }

            // If not currently active, skip this line
            if !currently_active {
                output.push('\n');
                continue;
            }

            // --- Active code processing ---
            if trimmed.starts_with("#include") {
                if let Some(header_name) = self.extract_include(trimmed) {
                    if self.included.contains(&header_name) {
                        output.push('\n');
                        continue;
                    }
                    self.included.insert(header_name.clone());

                    // Inject common prologue on first include
                    if !self.prologue_injected {
                        self.prologue_injected = true;
                        output.push_str(cpp_stdlib::CPP_COMMON_PROLOGUE);
                        output.push('\n');
                    }

                    // Look up C++ header declarations
                    if let Some(declarations) = cpp_stdlib::get_cpp_header(&header_name) {
                        let expanded = self.process(declarations);
                        output.push_str(&expanded);
                        output.push('\n');
                    } else {
                        output.push('\n');
                    }
                } else {
                    output.push('\n');
                }
            } else if trimmed.starts_with("#define ") || trimmed.starts_with("#define\t") {
                self.parse_define(trimmed);
                output.push('\n');
            } else if trimmed.starts_with("#undef ") {
                let name = trimmed[7..].trim().to_string();
                self.macros.remove(&name);
                output.push('\n');
            } else if trimmed.starts_with('#') {
                // Skip other preprocessor directives: #pragma, #error, #warning, #line
                output.push('\n');
            } else {
                // Expand macros in regular code lines
                let mut expanded = self.expand_macros(line);
                // Handle __LINE__ and __FILE__ after macro expansion
                expanded = expanded.replace("__LINE__", &source_line.to_string());
                expanded = expanded.replace("__FILE__", &format!("\"{}\"", self.current_file));
                output.push_str(&expanded);
                output.push('\n');
            }
        }

        output
    }

    /// Join backslash-continued lines into single logical lines
    fn join_continuation_lines(source: &str) -> String {
        let mut result = String::with_capacity(source.len());
        let mut continuation = String::new();
        for line in source.lines() {
            if line.ends_with('\\') {
                continuation.push_str(&line[..line.len()-1]);
                continuation.push(' ');
            } else if !continuation.is_empty() {
                continuation.push_str(line);
                result.push_str(&continuation);
                result.push('\n');
                continuation.clear();
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
        if !continuation.is_empty() {
            result.push_str(&continuation);
            result.push('\n');
        }
        result
    }

    /// Evaluate a #if / #elif preprocessor expression
    /// Supports: integer literals, defined(X), defined X, !, &&, ||,
    ///           ==, !=, <, >, <=, >=, +, -, *, /, %, parentheses
    fn eval_if_expression(&self, expr: &str) -> bool {
        let expr = expr.trim();
        // Strip trailing comments
        let expr = if let Some(pos) = expr.find("//") {
            expr[..pos].trim()
        } else {
            expr
        };
        self.eval_if_value(expr) != 0
    }

    /// Evaluate a preprocessor expression to an i64 value
    fn eval_if_value(&self, expr: &str) -> i64 {
        let expr = expr.trim();
        if expr.is_empty() {
            return 0;
        }

        // Try to parse as simple integer
        if let Ok(n) = expr.parse::<i64>() {
            return n;
        }
        // Hex literal
        if expr.starts_with("0x") || expr.starts_with("0X") {
            if let Ok(n) = i64::from_str_radix(&expr[2..], 16) {
                return n;
            }
        }
        // Strip L/LL/U suffix from integer literals
        let stripped = expr.trim_end_matches('L').trim_end_matches('l')
            .trim_end_matches('U').trim_end_matches('u');
        if stripped != expr {
            if let Ok(n) = stripped.parse::<i64>() {
                return n;
            }
        }

        // defined(X) or defined X
        if expr.starts_with("defined(") {
            // Find the matching close paren for the one at position 7
            if let Some(close) = self.find_matching_paren(expr, 7) {
                if close == expr.len() - 1 {
                    // Entire expression is defined(X)
                    let name = expr[8..close].trim();
                    return if self.macros.contains_key(name) { 1 } else { 0 };
                }
                // Otherwise there's more after defined(X) — fall through to binary op splitting
            }
        }
        if expr.starts_with("defined ") {
            let rest = expr[8..].trim();
            // Only match if rest is a simple identifier (no operators)
            if rest.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return if self.macros.contains_key(rest) { 1 } else { 0 };
            }
        }

        // Unary ! operator
        if expr.starts_with('!') {
            let inner = expr[1..].trim();
            return if self.eval_if_value(inner) == 0 { 1 } else { 0 };
        }

        // Parenthesized expression
        if expr.starts_with('(') {
            if let Some(close) = self.find_matching_paren(expr, 0) {
                if close == expr.len() - 1 {
                    return self.eval_if_value(&expr[1..close]);
                }
            }
        }

        // Binary operators — split at lowest precedence first
        // Precedence (lowest first): ||, &&, ==, !=, <, >, <=, >=, +, -, *, /, %
        if let Some((l, r)) = self.split_binary_op(expr, "||") {
            return if self.eval_if_value(l) != 0 || self.eval_if_value(r) != 0 { 1 } else { 0 };
        }
        if let Some((l, r)) = self.split_binary_op(expr, "&&") {
            return if self.eval_if_value(l) != 0 && self.eval_if_value(r) != 0 { 1 } else { 0 };
        }
        if let Some((l, r)) = self.split_binary_op(expr, "==") {
            return if self.eval_if_value(l) == self.eval_if_value(r) { 1 } else { 0 };
        }
        if let Some((l, r)) = self.split_binary_op(expr, "!=") {
            return if self.eval_if_value(l) != self.eval_if_value(r) { 1 } else { 0 };
        }
        if let Some((l, r)) = self.split_binary_op(expr, ">=") {
            return if self.eval_if_value(l) >= self.eval_if_value(r) { 1 } else { 0 };
        }
        if let Some((l, r)) = self.split_binary_op(expr, "<=") {
            return if self.eval_if_value(l) <= self.eval_if_value(r) { 1 } else { 0 };
        }
        // > and < must not match >= or <=
        if let Some((l, r)) = self.split_binary_op_exclusive(expr, ">", &[">=", ">>"])  {
            return if self.eval_if_value(l) > self.eval_if_value(r) { 1 } else { 0 };
        }
        if let Some((l, r)) = self.split_binary_op_exclusive(expr, "<", &["<=", "<<"]) {
            return if self.eval_if_value(l) < self.eval_if_value(r) { 1 } else { 0 };
        }
        if let Some((l, r)) = self.split_binary_op(expr, "+") {
            // Avoid matching unary + at start
            if !l.is_empty() {
                return self.eval_if_value(l) + self.eval_if_value(r);
            }
        }
        if let Some((l, r)) = self.split_binary_op(expr, "-") {
            if !l.is_empty() {
                return self.eval_if_value(l) - self.eval_if_value(r);
            }
        }
        if let Some((l, r)) = self.split_binary_op(expr, "*") {
            return self.eval_if_value(l) * self.eval_if_value(r);
        }
        if let Some((l, r)) = self.split_binary_op(expr, "/") {
            let rv = self.eval_if_value(r);
            return if rv != 0 { self.eval_if_value(l) / rv } else { 0 };
        }
        if let Some((l, r)) = self.split_binary_op(expr, "%") {
            let rv = self.eval_if_value(r);
            return if rv != 0 { self.eval_if_value(l) % rv } else { 0 };
        }

        // Identifier — check if it's a macro that expands to a number
        if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
            if let Some(CppMacro::Object(val)) = self.macros.get(expr) {
                return self.eval_if_value(val);
            }
            // Unknown identifier = 0 (standard C preprocessor behavior)
            return 0;
        }

        0
    }

    /// Find the matching closing parenthesis for an opening '(' at `start`
    fn find_matching_paren(&self, expr: &str, start: usize) -> Option<usize> {
        let chars: Vec<char> = expr.chars().collect();
        let mut depth = 0;
        for i in start..chars.len() {
            if chars[i] == '(' { depth += 1; }
            if chars[i] == ')' {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
        }
        None
    }

    /// Split expression at the rightmost occurrence of `op` that is not inside parentheses
    fn split_binary_op<'a>(&self, expr: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
        let chars: Vec<char> = expr.chars().collect();
        let op_chars: Vec<char> = op.chars().collect();
        if chars.len() < op_chars.len() { return None; }
        // Pre-compute depth at each position (depth[i] = paren nesting level to the RIGHT of i)
        // Scan from rightmost char to compute running depth
        let mut depth_at: Vec<i32> = vec![0; chars.len()];
        let mut d = 0i32;
        for i in (0..chars.len()).rev() {
            if chars[i] == ')' { d += 1; }
            depth_at[i] = d;
            if chars[i] == '(' { d -= 1; }
        }
        // Scan right-to-left for the operator at depth 0
        let mut i = chars.len() as isize - op_chars.len() as isize;
        while i >= 0 {
            let idx = i as usize;
            if depth_at[idx] == 0 && idx + op_chars.len() <= chars.len() {
                let slice: String = chars[idx..idx+op_chars.len()].iter().collect();
                if slice == op {
                    let left = expr[..idx].trim();
                    let right = expr[idx+op.len()..].trim();
                    if !left.is_empty() || op == "!" {
                        return Some((left, right));
                    }
                }
            }
            i -= 1;
        }
        None
    }

    /// Split at `op` but not if it's part of any of `exclude` operators
    fn split_binary_op_exclusive<'a>(&self, expr: &'a str, op: &str, exclude: &[&str]) -> Option<(&'a str, &'a str)> {
        let chars: Vec<char> = expr.chars().collect();
        let op_chars: Vec<char> = op.chars().collect();
        if chars.len() < op_chars.len() { return None; }
        // Pre-compute depth at each position
        let mut depth_at: Vec<i32> = vec![0; chars.len()];
        let mut d = 0i32;
        for i in (0..chars.len()).rev() {
            if chars[i] == ')' { d += 1; }
            depth_at[i] = d;
            if chars[i] == '(' { d -= 1; }
        }
        let mut i = chars.len() as isize - op_chars.len() as isize;
        while i >= 0 {
            let idx = i as usize;
            if depth_at[idx] == 0 && idx + op_chars.len() <= chars.len() {
                let slice: String = chars[idx..idx+op_chars.len()].iter().collect();
                if slice == op {
                    let mut is_excluded = false;
                    for ex in exclude {
                        let ex_chars: Vec<char> = ex.chars().collect();
                        if idx + ex_chars.len() <= chars.len() {
                            let ex_slice: String = chars[idx..idx+ex_chars.len()].iter().collect();
                            if ex_slice == *ex {
                                is_excluded = true;
                                break;
                            }
                        }
                    }
                    if !is_excluded {
                        let left = expr[..idx].trim();
                        let right = expr[idx+op.len()..].trim();
                        if !left.is_empty() {
                            return Some((left, right));
                        }
                    }
                }
            }
            i -= 1;
        }
        None
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
                        .filter(|p| !p.is_empty() && *p != "...")
                        .collect();
                    let body = after_name[close + 1..].trim().to_string();
                    self.macros
                        .insert(name.to_string(), CppMacro::Function { params, body });
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
        // Strip trailing // comment
        let value = parts.next().unwrap_or("").trim();
        let value = if let Some(comment_pos) = value.find("//") {
            value[..comment_pos].trim()
        } else {
            value
        };
        self.macros
            .insert(name.to_string(), CppMacro::Object(value.to_string()));
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
                    CppMacro::Object(value) => {
                        result = self.replace_whole_word(&result, name, value);
                    }
                    CppMacro::Function { params, body } => {
                        result = self.expand_function_macro(&result, name, params, body);
                    }
                }
            }
            // Handle ## token pasting
            while result.contains("##") {
                let old = result.clone();
                result = self.handle_token_paste(&result);
                if result == old {
                    break;
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
        // Don't expand inside string literals
        let mut in_string = false;
        let mut in_char = false;
        while i < chars.len() {
            if chars[i] == '"' && !in_char {
                in_string = !in_string;
                result.push(chars[i]);
                i += 1;
                continue;
            }
            if chars[i] == '\'' && !in_string {
                in_char = !in_char;
                result.push(chars[i]);
                i += 1;
                continue;
            }
            if in_string || in_char {
                result.push(chars[i]);
                i += 1;
                continue;
            }
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
                                // Handle # stringification
                                let stringify_pat = format!("#{}", param);
                                if expanded.contains(&stringify_pat) {
                                    let stringified = format!("\"{}\"", args[pi]);
                                    expanded = expanded.replace(&stringify_pat, &stringified);
                                }
                                expanded = self.replace_whole_word(&expanded, param, &args[pi]);
                            }
                        }
                        // Handle ##__VA_ARGS__ — remove leading comma if no variadic args
                        if expanded.contains("##__VA_ARGS__") {
                            // Collect variadic args (everything past named params)
                            if args.len() > params.len() {
                                let va_args: Vec<&str> = args[params.len()..].iter().map(|s| s.as_str()).collect();
                                expanded = expanded.replace("##__VA_ARGS__", &va_args.join(", "));
                            } else {
                                // No variadic args — remove ", ##__VA_ARGS__"
                                expanded = expanded.replace(", ##__VA_ARGS__", "");
                                expanded = expanded.replace("##__VA_ARGS__", "");
                            }
                        }
                        if expanded.contains("__VA_ARGS__") {
                            if args.len() > params.len() {
                                let va_args: Vec<&str> = args[params.len()..].iter().map(|s| s.as_str()).collect();
                                expanded = expanded.replace("__VA_ARGS__", &va_args.join(", "));
                            } else {
                                expanded = expanded.replace("__VA_ARGS__", "");
                            }
                        }
                        // Apply ## token pasting before parenthesization check
                        while expanded.contains("##") {
                            let old_exp = expanded.clone();
                            expanded = self.handle_token_paste(&expanded);
                            if expanded == old_exp { break; }
                        }
                        // Wrap in parentheses for complex expression macros,
                        // but NOT for:
                        //   - statement macros (if, for, while, etc.)
                        //   - simple identifiers/tokens (CONCAT result)
                        //   - void casts: (void)(x)
                        let trimmed_exp = expanded.trim();
                        let is_statement_macro = trimmed_exp.starts_with("if ")
                            || trimmed_exp.starts_with("if(")
                            || trimmed_exp.starts_with("for ")
                            || trimmed_exp.starts_with("for(")
                            || trimmed_exp.starts_with("while ")
                            || trimmed_exp.starts_with("while(")
                            || trimmed_exp.starts_with("do ")
                            || trimmed_exp.starts_with("do{")
                            || trimmed_exp.starts_with("switch ")
                            || trimmed_exp.starts_with("switch(")
                            || trimmed_exp.starts_with("return ")
                            || trimmed_exp.starts_with("return;")
                            || trimmed_exp.starts_with('{');
                        // Simple token: just an identifier, number, or string
                        let is_simple_token = trimmed_exp.chars().all(|c| c.is_alphanumeric() || c == '_')
                            && !trimmed_exp.is_empty();
                        if is_statement_macro || is_simple_token {
                            result.push_str(&expanded);
                        } else {
                            result.push('(');
                            result.push_str(&expanded);
                            result.push(')');
                        }
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

    /// Handle ## token pasting operator
    fn handle_token_paste(&self, text: &str) -> String {
        // Replace "A ## B" → "AB" (removing spaces around ##)
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '#' {
                if let Some(&'#') = chars.peek() {
                    chars.next(); // consume second #
                    // Remove trailing whitespace from result
                    while result.ends_with(' ') || result.ends_with('\t') {
                        result.pop();
                    }
                    // Skip leading whitespace in remaining
                    while let Some(&' ') | Some(&'\t') = chars.peek() {
                        chars.next();
                    }
                    continue;
                }
            }
            result.push(c);
        }
        result
    }

    /// Extract header name from #include directive
    fn extract_include(&self, line: &str) -> Option<String> {
        let after_include = line.strip_prefix("#include")?.trim();

        if after_include.starts_with('<') {
            let end = after_include.find('>')?;
            Some(after_include[1..end].trim().to_string())
        } else if after_include.starts_with('"') {
            let rest = &after_include[1..];
            let end = rest.find('"')?;
            Some(rest[..end].trim().to_string())
        } else {
            None
        }
    }

    pub fn included_headers(&self) -> &HashSet<String> {
        &self.included
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_angle_include() {
        let pp = CppPreprocessor::new();
        assert_eq!(
            pp.extract_include("#include <iostream>"),
            Some("iostream".to_string())
        );
        assert_eq!(
            pp.extract_include("#include <vector>"),
            Some("vector".to_string())
        );
        assert_eq!(
            pp.extract_include("#include <string>"),
            Some("string".to_string())
        );
    }

    #[test]
    fn test_extract_quote_include() {
        let pp = CppPreprocessor::new();
        assert_eq!(
            pp.extract_include("#include \"myheader.h\""),
            Some("myheader.h".to_string())
        );
    }

    #[test]
    fn test_no_double_include() {
        let mut pp = CppPreprocessor::new();
        let source = "#include <iostream>\n#include <iostream>\nint main() { return 0; }\n";
        let result = pp.process(source);
        let count = result.matches("int printf").count();
        assert!(count >= 1, "printf should be declared");
    }

    #[test]
    fn test_preserves_code() {
        let mut pp = CppPreprocessor::new();
        let source = "int main() {\n    return 0;\n}\n";
        let result = pp.process(source);
        assert!(result.contains("int main()"));
        assert!(result.contains("return 0;"));
    }

    #[test]
    fn test_object_macro_expansion() {
        let mut pp = CppPreprocessor::new();
        let source = "#define VERSION 42\nint x = VERSION;\n";
        let result = pp.process(source);
        assert!(result.contains("int x = 42;"), "Object macro should expand: {}", result);
    }

    #[test]
    fn test_function_macro_expansion() {
        let mut pp = CppPreprocessor::new();
        let source = "#define MAX(a, b) ((a) > (b) ? (a) : (b))\nint x = MAX(3, 5);\n";
        let result = pp.process(source);
        assert!(result.contains("((3) > (5) ? (3) : (5))"), "Function macro should expand: {}", result);
    }

    #[test]
    fn test_ifdef_true() {
        let mut pp = CppPreprocessor::new();
        let source = "#ifdef __cplusplus\nint cpp_mode = 1;\n#endif\n";
        let result = pp.process(source);
        assert!(result.contains("int cpp_mode = 1;"), "__cplusplus should be defined: {}", result);
    }

    #[test]
    fn test_ifdef_false() {
        let mut pp = CppPreprocessor::new();
        let source = "#ifdef NONEXISTENT\nint bad = 1;\n#endif\nint good = 1;\n";
        let result = pp.process(source);
        assert!(!result.contains("int bad"), "NONEXISTENT should not be defined");
        assert!(result.contains("int good"), "Code after #endif should be kept");
    }

    #[test]
    fn test_ifndef_else() {
        let mut pp = CppPreprocessor::new();
        let source = "#ifndef NULL\n#define NULL ((void*)0)\n#endif\n";
        let result = pp.process(source);
        // NULL is predefined, so the #ifndef body should be skipped
        assert!(!result.contains("#define"), "Should skip body when macro exists");
    }

    #[test]
    fn test_nested_macro() {
        let mut pp = CppPreprocessor::new();
        let source = "#define MAX(a, b) ((a) > (b) ? (a) : (b))\n#define MIN(a, b) ((a) < (b) ? (a) : (b))\n#define CLAMP(x, lo, hi) MIN(MAX(x, lo), hi)\nint y = CLAMP(25, 0, 20);\n";
        let result = pp.process(source);
        // CLAMP should expand to nested MAX/MIN
        assert!(result.contains("25"), "Should contain value 25: {}", result);
        assert!(!result.contains("CLAMP"), "CLAMP should be expanded: {}", result);
    }

    #[test]
    fn test_stringify() {
        let mut pp = CppPreprocessor::new();
        let source = "#define STR(x) #x\nchar *s = STR(hello);\n";
        let result = pp.process(source);
        assert!(result.contains("\"hello\""), "Should stringify: {}", result);
    }

    #[test]
    fn test_token_paste() {
        let mut pp = CppPreprocessor::new();
        let source = "#define CONCAT(a, b) a ## b\nint myVar = CONCAT(my, Var);\n";
        let result = pp.process(source);
        // After token paste, my ## Var → myVar
        assert!(result.contains("myVar"), "Should paste tokens: {}", result);
    }

    #[test]
    fn test_iostream_injected() {
        let mut pp = CppPreprocessor::new();
        let source = "#include <iostream>\nint main() { return 0; }\n";
        let result = pp.process(source);
        assert!(result.contains("printf"), "iostream should inject printf");
        assert!(result.contains("puts"), "iostream should inject puts");
        assert!(result.contains("size_t"), "prologue should inject size_t");
    }

    #[test]
    fn test_vector_injected() {
        let mut pp = CppPreprocessor::new();
        let source = "#include <vector>\nint main() { return 0; }\n";
        let result = pp.process(source);
        assert!(result.contains("size_t"), "prologue should inject size_t");
    }

    #[test]
    fn test_multiple_headers() {
        let mut pp = CppPreprocessor::new();
        let source =
            "#include <iostream>\n#include <vector>\n#include <string>\nint main() { return 0; }\n";
        let result = pp.process(source);
        assert!(result.contains("printf"), "iostream should inject printf");
        assert!(result.contains("size_t"), "prologue should inject size_t");
        assert!(result.contains("int main()"), "code should be preserved");
    }

    #[test]
    fn test_variadic_macro() {
        let mut pp = CppPreprocessor::new();
        let source = "#define LOG(fmt, ...) printf(fmt, ##__VA_ARGS__)\nLOG(\"hello %s\", \"world\");\n";
        let result = pp.process(source);
        assert!(result.contains("printf"), "Should expand LOG to printf: {}", result);
        assert!(result.contains("\"world\""), "Should include args: {}", result);
    }

    #[test]
    fn test_line_file_macros() {
        let mut pp = CppPreprocessor::new();
        pp.set_file("test.cpp");
        let source = "int line = __LINE__;\nchar *f = __FILE__;\n";
        let result = pp.process(source);
        assert!(result.contains("int line = 1;"), "__LINE__ should expand: {}", result);
        assert!(result.contains("\"test.cpp\""), "__FILE__ should expand: {}", result);
    }

    #[test]
    fn test_elif_basic() {
        let mut pp = CppPreprocessor::new();
        let source = "#define V 2\n#if V == 1\nint x = 1;\n#elif V == 2\nint x = 2;\n#elif V == 3\nint x = 3;\n#else\nint x = 0;\n#endif\n";
        let result = pp.process(source);
        assert!(result.contains("int x = 2;"), "#elif V==2 should match: {}", result);
        assert!(!result.contains("int x = 1;"), "#if V==1 should not match: {}", result);
        assert!(!result.contains("int x = 3;"), "#elif V==3 should not match: {}", result);
        assert!(!result.contains("int x = 0;"), "#else should not match: {}", result);
    }

    #[test]
    fn test_elif_else_fallback() {
        let mut pp = CppPreprocessor::new();
        let source = "#define V 99\n#if V == 1\nint x = 1;\n#elif V == 2\nint x = 2;\n#else\nint x = 0;\n#endif\n";
        let result = pp.process(source);
        assert!(result.contains("int x = 0;"), "#else fallback should match: {}", result);
        assert!(!result.contains("int x = 1;"), "#if should not match: {}", result);
        assert!(!result.contains("int x = 2;"), "#elif should not match: {}", result);
    }

    #[test]
    fn test_if_defined_and() {
        let mut pp = CppPreprocessor::new();
        let source = "#define A 1\n#define B 1\n#if defined(A) && defined(B)\nint both = 1;\n#endif\n";
        let result = pp.process(source);
        assert!(result.contains("int both = 1;"), "defined(A) && defined(B) should be true: {}", result);
    }

    #[test]
    fn test_if_comparison() {
        let mut pp = CppPreprocessor::new();
        let source = "#define LEVEL 5\n#if LEVEL >= 3\nint high = 1;\n#endif\n#if LEVEL < 2\nint low = 1;\n#endif\n";
        let result = pp.process(source);
        assert!(result.contains("int high = 1;"), "LEVEL >= 3 should be true: {}", result);
        assert!(!result.contains("int low = 1;"), "LEVEL < 2 should be false: {}", result);
    }

    #[test]
    fn test_if_arithmetic() {
        let mut pp = CppPreprocessor::new();
        let source = "#if (2 + 3) == 5\nint ok = 1;\n#endif\n";
        let result = pp.process(source);
        assert!(result.contains("int ok = 1;"), "(2+3)==5 should be true: {}", result);
    }

    #[test]
    fn test_if_not_defined() {
        let mut pp = CppPreprocessor::new();
        let source = "#if !defined(NONEXISTENT)\nint ok = 1;\n#endif\n";
        let result = pp.process(source);
        assert!(result.contains("int ok = 1;"), "!defined(NONEXISTENT) should be true: {}", result);
    }

    #[test]
    fn test_backslash_continuation() {
        let mut pp = CppPreprocessor::new();
        let source = "#define SWAP(a, b) \\\n    do { \\\n        int tmp = (a); \\\n        (a) = (b); \\\n        (b) = tmp; \\\n    } while(0)\nSWAP(x, y);\n";
        let result = pp.process(source);
        assert!(result.contains("do {"), "Multi-line macro should expand: {}", result);
        assert!(result.contains("while(0)"), "Multi-line macro should have while(0): {}", result);
    }
}
