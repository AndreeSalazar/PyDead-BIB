use super::PyParser;
use crate::frontend::python::ast::*;
use crate::frontend::python::lexer::*;

impl PyParser {
    // ── Helpers ──────────────────────────────────────────

    pub fn peek(&self) -> Option<&PyToken> {
        self.tokens.get(self.pos)
    }

    pub fn advance_tok(&mut self) -> Option<&PyToken> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    pub fn check(&self, expected: &PyToken) -> bool {
        self.peek().map_or(false, |t| std::mem::discriminant(t) == std::mem::discriminant(expected))
    }

    pub fn expect(&mut self, expected: &PyToken) -> Result<(), Box<dyn std::error::Error>> {
        if self.check(expected) {
            self.advance_tok();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?} at position {}", expected, self.peek(), self.pos).into())
        }
    }

    pub fn expect_identifier(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        match self.peek() {
            Some(PyToken::Identifier(name)) => {
                let name = name.clone();
                self.advance_tok();
                Ok(name)
            }
            other => Err(format!("Expected identifier, got {:?} at position {}", other, self.pos).into()),
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek(), Some(PyToken::Eof))
    }

    pub fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(PyToken::Newline) | Some(PyToken::Comment(_))) {
            self.advance_tok();
        }
    }

    pub fn try_parse_docstring(&mut self) -> Option<String> {
        self.skip_newlines();
        if let Some(PyToken::StringLiteral(s)) = self.peek() {
            let doc = s.clone();
            self.advance_tok();
            Some(doc)
        } else {
            std::option::Option::None
        }
    }

    pub fn extract_docstring(&self, body: &[PyStmt]) -> Option<String> {
        if let Some(PyStmt::Expr(PyExpr::StringLiteral(s))) = body.first() {
            Some(s.clone())
        } else {
            std::option::Option::None
        }
    }

    pub fn check_aug_assign(&self) -> Option<PyBinOp> {
        match self.peek() {
            Some(PyToken::PlusAssign) => Some(PyBinOp::Add),
            Some(PyToken::MinusAssign) => Some(PyBinOp::Sub),
            Some(PyToken::StarAssign) => Some(PyBinOp::Mul),
            Some(PyToken::SlashAssign) => Some(PyBinOp::Div),
            Some(PyToken::DoubleSlashAssign) => Some(PyBinOp::FloorDiv),
            Some(PyToken::PercentAssign) => Some(PyBinOp::Mod),
            Some(PyToken::DoubleStarAssign) => Some(PyBinOp::Pow),
            Some(PyToken::AmpAssign) => Some(PyBinOp::BitAnd),
            Some(PyToken::PipeAssign) => Some(PyBinOp::BitOr),
            Some(PyToken::CaretAssign) => Some(PyBinOp::BitXor),
            Some(PyToken::LShiftAssign) => Some(PyBinOp::LShift),
            Some(PyToken::RShiftAssign) => Some(PyBinOp::RShift),
            Some(PyToken::AtAssign) => Some(PyBinOp::MatMul),
            _ => std::option::Option::None,
        }
    }
}
