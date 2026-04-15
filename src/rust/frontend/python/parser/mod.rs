pub mod expr;
pub mod stmt;
pub mod class_def;
// ============================================================
// Python Parser for PyDead-BIB
// ============================================================
// Recursive descent parser: PyToken → Python AST
// Supports Python 2.7 → 3.13 grammar
// Pure Rust — no external dependencies
// ============================================================

use super::py_ast::*;
use super::py_lexer::PyToken;

pub struct PyParser {
    tokens: Vec<PyToken>,
    pos: usize,
}

impl PyParser {
    pub fn new(tokens: Vec<PyToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse entire module
    pub fn parse(&mut self) -> Result<PyModule, Box<dyn std::error::Error>> {
        let mut body = Vec::new();
        let docstring = self.try_parse_docstring();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }
            let stmt = self.parse_statement()?;
            body.push(stmt);
        }

        Ok(PyModule { body, docstring })
    }

