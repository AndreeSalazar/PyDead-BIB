// ============================================================
// Python Parser for PyDead-BIB
// ============================================================
// Recursive descent parser: PyToken → Python AST
// Supports Python 2.7 → 3.13 grammar
// Pure Rust — no external dependencies
// ============================================================

use super::ast::*;
use super::lexer::PyToken;

pub mod expr;
pub mod stmt;
pub mod helpers;

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

}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::python::lexer::PyLexer;

    #[test]
    fn test_parse_assignment() {
        let mut lexer = PyLexer::new("x = 42\n");
        let tokens = lexer.tokenize();
        let mut parser = PyParser::new(tokens);
        let module = parser.parse().unwrap();
        assert_eq!(module.body.len(), 1);
    }

    #[test]
    fn test_parse_function() {
        let src = "def hello():\n    return 42\n";
        let mut lexer = PyLexer::new(src);
        let tokens = lexer.tokenize();
        let mut parser = PyParser::new(tokens);
        let module = parser.parse().unwrap();
        assert!(matches!(module.body[0], PyStmt::FunctionDef { .. }));
    }

    #[test]
    fn test_parse_if() {
        let src = "if x > 0:\n    y = 1\nelse:\n    y = 0\n";
        let mut lexer = PyLexer::new(src);
        let tokens = lexer.tokenize();
        let mut parser = PyParser::new(tokens);
        let module = parser.parse().unwrap();
        assert!(matches!(module.body[0], PyStmt::If { .. }));
    }

    #[test]
    fn test_parse_class() {
        let src = "class Dog:\n    pass\n";
        let mut lexer = PyLexer::new(src);
        let tokens = lexer.tokenize();
        let mut parser = PyParser::new(tokens);
        let module = parser.parse().unwrap();
        assert!(matches!(module.body[0], PyStmt::ClassDef { .. }));
    }
}
