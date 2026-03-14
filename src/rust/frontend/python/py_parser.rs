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

    // ── Statement parsing ────────────────────────────────

    fn parse_statement(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.skip_newlines();

        match self.peek() {
            Some(PyToken::Def) | Some(PyToken::Async) => self.parse_function_def(),
            Some(PyToken::Class) => self.parse_class_def(),
            Some(PyToken::If) => self.parse_if(),
            Some(PyToken::While) => self.parse_while(),
            Some(PyToken::For) => self.parse_for(),
            Some(PyToken::Try) => self.parse_try(),
            Some(PyToken::With) => self.parse_with(),
            Some(PyToken::Return) => self.parse_return(),
            Some(PyToken::Break) => { self.advance_tok(); Ok(PyStmt::Break) }
            Some(PyToken::Continue) => { self.advance_tok(); Ok(PyStmt::Continue) }
            Some(PyToken::Pass) => { self.advance_tok(); Ok(PyStmt::Pass) }
            Some(PyToken::Import) => self.parse_import(),
            Some(PyToken::From) => self.parse_import_from(),
            Some(PyToken::Raise) => self.parse_raise(),
            Some(PyToken::Assert) => self.parse_assert(),
            Some(PyToken::Del) => self.parse_del(),
            Some(PyToken::Global) => self.parse_global(),
            Some(PyToken::Nonlocal) => self.parse_nonlocal(),
            Some(PyToken::Match) => self.parse_match(),
            Some(PyToken::Decorator(_)) => self.parse_decorated(),
            _ => self.parse_expr_or_assign(),
        }
    }

    fn parse_function_def(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        let is_async = self.check(&PyToken::Async);
        if is_async { self.advance_tok(); }
        self.expect(&PyToken::Def)?;

        let name = self.expect_identifier()?;
        self.expect(&PyToken::LParen)?;
        let params = self.parse_params()?;
        self.expect(&PyToken::RParen)?;

        let return_type = if self.check(&PyToken::Arrow) {
            self.advance_tok();
            Some(self.parse_type_annotation()?)
        } else {
            std::option::Option::None
        };

        self.expect(&PyToken::Colon)?;
        let body = self.parse_block()?;
        let docstring = self.extract_docstring(&body);

        Ok(PyStmt::FunctionDef {
            name,
            params,
            body,
            decorators: Vec::new(),
            return_type,
            is_async,
            docstring,
        })
    }

    fn parse_class_def(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Class)?;
        let name = self.expect_identifier()?;

        let bases = if self.check(&PyToken::LParen) {
            self.advance_tok();
            let mut bases = Vec::new();
            while !self.check(&PyToken::RParen) && !self.is_at_end() {
                bases.push(self.parse_expr()?);
                if self.check(&PyToken::Comma) { self.advance_tok(); }
            }
            self.expect(&PyToken::RParen)?;
            bases
        } else {
            Vec::new()
        };

        self.expect(&PyToken::Colon)?;
        let body = self.parse_block()?;
        let docstring = self.extract_docstring(&body);

        Ok(PyStmt::ClassDef {
            name,
            bases,
            body,
            decorators: Vec::new(),
            docstring,
        })
    }

    fn parse_if(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::If)?;
        let test = self.parse_expr()?;
        self.expect(&PyToken::Colon)?;
        let body = self.parse_block()?;

        let mut elif_clauses = Vec::new();
        while self.check(&PyToken::Elif) {
            self.advance_tok();
            let elif_test = self.parse_expr()?;
            self.expect(&PyToken::Colon)?;
            let elif_body = self.parse_block()?;
            elif_clauses.push((elif_test, elif_body));
        }

        let orelse = if self.check(&PyToken::Else) {
            self.advance_tok();
            self.expect(&PyToken::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(PyStmt::If { test, body, elif_clauses, orelse })
    }

    fn parse_while(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::While)?;
        let test = self.parse_expr()?;
        self.expect(&PyToken::Colon)?;
        let body = self.parse_block()?;

        let orelse = if self.check(&PyToken::Else) {
            self.advance_tok();
            self.expect(&PyToken::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(PyStmt::While { test, body, orelse })
    }

    fn parse_for(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::For)?;
        let target = self.parse_expr()?;
        self.expect(&PyToken::In)?;
        let iter = self.parse_expr()?;
        self.expect(&PyToken::Colon)?;
        let body = self.parse_block()?;

        let orelse = if self.check(&PyToken::Else) {
            self.advance_tok();
            self.expect(&PyToken::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(PyStmt::For {
            target, iter, body, orelse,
            is_async: false,
        })
    }

    fn parse_try(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Try)?;
        self.expect(&PyToken::Colon)?;
        let body = self.parse_block()?;

        let mut handlers = Vec::new();
        while self.check(&PyToken::Except) {
            self.advance_tok();
            let exc_type = if !self.check(&PyToken::Colon) {
                let expr = self.parse_expr()?;
                Some(expr)
            } else {
                std::option::Option::None
            };
            let name = if self.check(&PyToken::As) {
                self.advance_tok();
                Some(self.expect_identifier()?)
            } else {
                std::option::Option::None
            };
            self.expect(&PyToken::Colon)?;
            let handler_body = self.parse_block()?;
            handlers.push(PyExceptHandler {
                exc_type,
                name,
                body: handler_body,
            });
        }

        let orelse = if self.check(&PyToken::Else) {
            self.advance_tok();
            self.expect(&PyToken::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        let finalbody = if self.check(&PyToken::Finally) {
            self.advance_tok();
            self.expect(&PyToken::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(PyStmt::Try { body, handlers, orelse, finalbody })
    }

    fn parse_with(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::With)?;
        let mut items = Vec::new();
        loop {
            let context = self.parse_expr()?;
            let var = if self.check(&PyToken::As) {
                self.advance_tok();
                Some(self.parse_expr()?)
            } else {
                std::option::Option::None
            };
            items.push((context, var));
            if !self.check(&PyToken::Comma) { break; }
            self.advance_tok();
        }
        self.expect(&PyToken::Colon)?;
        let body = self.parse_block()?;

        Ok(PyStmt::With { items, body, is_async: false })
    }

    fn parse_return(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Return)?;
        if self.check(&PyToken::Newline) || self.check(&PyToken::Eof) || self.check(&PyToken::Dedent) {
            return Ok(PyStmt::Return(std::option::Option::None));
        }
        let value = self.parse_expr()?;
        Ok(PyStmt::Return(Some(value)))
    }

    fn parse_import(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Import)?;
        let mut names = Vec::new();
        loop {
            let name = self.expect_identifier()?;
            let mut full_name = name;
            while self.check(&PyToken::Dot) {
                self.advance_tok();
                let part = self.expect_identifier()?;
                full_name = format!("{}.{}", full_name, part);
            }
            let asname = if self.check(&PyToken::As) {
                self.advance_tok();
                Some(self.expect_identifier()?)
            } else {
                std::option::Option::None
            };
            names.push(PyAlias { name: full_name, asname });
            if !self.check(&PyToken::Comma) { break; }
            self.advance_tok();
        }
        Ok(PyStmt::Import { names })
    }

    fn parse_import_from(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::From)?;

        let mut level = 0;
        while self.check(&PyToken::Dot) {
            self.advance_tok();
            level += 1;
        }

        let module = if !self.check(&PyToken::Import) {
            let name = self.expect_identifier()?;
            let mut full = name;
            while self.check(&PyToken::Dot) {
                self.advance_tok();
                let part = self.expect_identifier()?;
                full = format!("{}.{}", full, part);
            }
            Some(full)
        } else {
            std::option::Option::None
        };

        self.expect(&PyToken::Import)?;

        let mut names = Vec::new();
        if self.check(&PyToken::Star) {
            self.advance_tok();
            names.push(PyAlias { name: "*".to_string(), asname: std::option::Option::None });
        } else {
            loop {
                let name = self.expect_identifier()?;
                let asname = if self.check(&PyToken::As) {
                    self.advance_tok();
                    Some(self.expect_identifier()?)
                } else {
                    std::option::Option::None
                };
                names.push(PyAlias { name, asname });
                if !self.check(&PyToken::Comma) { break; }
                self.advance_tok();
            }
        }

        Ok(PyStmt::ImportFrom { module, names, level })
    }

    fn parse_raise(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Raise)?;
        if self.check(&PyToken::Newline) || self.check(&PyToken::Eof) {
            return Ok(PyStmt::Raise { exc: std::option::Option::None, cause: std::option::Option::None });
        }
        let exc = self.parse_expr()?;
        let cause = if self.check(&PyToken::From) {
            self.advance_tok();
            Some(self.parse_expr()?)
        } else {
            std::option::Option::None
        };
        Ok(PyStmt::Raise { exc: Some(exc), cause })
    }

    fn parse_assert(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Assert)?;
        let test = self.parse_expr()?;
        let msg = if self.check(&PyToken::Comma) {
            self.advance_tok();
            Some(self.parse_expr()?)
        } else {
            std::option::Option::None
        };
        Ok(PyStmt::Assert { test, msg })
    }

    fn parse_del(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Del)?;
        let mut targets = Vec::new();
        loop {
            targets.push(self.parse_expr()?);
            if !self.check(&PyToken::Comma) { break; }
            self.advance_tok();
        }
        Ok(PyStmt::Delete(targets))
    }

    fn parse_global(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Global)?;
        let mut names = Vec::new();
        loop {
            names.push(self.expect_identifier()?);
            if !self.check(&PyToken::Comma) { break; }
            self.advance_tok();
        }
        Ok(PyStmt::Global(names))
    }

    fn parse_nonlocal(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Nonlocal)?;
        let mut names = Vec::new();
        loop {
            names.push(self.expect_identifier()?);
            if !self.check(&PyToken::Comma) { break; }
            self.advance_tok();
        }
        Ok(PyStmt::Nonlocal(names))
    }

    fn parse_match(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Match)?;
        let subject = self.parse_expr()?;
        self.expect(&PyToken::Colon)?;
        self.skip_newlines();
        self.expect(&PyToken::Indent)?;

        let mut cases = Vec::new();
        while self.check(&PyToken::Case) {
            self.advance_tok();
            let pattern = self.parse_pattern()?;
            let guard = if self.check(&PyToken::If) {
                self.advance_tok();
                Some(self.parse_expr()?)
            } else {
                std::option::Option::None
            };
            self.expect(&PyToken::Colon)?;
            let body = self.parse_block()?;
            cases.push(PyMatchCase { pattern, guard, body });
            self.skip_newlines();
        }

        if self.check(&PyToken::Dedent) { self.advance_tok(); }

        Ok(PyStmt::Match { subject, cases })
    }

    fn parse_pattern(&mut self) -> Result<PyPattern, Box<dyn std::error::Error>> {
        match self.peek() {
            Some(PyToken::Identifier(s)) if s == "_" => {
                self.advance_tok();
                Ok(PyPattern::Wildcard)
            }
            Some(PyToken::Identifier(_)) => {
                let name = self.expect_identifier()?;
                Ok(PyPattern::Capture(name))
            }
            Some(PyToken::IntLiteral(_)) | Some(PyToken::FloatLiteral(_))
            | Some(PyToken::StringLiteral(_)) | Some(PyToken::True)
            | Some(PyToken::False) | Some(PyToken::None) => {
                let expr = self.parse_expr()?;
                Ok(PyPattern::Literal(expr))
            }
            _ => {
                let expr = self.parse_expr()?;
                Ok(PyPattern::Value(expr))
            }
        }
    }

    fn parse_decorated(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        let mut decorators = Vec::new();
        while let Some(PyToken::Decorator(name)) = self.peek() {
            let dec_name = name.clone();
            self.advance_tok();
            decorators.push(PyExpr::Name(dec_name));
            self.skip_newlines();
        }

        let mut stmt = self.parse_statement()?;
        match &mut stmt {
            PyStmt::FunctionDef { decorators: decs, .. } => { *decs = decorators; }
            PyStmt::ClassDef { decorators: decs, .. } => { *decs = decorators; }
            _ => {}
        }
        Ok(stmt)
    }

    fn parse_expr_or_assign(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        let expr = self.parse_expr()?;

        // Check for assignment
        if self.check(&PyToken::Assign) {
            self.advance_tok();
            let value = self.parse_expr()?;
            return Ok(PyStmt::Assign {
                targets: vec![expr],
                value,
            });
        }

        // Check for augmented assignment
        if let Some(op) = self.check_aug_assign() {
            self.advance_tok();
            let value = self.parse_expr()?;
            return Ok(PyStmt::AugAssign {
                target: expr,
                op,
                value,
            });
        }

        // Check for type annotation
        if self.check(&PyToken::Colon) {
            self.advance_tok();
            let annotation = self.parse_type_annotation()?;
            let value = if self.check(&PyToken::Assign) {
                self.advance_tok();
                Some(self.parse_expr()?)
            } else {
                std::option::Option::None
            };
            return Ok(PyStmt::AnnAssign {
                target: expr,
                annotation,
                value,
            });
        }

        Ok(PyStmt::Expr(expr))
    }

    // ── Expression parsing (precedence climbing) ─────────

    pub fn parse_expr(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let body = self.parse_or()?;
        if self.check(&PyToken::If) {
            self.advance_tok();
            let test = self.parse_or()?;
            self.expect(&PyToken::Else)?;
            let orelse = self.parse_ternary()?;
            return Ok(PyExpr::IfExpr {
                test: Box::new(test),
                body: Box::new(body),
                orelse: Box::new(orelse),
            });
        }
        Ok(body)
    }

    fn parse_or(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut left = self.parse_and()?;
        while self.check(&PyToken::Or) {
            self.advance_tok();
            let right = self.parse_and()?;
            left = PyExpr::BoolOp {
                op: PyBoolOp::Or,
                values: vec![left, right],
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut left = self.parse_not()?;
        while self.check(&PyToken::And) {
            self.advance_tok();
            let right = self.parse_not()?;
            left = PyExpr::BoolOp {
                op: PyBoolOp::And,
                values: vec![left, right],
            };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        if self.check(&PyToken::Not) {
            self.advance_tok();
            let operand = self.parse_not()?;
            return Ok(PyExpr::UnaryOp {
                op: PyUnaryOp::Not,
                operand: Box::new(operand),
            });
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let left = self.parse_bitor()?;

        let mut ops = Vec::new();
        let mut comparators = Vec::new();

        loop {
            let cmp_op = match self.peek() {
                Some(PyToken::EqEq) => Some(PyCmpOp::Eq),
                Some(PyToken::NotEq) => Some(PyCmpOp::NotEq),
                Some(PyToken::Less) => Some(PyCmpOp::Lt),
                Some(PyToken::LessEq) => Some(PyCmpOp::LtE),
                Some(PyToken::Greater) => Some(PyCmpOp::Gt),
                Some(PyToken::GreaterEq) => Some(PyCmpOp::GtE),
                Some(PyToken::Is) => {
                    self.advance_tok();
                    if self.check(&PyToken::Not) {
                        self.advance_tok();
                        comparators.push(self.parse_bitor()?);
                        ops.push(PyCmpOp::IsNot);
                        continue;
                    } else {
                        comparators.push(self.parse_bitor()?);
                        ops.push(PyCmpOp::Is);
                        continue;
                    }
                }
                Some(PyToken::In) => Some(PyCmpOp::In),
                Some(PyToken::Not) => {
                    // "not in"
                    let saved = self.pos;
                    self.advance_tok();
                    if self.check(&PyToken::In) {
                        self.advance_tok();
                        comparators.push(self.parse_bitor()?);
                        ops.push(PyCmpOp::NotIn);
                        continue;
                    } else {
                        self.pos = saved;
                        std::option::Option::None
                    }
                }
                _ => std::option::Option::None,
            };

            if let Some(op) = cmp_op {
                self.advance_tok();
                comparators.push(self.parse_bitor()?);
                ops.push(op);
            } else {
                break;
            }
        }

        if ops.is_empty() {
            Ok(left)
        } else {
            Ok(PyExpr::Compare {
                left: Box::new(left),
                ops,
                comparators,
            })
        }
    }

    fn parse_bitor(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut left = self.parse_bitxor()?;
        while self.check(&PyToken::Pipe) {
            self.advance_tok();
            let right = self.parse_bitxor()?;
            left = PyExpr::BinOp { op: PyBinOp::BitOr, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_bitxor(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut left = self.parse_bitand()?;
        while self.check(&PyToken::Caret) {
            self.advance_tok();
            let right = self.parse_bitand()?;
            left = PyExpr::BinOp { op: PyBinOp::BitXor, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_bitand(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut left = self.parse_shift()?;
        while self.check(&PyToken::Ampersand) {
            self.advance_tok();
            let right = self.parse_shift()?;
            left = PyExpr::BinOp { op: PyBinOp::BitAnd, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Some(PyToken::LShift) => PyBinOp::LShift,
                Some(PyToken::RShift) => PyBinOp::RShift,
                _ => break,
            };
            self.advance_tok();
            let right = self.parse_additive()?;
            left = PyExpr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(PyToken::Plus) => PyBinOp::Add,
                Some(PyToken::Minus) => PyBinOp::Sub,
                _ => break,
            };
            self.advance_tok();
            let right = self.parse_multiplicative()?;
            left = PyExpr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(PyToken::Star) => PyBinOp::Mul,
                Some(PyToken::Slash) => PyBinOp::Div,
                Some(PyToken::DoubleSlash) => PyBinOp::FloorDiv,
                Some(PyToken::Percent) => PyBinOp::Mod,
                Some(PyToken::At) => PyBinOp::MatMul,
                _ => break,
            };
            self.advance_tok();
            let right = self.parse_unary()?;
            left = PyExpr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        match self.peek() {
            Some(PyToken::Plus) => {
                self.advance_tok();
                let operand = self.parse_unary()?;
                Ok(PyExpr::UnaryOp { op: PyUnaryOp::Pos, operand: Box::new(operand) })
            }
            Some(PyToken::Minus) => {
                self.advance_tok();
                let operand = self.parse_unary()?;
                Ok(PyExpr::UnaryOp { op: PyUnaryOp::Neg, operand: Box::new(operand) })
            }
            Some(PyToken::Tilde) => {
                self.advance_tok();
                let operand = self.parse_unary()?;
                Ok(PyExpr::UnaryOp { op: PyUnaryOp::Invert, operand: Box::new(operand) })
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let base = self.parse_postfix()?;
        if self.check(&PyToken::DoubleStar) {
            self.advance_tok();
            let exp = self.parse_unary()?;
            return Ok(PyExpr::BinOp {
                op: PyBinOp::Pow,
                left: Box::new(base),
                right: Box::new(exp),
            });
        }
        Ok(base)
    }

    fn parse_postfix(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        let mut expr = self.parse_atom()?;
        loop {
            match self.peek() {
                Some(PyToken::LParen) => {
                    self.advance_tok();
                    let mut args = Vec::new();
                    let kwargs = Vec::new();
                    while !self.check(&PyToken::RParen) && !self.is_at_end() {
                        self.skip_newlines();
                        args.push(self.parse_expr()?);
                        self.skip_newlines();
                        if self.check(&PyToken::Comma) { self.advance_tok(); }
                    }
                    self.expect(&PyToken::RParen)?;
                    expr = PyExpr::Call {
                        func: Box::new(expr),
                        args,
                        kwargs,
                        starargs: std::option::Option::None,
                        starkwargs: std::option::Option::None,
                    };
                }
                Some(PyToken::LBracket) => {
                    self.advance_tok();
                    let slice = self.parse_expr()?;
                    self.expect(&PyToken::RBracket)?;
                    expr = PyExpr::Subscript {
                        value: Box::new(expr),
                        slice: Box::new(slice),
                    };
                }
                Some(PyToken::Dot) => {
                    self.advance_tok();
                    let attr = self.expect_identifier()?;
                    expr = PyExpr::Attribute {
                        value: Box::new(expr),
                        attr,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        match self.peek() {
            Some(PyToken::IntLiteral(n)) => {
                let val = *n;
                self.advance_tok();
                Ok(PyExpr::IntLiteral(val))
            }
            Some(PyToken::FloatLiteral(f)) => {
                let val = *f;
                self.advance_tok();
                Ok(PyExpr::FloatLiteral(val))
            }
            Some(PyToken::StringLiteral(s)) => {
                let val = s.clone();
                self.advance_tok();
                Ok(PyExpr::StringLiteral(val))
            }
            Some(PyToken::FStringStart(s)) => {
                let val = s.clone();
                self.advance_tok();
                Ok(PyExpr::FString {
                    parts: vec![FStringPart::Literal(val)],
                })
            }
            Some(PyToken::BytesLiteral(b)) => {
                let val = b.clone();
                self.advance_tok();
                Ok(PyExpr::BytesLiteral(val))
            }
            Some(PyToken::True) => { self.advance_tok(); Ok(PyExpr::BoolLiteral(true)) }
            Some(PyToken::False) => { self.advance_tok(); Ok(PyExpr::BoolLiteral(false)) }
            Some(PyToken::None) => { self.advance_tok(); Ok(PyExpr::NoneLiteral) }
            Some(PyToken::Ellipsis) => { self.advance_tok(); Ok(PyExpr::EllipsisLiteral) }
            Some(PyToken::Identifier(name)) => {
                let name = name.clone();
                self.advance_tok();
                Ok(PyExpr::Name(name))
            }
            Some(PyToken::Print) => {
                // Python 2 compat: treat 'print' as identifier
                self.advance_tok();
                Ok(PyExpr::Name("print".to_string()))
            }
            Some(PyToken::Lambda) => self.parse_lambda(),
            Some(PyToken::LParen) => {
                self.advance_tok();
                if self.check(&PyToken::RParen) {
                    self.advance_tok();
                    return Ok(PyExpr::Tuple(Vec::new()));
                }
                let expr = self.parse_expr()?;
                if self.check(&PyToken::Comma) {
                    // Tuple
                    let mut elts = vec![expr];
                    while self.check(&PyToken::Comma) {
                        self.advance_tok();
                        if self.check(&PyToken::RParen) { break; }
                        elts.push(self.parse_expr()?);
                    }
                    self.expect(&PyToken::RParen)?;
                    Ok(PyExpr::Tuple(elts))
                } else {
                    self.expect(&PyToken::RParen)?;
                    Ok(expr)
                }
            }
            Some(PyToken::LBracket) => {
                self.advance_tok();
                if self.check(&PyToken::RBracket) {
                    self.advance_tok();
                    return Ok(PyExpr::List(Vec::new()));
                }
                let first = self.parse_expr()?;
                if self.check(&PyToken::Comma) || self.check(&PyToken::RBracket) {
                    let mut elts = vec![first];
                    while self.check(&PyToken::Comma) {
                        self.advance_tok();
                        if self.check(&PyToken::RBracket) { break; }
                        elts.push(self.parse_expr()?);
                    }
                    self.expect(&PyToken::RBracket)?;
                    Ok(PyExpr::List(elts))
                } else {
                    // List comprehension
                    let generators = self.parse_comprehension_generators()?;
                    self.expect(&PyToken::RBracket)?;
                    Ok(PyExpr::ListComp {
                        element: Box::new(first),
                        generators,
                    })
                }
            }
            Some(PyToken::LBrace) => {
                self.advance_tok();
                if self.check(&PyToken::RBrace) {
                    self.advance_tok();
                    return Ok(PyExpr::Dict { keys: Vec::new(), values: Vec::new() });
                }
                let first = self.parse_expr()?;
                if self.check(&PyToken::Colon) {
                    // Dict
                    self.advance_tok();
                    let first_val = self.parse_expr()?;
                    let mut keys = vec![Some(first)];
                    let mut values = vec![first_val];
                    while self.check(&PyToken::Comma) {
                        self.advance_tok();
                        if self.check(&PyToken::RBrace) { break; }
                        let k = self.parse_expr()?;
                        self.expect(&PyToken::Colon)?;
                        let v = self.parse_expr()?;
                        keys.push(Some(k));
                        values.push(v);
                    }
                    self.expect(&PyToken::RBrace)?;
                    Ok(PyExpr::Dict { keys, values })
                } else if self.check(&PyToken::Comma) || self.check(&PyToken::RBrace) {
                    // Set
                    let mut elts = vec![first];
                    while self.check(&PyToken::Comma) {
                        self.advance_tok();
                        if self.check(&PyToken::RBrace) { break; }
                        elts.push(self.parse_expr()?);
                    }
                    self.expect(&PyToken::RBrace)?;
                    Ok(PyExpr::Set(elts))
                } else {
                    // Set comprehension
                    let generators = self.parse_comprehension_generators()?;
                    self.expect(&PyToken::RBrace)?;
                    Ok(PyExpr::SetComp {
                        element: Box::new(first),
                        generators,
                    })
                }
            }
            Some(PyToken::Star) => {
                self.advance_tok();
                let expr = self.parse_expr()?;
                Ok(PyExpr::Starred(Box::new(expr)))
            }
            Some(PyToken::Yield) => {
                self.advance_tok();
                if self.check(&PyToken::From) {
                    self.advance_tok();
                    let val = self.parse_expr()?;
                    Ok(PyExpr::YieldFrom(Box::new(val)))
                } else if self.check(&PyToken::Newline) || self.check(&PyToken::Eof) || self.check(&PyToken::RParen) {
                    Ok(PyExpr::Yield(std::option::Option::None))
                } else {
                    let val = self.parse_expr()?;
                    Ok(PyExpr::Yield(Some(Box::new(val))))
                }
            }
            Some(PyToken::Await) => {
                self.advance_tok();
                let val = self.parse_expr()?;
                Ok(PyExpr::Await(Box::new(val)))
            }
            _ => {
                Err(format!("Unexpected token at position {}: {:?}", self.pos, self.peek()).into())
            }
        }
    }

    fn parse_lambda(&mut self) -> Result<PyExpr, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Lambda)?;
        let params = if !self.check(&PyToken::Colon) {
            self.parse_params()?
        } else {
            Vec::new()
        };
        self.expect(&PyToken::Colon)?;
        let body = self.parse_expr()?;
        Ok(PyExpr::Lambda {
            params,
            body: Box::new(body),
        })
    }

    fn parse_comprehension_generators(&mut self) -> Result<Vec<PyComprehension>, Box<dyn std::error::Error>> {
        let mut generators = Vec::new();
        while self.check(&PyToken::For) {
            self.advance_tok();
            let target = self.parse_expr()?;
            self.expect(&PyToken::In)?;
            let iter = self.parse_or()?;
            let mut ifs = Vec::new();
            while self.check(&PyToken::If) {
                self.advance_tok();
                ifs.push(self.parse_or()?);
            }
            generators.push(PyComprehension {
                target,
                iter,
                ifs,
                is_async: false,
            });
        }
        Ok(generators)
    }

    // ── Block parsing ────────────────────────────────────

    fn parse_block(&mut self) -> Result<Vec<PyStmt>, Box<dyn std::error::Error>> {
        self.skip_newlines();

        if self.check(&PyToken::Indent) {
            self.advance_tok();
            let mut stmts = Vec::new();
            loop {
                self.skip_newlines();
                if self.check(&PyToken::Dedent) || self.is_at_end() {
                    break;
                }
                stmts.push(self.parse_statement()?);
            }
            if self.check(&PyToken::Dedent) {
                self.advance_tok();
            }
            Ok(stmts)
        } else {
            // Single-line block (e.g., `if x: pass`)
            let stmt = self.parse_statement()?;
            Ok(vec![stmt])
        }
    }

    // ── Parameters ───────────────────────────────────────

    fn parse_params(&mut self) -> Result<Vec<PyParam>, Box<dyn std::error::Error>> {
        let mut params = Vec::new();

        while !self.check(&PyToken::RParen) && !self.check(&PyToken::Colon) && !self.is_at_end() {
            if self.check(&PyToken::Star) {
                self.advance_tok();
                if self.check(&PyToken::Comma) || self.check(&PyToken::RParen) {
                    // bare * separator
                    if self.check(&PyToken::Comma) { self.advance_tok(); }
                    continue;
                }
                let name = self.expect_identifier()?;
                let annotation = if self.check(&PyToken::Colon) {
                    self.advance_tok();
                    Some(self.parse_type_annotation()?)
                } else {
                    std::option::Option::None
                };
                params.push(PyParam {
                    name,
                    annotation,
                    default: std::option::Option::None,
                    kind: PyParamKind::VarPositional,
                });
            } else if self.check(&PyToken::DoubleStar) {
                self.advance_tok();
                let name = self.expect_identifier()?;
                let annotation = if self.check(&PyToken::Colon) {
                    self.advance_tok();
                    Some(self.parse_type_annotation()?)
                } else {
                    std::option::Option::None
                };
                params.push(PyParam {
                    name,
                    annotation,
                    default: std::option::Option::None,
                    kind: PyParamKind::VarKeyword,
                });
            } else {
                let name = self.expect_identifier()?;
                let annotation = if self.check(&PyToken::Colon) {
                    self.advance_tok();
                    Some(self.parse_type_annotation()?)
                } else {
                    std::option::Option::None
                };
                let default = if self.check(&PyToken::Assign) {
                    self.advance_tok();
                    Some(self.parse_expr()?)
                } else {
                    std::option::Option::None
                };
                params.push(PyParam {
                    name,
                    annotation,
                    default,
                    kind: PyParamKind::Regular,
                });
            }

            if self.check(&PyToken::Comma) {
                self.advance_tok();
            }
        }

        Ok(params)
    }

    // ── Type annotations ─────────────────────────────────

    fn parse_type_annotation(&mut self) -> Result<PyType, Box<dyn std::error::Error>> {
        let name = match self.peek() {
            Some(PyToken::Identifier(s)) => {
                let n = s.clone();
                self.advance_tok();
                n
            }
            Some(PyToken::None) => {
                self.advance_tok();
                return Ok(PyType::None);
            }
            _ => return Ok(PyType::Any),
        };

        // Check for generic parameters: list[int], dict[str, int]
        if self.check(&PyToken::LBracket) {
            self.advance_tok();
            let mut args = Vec::new();
            while !self.check(&PyToken::RBracket) && !self.is_at_end() {
                args.push(self.parse_type_annotation()?);
                if self.check(&PyToken::Comma) { self.advance_tok(); }
            }
            self.expect(&PyToken::RBracket)?;

            return match name.as_str() {
                "list" | "List" => Ok(PyType::List(Box::new(args.into_iter().next().unwrap_or(PyType::Any)))),
                "dict" | "Dict" => {
                    let k = args.first().cloned().unwrap_or(PyType::Any);
                    let v = args.get(1).cloned().unwrap_or(PyType::Any);
                    Ok(PyType::Dict(Box::new(k), Box::new(v)))
                }
                "set" | "Set" => Ok(PyType::Set(Box::new(args.into_iter().next().unwrap_or(PyType::Any)))),
                "tuple" | "Tuple" => Ok(PyType::Tuple(args)),
                "Optional" => Ok(PyType::Optional(Box::new(args.into_iter().next().unwrap_or(PyType::Any)))),
                "Union" => Ok(PyType::Union(args)),
                "Callable" => {
                    let ret = args.last().cloned().unwrap_or(PyType::Any);
                    let params: Vec<PyType> = args.into_iter().rev().skip(1).rev().collect();
                    Ok(PyType::Callable(params, Box::new(ret)))
                }
                _ => Ok(PyType::Custom(name)),
            };
        }

        match name.as_str() {
            "int" => Ok(PyType::Int),
            "float" => Ok(PyType::Float),
            "str" => Ok(PyType::Str),
            "bool" => Ok(PyType::Bool),
            "bytes" => Ok(PyType::Bytes),
            "Any" => Ok(PyType::Any),
            _ => Ok(PyType::Custom(name)),
        }
    }

    // ── Helpers ──────────────────────────────────────────

    fn peek(&self) -> Option<&PyToken> {
        self.tokens.get(self.pos)
    }

    fn advance_tok(&mut self) -> Option<&PyToken> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    fn check(&self, expected: &PyToken) -> bool {
        self.peek().map_or(false, |t| std::mem::discriminant(t) == std::mem::discriminant(expected))
    }

    fn expect(&mut self, expected: &PyToken) -> Result<(), Box<dyn std::error::Error>> {
        if self.check(expected) {
            self.advance_tok();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?} at position {}", expected, self.peek(), self.pos).into())
        }
    }

    fn expect_identifier(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        match self.peek() {
            Some(PyToken::Identifier(name)) => {
                let name = name.clone();
                self.advance_tok();
                Ok(name)
            }
            other => Err(format!("Expected identifier, got {:?} at position {}", other, self.pos).into()),
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek(), Some(PyToken::Eof))
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(PyToken::Newline) | Some(PyToken::Comment(_))) {
            self.advance_tok();
        }
    }

    fn try_parse_docstring(&mut self) -> Option<String> {
        self.skip_newlines();
        if let Some(PyToken::StringLiteral(s)) = self.peek() {
            let doc = s.clone();
            self.advance_tok();
            Some(doc)
        } else {
            std::option::Option::None
        }
    }

    fn extract_docstring(&self, body: &[PyStmt]) -> Option<String> {
        if let Some(PyStmt::Expr(PyExpr::StringLiteral(s))) = body.first() {
            Some(s.clone())
        } else {
            std::option::Option::None
        }
    }

    fn check_aug_assign(&self) -> Option<PyBinOp> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::python::py_lexer::PyLexer;

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
