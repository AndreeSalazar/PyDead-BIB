use super::PyParser;
use crate::frontend::python::ast::*;
use crate::frontend::python::lexer::*;

impl PyParser {
    // ── Statement parsing ────────────────────────────────

    pub fn parse_statement(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_function_def(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_class_def(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_if(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_while(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_for(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::For)?;
        // Parse target as simple name(s), not full expr (avoids consuming 'in' as cmp op)
        let target = self.parse_atom()?;
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

    pub fn parse_try(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_with(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_return(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Return)?;
        if self.check(&PyToken::Newline) || self.check(&PyToken::Eof) || self.check(&PyToken::Dedent) {
            return Ok(PyStmt::Return(std::option::Option::None));
        }
        let value = self.parse_expr()?;
        Ok(PyStmt::Return(Some(value)))
    }

    pub fn parse_import(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_import_from(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_raise(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_assert(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_del(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Del)?;
        let mut targets = Vec::new();
        loop {
            targets.push(self.parse_expr()?);
            if !self.check(&PyToken::Comma) { break; }
            self.advance_tok();
        }
        Ok(PyStmt::Delete(targets))
    }

    pub fn parse_global(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Global)?;
        let mut names = Vec::new();
        loop {
            names.push(self.expect_identifier()?);
            if !self.check(&PyToken::Comma) { break; }
            self.advance_tok();
        }
        Ok(PyStmt::Global(names))
    }

    pub fn parse_nonlocal(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        self.expect(&PyToken::Nonlocal)?;
        let mut names = Vec::new();
        loop {
            names.push(self.expect_identifier()?);
            if !self.check(&PyToken::Comma) { break; }
            self.advance_tok();
        }
        Ok(PyStmt::Nonlocal(names))
    }

    pub fn parse_match(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_pattern(&mut self) -> Result<PyPattern, Box<dyn std::error::Error>> {
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

    pub fn parse_decorated(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
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

    pub fn parse_expr_or_assign(&mut self) -> Result<PyStmt, Box<dyn std::error::Error>> {
        let expr = self.parse_expr()?;

        // Check for tuple target: x, y = ...
        if self.check(&PyToken::Comma) {
            let mut elts = vec![expr];
            while self.check(&PyToken::Comma) {
                self.advance_tok();
                if self.check(&PyToken::Assign) { break; }
                elts.push(self.parse_expr()?);
            }
            if self.check(&PyToken::Assign) {
                self.advance_tok();
                // Parse value side — may also be a tuple
                let first_val = self.parse_expr()?;
                if self.check(&PyToken::Comma) {
                    let mut vals = vec![first_val];
                    while self.check(&PyToken::Comma) {
                        self.advance_tok();
                        if self.is_at_end() || self.check(&PyToken::Newline) { break; }
                        vals.push(self.parse_expr()?);
                    }
                    return Ok(PyStmt::Assign {
                        targets: vec![PyExpr::Tuple(elts)],
                        value: PyExpr::Tuple(vals),
                    });
                }
                return Ok(PyStmt::Assign {
                    targets: vec![PyExpr::Tuple(elts)],
                    value: first_val,
                });
            }
            // Not an assignment — it's a tuple expression
            return Ok(PyStmt::Expr(PyExpr::Tuple(elts)));
        }

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

}
