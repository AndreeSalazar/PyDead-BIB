// Parser para ADead-BIB
// Convierte tokens en AST
// Lenguaje de uso general - Binario + HEX

use super::ast::*;
use super::lexer::{Lexer, Token};
use std::iter::Peekable;
use std::vec::IntoIter;

pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(Token),
    UnexpectedEof,
    ExpectedToken(&'static str),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken(t) => {
                write!(
                    f,
                    "❌ Syntax Error: Unexpected token '{:?}'. Check your syntax.",
                    t
                )
            }
            ParseError::UnexpectedEof => {
                write!(f, "❌ Syntax Error: Unexpected end of file. Missing closing brackets, parentheses, or statements.")
            }
            ParseError::ExpectedToken(s) => {
                write!(
                    f,
                    "❌ Syntax Error: Expected '{}' but found something else.",
                    s
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into_iter().peekable(),
        }
    }

    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek()
    }

    fn advance(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        match self.advance() {
            Some(token) if token == expected => Ok(()),
            Some(token) => Err(ParseError::UnexpectedToken(token)),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Token::Newline)) {
            self.advance();
        }
    }

    /// Check if identifier looks like a type name (starts with uppercase)
    /// Used to disambiguate struct literals from block-delimited constructs
    fn is_type_ident(name: &str) -> bool {
        name.rsplit("::")
            .next()
            .and_then(|seg| seg.chars().next())
            .is_some_and(|c| c.is_ascii_uppercase())
    }

    fn parse_type_name(&mut self) -> Option<String> {
        match self.peek() {
            Some(Token::Identifier(_)) => match self.advance() {
                Some(Token::Identifier(t)) => Some(t),
                _ => None,
            },
            Some(Token::IntType) => {
                self.advance();
                Some("int".to_string())
            }
            Some(Token::CharType) => {
                self.advance();
                Some("char".to_string())
            }
            Some(Token::VoidType) => {
                self.advance();
                Some("void".to_string())
            }
            Some(Token::LongType) => {
                self.advance();
                Some("long".to_string())
            }
            Some(Token::ShortType) => {
                self.advance();
                Some("short".to_string())
            }
            Some(Token::DoubleType) => {
                self.advance();
                Some("double".to_string())
            }
            Some(Token::FloatType) => {
                self.advance();
                Some("float".to_string())
            }
            Some(Token::Bool) => {
                self.advance();
                Some("bool".to_string())
            }
            Some(Token::Str) => {
                self.advance();
                Some("str".to_string())
            }
            _ => {
                self.advance();
                None
            }
        }
    }

    pub fn parse_program(source: &str) -> Result<Program, ParseError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        let tokens: Vec<Token> = tokens
            .into_iter()
            .filter(|t| !matches!(t, Token::Eof))
            .collect();

        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut program = Program::new();

        self.skip_newlines();

        // Parse imports first
        while matches!(self.peek(), Some(Token::Import) | Some(Token::From)) {
            let import = self.parse_import()?;
            program.add_import(import);
            self.skip_newlines();
        }

        while self.peek().is_some() {
            match self.peek() {
                // Python style: def
                Some(Token::Def) => {
                    let func = self.parse_function()?;
                    program.add_function(func);
                    self.skip_newlines();
                }
                // Rust style: fn
                Some(Token::Fn) => {
                    let func = self.parse_rust_function()?;
                    program.add_function(func);
                    self.skip_newlines();
                }
                // C-style: int/void/char/bool/long/short/double/float function (NEW v3.0)
                Some(Token::IntType)
                | Some(Token::VoidType)
                | Some(Token::CharType)
                | Some(Token::Bool)
                | Some(Token::LongType)
                | Some(Token::ShortType)
                | Some(Token::DoubleType)
                | Some(Token::FloatType) => {
                    let func = self.parse_c_function()?;
                    program.add_function(func);
                    self.skip_newlines();
                }
                // Rust style: struct
                Some(Token::Struct) => {
                    let class = self.parse_struct()?;
                    program.add_class(class);
                    self.skip_newlines();
                }
                // Rust style: impl
                Some(Token::Impl) => {
                    self.parse_impl(&mut program)?;
                    self.skip_newlines();
                }
                // Rust style: trait (v1.6.0)
                Some(Token::Trait) => {
                    let trait_def = self.parse_trait_def()?;
                    program.add_trait(trait_def);
                    self.skip_newlines();
                }
                Some(Token::Class) => {
                    let class = self.parse_class()?;
                    program.add_class(class);
                    self.skip_newlines();
                }
                Some(Token::Interface) => {
                    let iface = self.parse_interface()?;
                    program.add_interface(iface);
                    self.skip_newlines();
                }
                // Rust style: let / const
                Some(Token::Let) | Some(Token::Const) => {
                    let stmt = self.parse_let_statement()?;
                    program.add_statement(stmt);
                    self.skip_newlines();
                }
                _ => {
                    // Try to parse as a statement (for scripts)
                    let stmt = self.parse_statement()?;
                    program.add_statement(stmt);
                    self.skip_newlines();
                }
            }
        }

        Ok(program)
    }

    /// Parse module path like "std::math" or "mymodule"
    fn parse_module_path(&mut self) -> Result<String, ParseError> {
        let mut path = match self.advance() {
            Some(Token::Identifier(m)) => m,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        // Handle :: separators for nested modules (e.g., std::math)
        while matches!(self.peek(), Some(Token::DoubleColon)) {
            self.advance(); // consume ::
            path.push_str("::");
            match self.advance() {
                Some(Token::Identifier(part)) => path.push_str(&part),
                Some(token) => return Err(ParseError::UnexpectedToken(token)),
                None => return Err(ParseError::UnexpectedEof),
            }
        }

        Ok(path)
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        if matches!(self.peek(), Some(Token::From)) {
            // from module import item1, item2
            // from std::math import abs, max
            self.advance();
            let module = self.parse_module_path()?;

            self.expect(Token::Import)?;

            let mut items = Vec::new();
            loop {
                match self.advance() {
                    Some(Token::Identifier(i)) => items.push(i),
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                }
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }

            self.skip_newlines();
            Ok(Import {
                module,
                items,
                alias: None,
            })
        } else {
            // import module [as alias]
            self.expect(Token::Import)?;
            let module = match self.advance() {
                Some(Token::Identifier(m)) => m,
                Some(token) => return Err(ParseError::UnexpectedToken(token)),
                None => return Err(ParseError::UnexpectedEof),
            };

            let alias = if matches!(self.peek(), Some(Token::As)) {
                self.advance();
                match self.advance() {
                    Some(Token::Identifier(a)) => Some(a),
                    _ => None,
                }
            } else {
                None
            };

            self.skip_newlines();
            Ok(Import {
                module,
                items: Vec::new(),
                alias,
            })
        }
    }

    fn parse_interface(&mut self) -> Result<Interface, ParseError> {
        self.expect(Token::Interface)?;

        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::Colon)?;
        self.skip_newlines();

        let mut methods = Vec::new();

        // Parse interface body (method signatures only)
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Some(Token::Def)) {
                self.advance();
                let method_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };

                self.expect(Token::LParen)?;
                let mut params = Vec::new();
                if !matches!(self.peek(), Some(Token::RParen)) {
                    loop {
                        if matches!(self.peek(), Some(Token::This)) {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            }
                            continue;
                        }
                        let param_name = match self.advance() {
                            Some(Token::Identifier(n)) => n,
                            Some(token) => return Err(ParseError::UnexpectedToken(token)),
                            None => return Err(ParseError::UnexpectedEof),
                        };

                        let type_name = if matches!(self.peek(), Some(Token::Colon)) {
                            self.advance();
                            self.parse_type_name()
                        } else {
                            None
                        };

                        params.push(Param {
                            name: param_name,
                            param_type: type_name
                                .map(|t| Type::from_c_name(&t))
                                .unwrap_or(Type::Auto),
                            default_value: None,
                        });

                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RParen)?;

                let return_type = if matches!(self.peek(), Some(Token::Arrow)) {
                    self.advance();
                    self.parse_type_name()
                } else {
                    None
                };

                self.skip_newlines();
                let resolved_return_type = return_type
                    .as_ref()
                    .map(|t| Type::from_c_name(t))
                    .unwrap_or(Type::Void);
                methods.push(MethodSignature {
                    name: method_name,
                    params,
                    return_type,
                    resolved_return_type,
                });
            } else {
                break;
            }
        }

        Ok(Interface { name, methods })
    }

    /// Parse Rust-style function: fn name(params) -> ReturnType { body }
    /// Supports &self, &mut self, self as first parameter for methods
    fn parse_rust_function(&mut self) -> Result<Function, ParseError> {
        self.advance(); // consume 'fn'

        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::LParen)?;

        let mut params = Vec::new();
        if !matches!(self.peek(), Some(Token::RParen)) {
            loop {
                // Handle &self, &mut self, or self as first parameter
                if matches!(self.peek(), Some(Token::Ampersand)) {
                    self.advance(); // consume &
                    if matches!(self.peek(), Some(Token::Mut)) {
                        self.advance(); // consume mut
                    }
                    // Expect 'self' after & or &mut
                    if matches!(self.peek(), Some(Token::This)) {
                        self.advance(); // consume self
                                        // Add self as implicit first parameter
                        params.push(Param {
                            name: "self".to_string(),
                            param_type: Type::Named("Self".to_string()),
                            default_value: None,
                        });
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                            continue;
                        } else {
                            break;
                        }
                    }
                }

                // Handle bare 'self'
                if matches!(self.peek(), Some(Token::This)) {
                    self.advance();
                    params.push(Param {
                        name: "self".to_string(),
                        param_type: Type::Named("Self".to_string()),
                        default_value: None,
                    });
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                        continue;
                    } else {
                        break;
                    }
                }

                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }

                let param_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };

                let type_name = if matches!(self.peek(), Some(Token::Colon)) {
                    self.advance();
                    self.parse_type_name()
                } else {
                    None
                };

                params.push(Param {
                    name: param_name,
                    param_type: type_name
                        .map(|t| Type::from_c_name(&t))
                        .unwrap_or(Type::Auto),
                    default_value: None,
                });

                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(Token::RParen)?;

        let return_type = if matches!(self.peek(), Some(Token::Arrow)) {
            self.advance();
            self.parse_type_name()
        } else {
            None
        };

        // Rust style: { body }
        self.expect(Token::LBrace)?;
        self.skip_newlines();

        let mut body = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let stmt = self.parse_statement()?;
            body.push(stmt);
            self.skip_newlines();
            // Skip optional semicolons
            while matches!(self.peek(), Some(Token::Semicolon)) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(Token::RBrace)?;

        let resolved_return_type = return_type
            .as_ref()
            .map(|t| Type::from_c_name(t))
            .unwrap_or(Type::Void);
        Ok(Function {
            name,
            params,
            return_type,
            resolved_return_type,
            body,
            attributes: FunctionAttributes::default(),
        })
    }

    /// Parse C-style function: int/void/char name(params) { body }
    /// Supports: int main() { ... }, void func(int x) { ... }
    fn parse_c_function(&mut self) -> Result<Function, ParseError> {
        // Get return type (int, void, char, etc.)
        let return_type = match self.advance() {
            Some(Token::IntType) => Some("int".to_string()),
            Some(Token::VoidType) => None,
            Some(Token::CharType) => Some("char".to_string()),
            Some(Token::LongType) => Some("long".to_string()),
            Some(Token::ShortType) => Some("short".to_string()),
            Some(Token::DoubleType) => Some("double".to_string()),
            Some(Token::FloatType) => Some("float".to_string()),
            Some(Token::Bool) => Some("bool".to_string()),
            Some(Token::Identifier(t)) => Some(t), // user-defined types (e.g. struct names)
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        // Check for pointer return type (int* func)
        let _is_pointer = if matches!(self.peek(), Some(Token::Star)) {
            self.advance();
            true
        } else {
            false
        };

        // Get function name
        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::LParen)?;

        let mut params = Vec::new();
        if !matches!(self.peek(), Some(Token::RParen)) {
            loop {
                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }

                // Parse C-style parameter: type name or type* name
                let param_type = match self.peek() {
                    Some(Token::IntType) => {
                        self.advance();
                        Some("int".to_string())
                    }
                    Some(Token::CharType) => {
                        self.advance();
                        Some("char".to_string())
                    }
                    Some(Token::VoidType) => {
                        self.advance();
                        Some("void".to_string())
                    }
                    Some(Token::LongType) => {
                        self.advance();
                        Some("long".to_string())
                    }
                    Some(Token::ShortType) => {
                        self.advance();
                        Some("short".to_string())
                    }
                    Some(Token::DoubleType) => {
                        self.advance();
                        Some("double".to_string())
                    }
                    Some(Token::FloatType) => {
                        self.advance();
                        Some("float".to_string())
                    }
                    Some(Token::Identifier(_)) => match self.advance() {
                        Some(Token::Identifier(t)) => Some(t),
                        _ => None,
                    },
                    _ => None,
                };

                // Check for pointer (int* x)
                if matches!(self.peek(), Some(Token::Star)) {
                    self.advance();
                }

                let param_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };

                params.push(Param {
                    name: param_name,
                    param_type: param_type
                        .map(|t| Type::from_c_name(&t))
                        .unwrap_or(Type::Auto),
                    default_value: None,
                });

                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(Token::RParen)?;

        // C style: { body }
        self.expect(Token::LBrace)?;
        self.skip_newlines();

        let mut body = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let stmt = self.parse_c_statement()?;
            body.push(stmt);
            self.skip_newlines();
            // Skip semicolons
            while matches!(self.peek(), Some(Token::Semicolon)) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(Token::RBrace)?;

        let resolved_return_type = return_type
            .as_ref()
            .map(|t| Type::from_c_name(t))
            .unwrap_or(Type::Void);
        Ok(Function {
            name,
            params,
            return_type,
            resolved_return_type,
            body,
            attributes: FunctionAttributes::default(),
        })
    }

    /// Parse C-style statement
    fn parse_c_statement(&mut self) -> Result<Stmt, ParseError> {
        self.skip_newlines();

        match self.peek() {
            // printf("...") or printf("...", args)
            Some(Token::Printf) => {
                self.advance();
                self.expect(Token::LParen)?;
                let format_expr = self.parse_expression()?;

                // Check for additional arguments
                let mut args = vec![format_expr];
                while matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                    args.push(self.parse_expression()?);
                }

                self.expect(Token::RParen)?;

                // Convert to Print (single arg)
                Ok(Stmt::Print(args.into_iter().next().unwrap()))
            }
            // return expr;
            Some(Token::Return) => {
                self.advance();
                if matches!(
                    self.peek(),
                    Some(Token::Semicolon) | Some(Token::Newline) | Some(Token::RBrace)
                ) {
                    Ok(Stmt::Return(None))
                } else {
                    let value = self.parse_expression()?;
                    Ok(Stmt::Return(Some(value)))
                }
            }
            // int x = ...; or int* p = ...; or bool flag = ...;
            Some(Token::IntType)
            | Some(Token::CharType)
            | Some(Token::LongType)
            | Some(Token::ShortType)
            | Some(Token::DoubleType)
            | Some(Token::FloatType)
            | Some(Token::Bool) => {
                self.advance(); // consume type

                // Check for pointer
                let _is_pointer = if matches!(self.peek(), Some(Token::Star)) {
                    self.advance();
                    true
                } else {
                    false
                };

                let name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };

                // Skip optional array size: int arr[5]
                if matches!(self.peek(), Some(Token::LBracket)) {
                    self.advance(); // [
                    while !matches!(self.peek(), Some(Token::RBracket) | None) {
                        self.advance();
                    }
                    if matches!(self.peek(), Some(Token::RBracket)) {
                        self.advance();
                    } // ]
                }

                if matches!(self.peek(), Some(Token::Equals)) {
                    self.advance();
                    let value = self.parse_expression()?;
                    Ok(Stmt::Assign { name, value })
                } else {
                    // Declaration without initialization
                    Ok(Stmt::Assign {
                        name,
                        value: Expr::Number(0),
                    })
                }
            }
            // *ptr = val — deref assignment
            Some(Token::Star) => {
                self.advance(); // consume *
                let target = self.parse_primary()?;
                if matches!(self.peek(), Some(Token::Equals)) {
                    self.advance();
                    let value = self.parse_expression()?;
                    Ok(Stmt::Assign {
                        name: "*deref".to_string(),
                        value: Expr::BinaryOp {
                            op: BinOp::Add,
                            left: Box::new(Expr::Deref(Box::new(target))),
                            right: Box::new(value),
                        },
                    })
                } else {
                    Ok(Stmt::Expr(Expr::Deref(Box::new(target))))
                }
            }
            // if, while, for
            Some(Token::If) => self.parse_if_statement(),
            Some(Token::While) => self.parse_while_statement(),
            Some(Token::For) => self.parse_for_statement(),
            // Default: expression or assignment
            _ => self.parse_statement(),
        }
    }

    /// Parse let statement: let [mut] name [: Type] = value;
    fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
        let is_const = matches!(self.peek(), Some(Token::Const));
        self.advance(); // consume 'let' or 'const'

        // Check for 'mut'
        let _is_mut = if matches!(self.peek(), Some(Token::Mut)) {
            self.advance();
            true
        } else {
            false
        };

        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        // Optional type annotation
        let _type_name = if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            self.parse_type_name()
        } else {
            None
        };

        self.expect(Token::Equals)?;
        let value = self.parse_expression()?;

        // Optional semicolon
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }

        // For const, we could add a different variant, but for now treat as assign
        let _ = is_const; // TODO: Handle const differently

        Ok(Stmt::Assign { name, value })
    }

    /// Parse Rust-style struct
    fn parse_struct(&mut self) -> Result<Class, ParseError> {
        self.advance(); // consume 'struct'

        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::LBrace)?;
        self.skip_newlines();

        let mut fields = Vec::new();

        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let field_name = match self.advance() {
                Some(Token::Identifier(n)) => n,
                Some(Token::RBrace) => break,
                Some(token) => return Err(ParseError::UnexpectedToken(token)),
                None => return Err(ParseError::UnexpectedEof),
            };

            self.expect(Token::Colon)?;

            let type_name = self.parse_type_name();

            fields.push(Field {
                name: field_name,
                type_name: type_name.clone(),
                field_type: type_name
                    .map(|t| Type::from_c_name(&t))
                    .unwrap_or(Type::Auto),
                default_value: None,
            });

            // Skip comma
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
            self.skip_newlines();
        }

        self.expect(Token::RBrace)?;

        Ok(Class {
            name,
            parent: None,
            implements: Vec::new(),
            fields,
            methods: Vec::new(),
            constructor: None,
            destructor: None,
        })
    }

    /// Parse Rust-style impl block
    /// Supports: `impl Struct { }` and `impl Trait for Struct { }`
    fn parse_impl(&mut self, program: &mut Program) -> Result<(), ParseError> {
        self.advance(); // consume 'impl'

        let first_name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        // Check if this is `impl Trait for Struct` or just `impl Struct`
        let (trait_name, struct_name) = if matches!(self.peek(), Some(Token::For)) {
            self.advance(); // consume 'for'
            let sname = match self.advance() {
                Some(Token::Identifier(n)) => n,
                Some(token) => return Err(ParseError::UnexpectedToken(token)),
                None => return Err(ParseError::UnexpectedEof),
            };
            (Some(first_name), sname)
        } else {
            (None, first_name)
        };

        self.expect(Token::LBrace)?;
        self.skip_newlines();

        // Parse methods and add them as functions with prefixed names
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            if matches!(self.peek(), Some(Token::Fn)) {
                let mut func = self.parse_rust_function()?;
                // Prefix method name with struct name (and trait if present)
                if let Some(ref tname) = trait_name {
                    func.name = format!("{}::{}::{}", struct_name, tname, func.name);
                } else {
                    func.name = format!("{}::{}", struct_name, func.name);
                }
                program.add_function(func);
            } else {
                self.advance(); // Skip unknown tokens
            }
            self.skip_newlines();
        }

        self.expect(Token::RBrace)?;

        // Store the impl block for trait verification
        program.add_impl(Impl {
            struct_name: struct_name.clone(),
            trait_name,
            methods: Vec::new(), // Methods are added as functions above
        });

        Ok(())
    }

    /// Parse Rust-style trait (v1.6.0)
    fn parse_trait_def(&mut self) -> Result<Trait, ParseError> {
        self.advance(); // consume 'trait'

        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::LBrace)?;
        self.skip_newlines();

        let mut methods = Vec::new();

        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            if matches!(self.peek(), Some(Token::Fn)) {
                self.advance();
                let method_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };

                self.expect(Token::LParen)?;
                let mut params = Vec::new();

                if !matches!(self.peek(), Some(Token::RParen)) {
                    loop {
                        // Skip &self, &mut self, self
                        if matches!(self.peek(), Some(Token::Ampersand)) {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Mut)) {
                                self.advance();
                            }
                        }
                        if matches!(self.peek(), Some(Token::This)) {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            }
                            continue;
                        }

                        if matches!(self.peek(), Some(Token::RParen)) {
                            break;
                        }

                        let param_name = match self.advance() {
                            Some(Token::Identifier(n)) => n,
                            Some(token) => return Err(ParseError::UnexpectedToken(token)),
                            None => return Err(ParseError::UnexpectedEof),
                        };

                        let type_name = if matches!(self.peek(), Some(Token::Colon)) {
                            self.advance();
                            self.parse_type_name()
                        } else {
                            None
                        };

                        params.push(Param {
                            name: param_name,
                            param_type: type_name
                                .map(|t| Type::from_c_name(&t))
                                .unwrap_or(Type::Auto),
                            default_value: None,
                        });

                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RParen)?;

                let return_type = if matches!(self.peek(), Some(Token::Arrow)) {
                    self.advance();
                    self.parse_type_name()
                } else {
                    None
                };

                // Check for default implementation or semicolon
                let default_body = if matches!(self.peek(), Some(Token::Semicolon)) {
                    self.advance();
                    None
                } else if matches!(self.peek(), Some(Token::LBrace)) {
                    // Parse default implementation body
                    self.advance(); // consume {
                    let mut body = Vec::new();
                    while !matches!(self.peek(), Some(Token::RBrace) | None) {
                        body.push(self.parse_statement()?);
                        self.skip_newlines();
                    }
                    self.expect(Token::RBrace)?;
                    Some(body)
                } else {
                    None
                };

                let resolved_return_type = return_type
                    .as_ref()
                    .map(|t| Type::from_c_name(t))
                    .unwrap_or(Type::Void);
                methods.push(TraitMethod {
                    name: method_name,
                    params,
                    return_type,
                    resolved_return_type,
                    default_body,
                });
            } else {
                self.advance();
            }
            self.skip_newlines();
        }

        self.expect(Token::RBrace)?;

        Ok(Trait { name, methods })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.advance(); // consume 'def'

        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::LParen)?;

        let mut params = Vec::new();
        if !matches!(self.peek(), Some(Token::RParen)) {
            loop {
                let param_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };

                let type_name = if matches!(self.peek(), Some(Token::Colon)) {
                    self.advance();
                    self.parse_type_name()
                } else {
                    None
                };

                params.push(Param {
                    name: param_name,
                    param_type: type_name
                        .map(|t| Type::from_c_name(&t))
                        .unwrap_or(Type::Auto),
                    default_value: None,
                });

                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(Token::RParen)?;

        let return_type = if matches!(self.peek(), Some(Token::Arrow)) {
            self.advance();
            self.parse_type_name()
        } else {
            None
        };

        self.expect(Token::Colon)?;
        self.skip_newlines();

        // Parse block (indentation based in python, but here we simplify to {})
        // Actually, previous examples didn't use braces, so likely indentation or just until next def/class
        // Or maybe just list of statements.
        // Let's assume indentation is handled by lexer or we just parse statements until end of block.
        // For simplicity: parse statements until EOF or Dedent (if we had it) or next Def/Class?
        // Wait, standard python uses indentation.
        // If we don't support indentation tokens, we might rely on "End" keyword or braces?
        // User's example:
        // def println(msg):
        //     print(msg)
        // Indentation.
        // If lexer produces Indent/Dedent tokens, we use them.
        // Let's assume simple parsing: read statements until we hit something that doesn't look like a statement belonging to the block.
        // Or if we use braces { }.
        // Given I don't see Indent tokens in Lexer usage here, I'll assume we parse one statement or a block if { } used.
        // OR: parse until next Def/Class/Interface/EOF?
        // This is tricky without Indent tokens.
        // Let's assume we read statements.

        let mut body = Vec::new();
        // Temporary hack: read until next Def/Class/Interface/EOF
        // Real implementation should use Indent/Dedent from Lexer.

        while self.peek().is_some() {
            match self.peek() {
                Some(Token::Def) | Some(Token::Class) | Some(Token::Interface) => break,
                _ => {
                    let stmt = self.parse_statement()?;
                    body.push(stmt);
                    self.skip_newlines();
                }
            }
        }

        let resolved_return_type = return_type
            .as_ref()
            .map(|t| Type::from_c_name(t))
            .unwrap_or(Type::Void);
        Ok(Function {
            name,
            params,
            return_type,
            resolved_return_type,
            body,
            attributes: FunctionAttributes::default(),
        })
    }

    /// Parse Python-style class: class Name: or class Name(Parent):
    fn parse_class(&mut self) -> Result<Class, ParseError> {
        self.advance(); // consume 'class'
        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            _ => return Err(ParseError::UnexpectedEof),
        };

        // Check for inheritance: class Child(Parent):
        let parent = if matches!(self.peek(), Some(Token::LParen)) {
            self.advance(); // consume (
            let parent_name = match self.advance() {
                Some(Token::Identifier(n)) => Some(n),
                _ => None,
            };
            if matches!(self.peek(), Some(Token::RParen)) {
                self.advance(); // consume )
            }
            parent_name
        } else {
            None
        };

        self.expect(Token::Colon)?;
        self.skip_newlines();

        let mut methods = Vec::new();
        let mut fields = Vec::new();
        let mut constructor = None;

        // Parse class body - methods and fields
        while self.peek().is_some() {
            match self.peek() {
                Some(Token::Def) => {
                    // Parse method
                    let method = self.parse_class_method(&name)?;
                    if method.name == "__init__" {
                        constructor = Some(method);
                    } else {
                        methods.push(method);
                    }
                    self.skip_newlines();
                }
                Some(Token::Identifier(_)) => {
                    // Parse field: field_name = default_value
                    let field = self.parse_class_field()?;
                    fields.push(field);
                    self.skip_newlines();
                }
                Some(Token::Class)
                | Some(Token::Interface)
                | Some(Token::Fn)
                | Some(Token::Impl)
                | Some(Token::Trait) => break,
                Some(Token::Newline) => {
                    self.advance();
                }
                _ => break,
            }
        }

        Ok(Class {
            name,
            parent,
            implements: Vec::new(),
            fields,
            methods,
            constructor,
            destructor: None,
        })
    }

    /// Parse a method inside a Python-style class
    fn parse_class_method(&mut self, _class_name: &str) -> Result<Method, ParseError> {
        self.advance(); // consume 'def'

        let method_name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            Some(Token::Init) => "__init__".to_string(),
            _ => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::LParen)?;

        let mut params = Vec::new();
        let mut is_static = true;

        if !matches!(self.peek(), Some(Token::RParen)) {
            loop {
                // Handle 'self' as first parameter
                if matches!(self.peek(), Some(Token::This)) {
                    self.advance();
                    is_static = false;
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                        continue;
                    } else {
                        break;
                    }
                }

                if matches!(self.peek(), Some(Token::RParen)) {
                    break;
                }

                let param_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    _ => break,
                };

                let type_name = if matches!(self.peek(), Some(Token::Colon)) {
                    self.advance();
                    self.parse_type_name()
                } else {
                    None
                };

                params.push(Param {
                    name: param_name,
                    param_type: type_name
                        .map(|t| Type::from_c_name(&t))
                        .unwrap_or(Type::Auto),
                    default_value: None,
                });

                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(Token::RParen)?;

        // Optional return type
        let return_type = if matches!(self.peek(), Some(Token::Arrow)) {
            self.advance();
            self.parse_type_name()
        } else {
            None
        };

        self.expect(Token::Colon)?;
        self.skip_newlines();

        // Parse method body
        let mut body = Vec::new();
        while self.peek().is_some() {
            match self.peek() {
                Some(Token::Def)
                | Some(Token::Class)
                | Some(Token::Interface)
                | Some(Token::Fn)
                | Some(Token::Impl)
                | Some(Token::Trait) => break,
                Some(Token::Return) => {
                    self.advance();
                    let expr = self.parse_expression()?;
                    body.push(Stmt::Return(Some(expr)));
                    self.skip_newlines();
                    break;
                }
                _ => {
                    let stmt = self.parse_statement()?;
                    body.push(stmt);
                    self.skip_newlines();
                }
            }
        }

        let resolved_return_type = return_type
            .as_ref()
            .map(|t| Type::from_c_name(t))
            .unwrap_or(Type::Void);
        Ok(Method {
            name: method_name,
            params,
            return_type,
            resolved_return_type,
            body,
            is_virtual: false,
            is_override: false,
            is_static,
        })
    }

    /// Parse a field inside a Python-style class
    fn parse_class_field(&mut self) -> Result<Field, ParseError> {
        let name = match self.advance() {
            Some(Token::Identifier(n)) => n,
            _ => return Err(ParseError::UnexpectedEof),
        };

        let default_value = if matches!(self.peek(), Some(Token::Equals)) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Field {
            name,
            type_name: None,
            field_type: Type::Auto,
            default_value,
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        match self.peek() {
            // Rust style: let / const inside functions
            Some(Token::Let) | Some(Token::Const) => self.parse_let_statement(),
            // Control de flujo
            Some(Token::If) => self.parse_if_statement(),
            Some(Token::While) => self.parse_while_statement(),
            Some(Token::For) => self.parse_for_statement(),
            Some(Token::Do) => {
                self.advance(); // consume 'do'
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                self.expect(Token::RBrace)?;
                self.expect(Token::While)?;
                let has_paren = matches!(self.peek(), Some(Token::LParen));
                if has_paren {
                    self.advance();
                }
                let condition = self.parse_comparison()?;
                if has_paren {
                    self.expect(Token::RParen)?;
                }
                Ok(Stmt::DoWhile { body, condition })
            }
            Some(Token::Break) => {
                self.advance();
                Ok(Stmt::Break)
            }
            Some(Token::Continue) => {
                self.advance();
                Ok(Stmt::Continue)
            }
            // C-style printf (NEW v3.0)
            Some(Token::Printf) => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                // Skip additional args for now
                while matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                    let _ = self.parse_expression()?;
                }
                self.expect(Token::RParen)?;
                Ok(Stmt::Print(expr))
            }
            // C-style variable declarations (NEW v3.0)
            Some(Token::IntType)
            | Some(Token::CharType)
            | Some(Token::LongType)
            | Some(Token::ShortType)
            | Some(Token::DoubleType)
            | Some(Token::FloatType)
            | Some(Token::Bool) => {
                self.advance(); // consume type
                                // Check for pointer
                if matches!(self.peek(), Some(Token::Star)) {
                    self.advance();
                }
                let name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                // Skip optional array size bracket: int arr[5]
                if matches!(self.peek(), Some(Token::LBracket)) {
                    self.advance(); // [
                    while !matches!(self.peek(), Some(Token::RBracket) | None) {
                        self.advance();
                    }
                    if matches!(self.peek(), Some(Token::RBracket)) {
                        self.advance();
                    } // ]
                }
                if matches!(self.peek(), Some(Token::Equals)) {
                    self.advance();
                    let value = self.parse_expression()?;
                    Ok(Stmt::Assign { name, value })
                } else {
                    Ok(Stmt::Assign {
                        name,
                        value: Expr::Number(0),
                    })
                }
            }
            // C-style return (handled here too)
            Some(Token::Return) => {
                self.advance();
                if matches!(
                    self.peek(),
                    Some(Token::Semicolon) | Some(Token::Newline) | Some(Token::RBrace)
                ) {
                    Ok(Stmt::Return(None))
                } else {
                    let value = self.parse_expression()?;
                    Ok(Stmt::Return(Some(value)))
                }
            }
            Some(Token::Print) => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Stmt::Print(expr))
            }
            Some(Token::Println) => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Stmt::Println(expr))
            }
            // ========== SELF (this) MEMBER ACCESS / ASSIGNMENT ==========
            Some(Token::This) => {
                self.advance(); // consume 'self'

                if matches!(self.peek(), Some(Token::Dot)) {
                    self.advance(); // consume '.'
                    let member = match self.advance() {
                        Some(Token::Identifier(m)) => m,
                        Some(tok) => return Err(ParseError::UnexpectedToken(tok)),
                        None => return Err(ParseError::UnexpectedEof),
                    };

                    // self.method(args)
                    if matches!(self.peek(), Some(Token::LParen)) {
                        self.advance();
                        let mut args = Vec::new();
                        if !matches!(self.peek(), Some(Token::RParen)) {
                            loop {
                                args.push(self.parse_expression()?);
                                if matches!(self.peek(), Some(Token::Comma)) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(Token::RParen)?;
                        return Ok(Stmt::Expr(Expr::MethodCall {
                            object: Box::new(Expr::This),
                            method: member,
                            args,
                        }));
                    }

                    // self.field = value
                    if matches!(self.peek(), Some(Token::Equals)) {
                        self.advance();
                        let value = self.parse_expression()?;
                        return Ok(Stmt::FieldAssign {
                            object: Expr::This,
                            field: member,
                            value,
                        });
                    }

                    // self.field += value
                    if matches!(self.peek(), Some(Token::PlusEq)) {
                        self.advance();
                        let right = self.parse_expression()?;
                        let value = Expr::BinaryOp {
                            left: Box::new(Expr::FieldAccess {
                                object: Box::new(Expr::This),
                                field: member.clone(),
                            }),
                            op: BinOp::Add,
                            right: Box::new(right),
                        };
                        return Ok(Stmt::FieldAssign {
                            object: Expr::This,
                            field: member,
                            value,
                        });
                    }

                    // self.field -= value
                    if matches!(self.peek(), Some(Token::MinusEq)) {
                        self.advance();
                        let right = self.parse_expression()?;
                        let value = Expr::BinaryOp {
                            left: Box::new(Expr::FieldAccess {
                                object: Box::new(Expr::This),
                                field: member.clone(),
                            }),
                            op: BinOp::Sub,
                            right: Box::new(right),
                        };
                        return Ok(Stmt::FieldAssign {
                            object: Expr::This,
                            field: member,
                            value,
                        });
                    }

                    // self.field *= value
                    if matches!(self.peek(), Some(Token::StarEq)) {
                        self.advance();
                        let right = self.parse_expression()?;
                        let value = Expr::BinaryOp {
                            left: Box::new(Expr::FieldAccess {
                                object: Box::new(Expr::This),
                                field: member.clone(),
                            }),
                            op: BinOp::Mul,
                            right: Box::new(right),
                        };
                        return Ok(Stmt::FieldAssign {
                            object: Expr::This,
                            field: member,
                            value,
                        });
                    }

                    // self.field /= value
                    if matches!(self.peek(), Some(Token::SlashEq)) {
                        self.advance();
                        let right = self.parse_expression()?;
                        let value = Expr::BinaryOp {
                            left: Box::new(Expr::FieldAccess {
                                object: Box::new(Expr::This),
                                field: member.clone(),
                            }),
                            op: BinOp::Div,
                            right: Box::new(right),
                        };
                        return Ok(Stmt::FieldAssign {
                            object: Expr::This,
                            field: member,
                            value,
                        });
                    }

                    // self.field (read-only expression)
                    return Ok(Stmt::Expr(Expr::FieldAccess {
                        object: Box::new(Expr::This),
                        field: member,
                    }));
                }

                Ok(Stmt::Expr(Expr::This))
            }
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();

                // Handle qualified names: Struct::method
                let mut name = name;
                while matches!(self.peek(), Some(Token::DoubleColon)) {
                    self.advance(); // consume ::
                    match self.advance() {
                        Some(Token::Identifier(part)) => name = format!("{}::{}", name, part),
                        _ => return Err(ParseError::ExpectedToken("identifier after ::")),
                    }
                }

                // Handle index assignment: arr[i] = val
                if matches!(self.peek(), Some(Token::LBracket)) {
                    self.advance(); // [
                    let index = self.parse_expression()?;
                    self.expect(Token::RBracket)?;

                    if matches!(self.peek(), Some(Token::Equals)) {
                        self.advance();
                        let value = self.parse_expression()?;
                        return Ok(Stmt::IndexAssign {
                            object: Expr::Variable(name),
                            index,
                            value,
                        });
                    }
                    // arr[i] as expression (e.g. passed to printf)
                    let indexed = Expr::Index {
                        object: Box::new(Expr::Variable(name)),
                        index: Box::new(index),
                    };
                    return Ok(Stmt::Expr(indexed));
                }

                // Handle member call/access: obj.method() or obj.field
                if matches!(self.peek(), Some(Token::Dot)) {
                    self.advance(); // consume '.'
                    let member = match self.advance() {
                        Some(Token::Identifier(m)) => m,
                        Some(Token::This) => "self".to_string(),
                        Some(tok) => return Err(ParseError::UnexpectedToken(tok)),
                        None => return Err(ParseError::UnexpectedEof),
                    };
                    if matches!(self.peek(), Some(Token::LParen)) {
                        self.advance();
                        let mut args = Vec::new();
                        if !matches!(self.peek(), Some(Token::RParen)) {
                            loop {
                                args.push(self.parse_expression()?);
                                if matches!(self.peek(), Some(Token::Comma)) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(Token::RParen)?;
                        return Ok(Stmt::Expr(Expr::MethodCall {
                            object: Box::new(Expr::Variable(name)),
                            method: member,
                            args,
                        }));
                    } else {
                        // field assignment: obj.field = val
                        if matches!(self.peek(), Some(Token::Equals)) {
                            self.advance();
                            let value = self.parse_expression()?;
                            return Ok(Stmt::Assign {
                                name: format!("{}.{}", name, member),
                                value,
                            });
                        }
                        return Ok(Stmt::Expr(Expr::FieldAccess {
                            object: Box::new(Expr::Variable(name)),
                            field: member,
                        }));
                    }
                }

                if matches!(self.peek(), Some(Token::Equals)) {
                    self.advance();
                    let value = self.parse_expression()?;
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::PlusEq)) {
                    // counter += 1 -> counter = counter + 1
                    self.advance();
                    let right = self.parse_expression()?;
                    let value = Expr::BinaryOp {
                        left: Box::new(Expr::Variable(name.clone())),
                        op: BinOp::Add,
                        right: Box::new(right),
                    };
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::MinusEq)) {
                    // counter -= 1 -> counter = counter - 1
                    self.advance();
                    let right = self.parse_expression()?;
                    let value = Expr::BinaryOp {
                        left: Box::new(Expr::Variable(name.clone())),
                        op: BinOp::Sub,
                        right: Box::new(right),
                    };
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::StarEq)) {
                    // counter *= 2 -> counter = counter * 2
                    self.advance();
                    let right = self.parse_expression()?;
                    let value = Expr::BinaryOp {
                        left: Box::new(Expr::Variable(name.clone())),
                        op: BinOp::Mul,
                        right: Box::new(right),
                    };
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::SlashEq)) {
                    // counter /= 2 -> counter = counter / 2
                    self.advance();
                    let right = self.parse_expression()?;
                    let value = Expr::BinaryOp {
                        left: Box::new(Expr::Variable(name.clone())),
                        op: BinOp::Div,
                        right: Box::new(right),
                    };
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::PercentEq)) {
                    // counter %= 2 -> counter = counter % 2
                    self.advance();
                    let right = self.parse_expression()?;
                    let value = Expr::BinaryOp {
                        left: Box::new(Expr::Variable(name.clone())),
                        op: BinOp::Mod,
                        right: Box::new(right),
                    };
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::AmpEq)) {
                    // x &= mask -> x = x & mask
                    self.advance();
                    let right = self.parse_expression()?;
                    let value = Expr::BitwiseOp {
                        left: Box::new(Expr::Variable(name.clone())),
                        op: BitwiseOp::And,
                        right: Box::new(right),
                    };
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::PipeEq)) {
                    // x |= mask -> x = x | mask
                    self.advance();
                    let right = self.parse_expression()?;
                    let value = Expr::BitwiseOp {
                        left: Box::new(Expr::Variable(name.clone())),
                        op: BitwiseOp::Or,
                        right: Box::new(right),
                    };
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::CaretEq)) {
                    // x ^= mask -> x = x ^ mask
                    self.advance();
                    let right = self.parse_expression()?;
                    let value = Expr::BitwiseOp {
                        left: Box::new(Expr::Variable(name.clone())),
                        op: BitwiseOp::Xor,
                        right: Box::new(right),
                    };
                    Ok(Stmt::Assign { name, value })
                } else if matches!(self.peek(), Some(Token::LBrace)) && Self::is_type_ident(&name) {
                    // Struct literal as statement: MyStruct { field: val }
                    self.advance(); // consume '{'
                    self.skip_newlines();
                    let mut args = Vec::new();
                    while !matches!(self.peek(), Some(Token::RBrace) | None) {
                        self.skip_newlines();
                        let _fname = match self.advance() {
                            Some(Token::Identifier(n)) => n,
                            Some(Token::RBrace) => break,
                            Some(tok) => return Err(ParseError::UnexpectedToken(tok)),
                            None => return Err(ParseError::UnexpectedEof),
                        };
                        self.expect(Token::Colon)?;
                        args.push(self.parse_expression()?);
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        }
                        self.skip_newlines();
                    }
                    self.expect(Token::RBrace)?;
                    Ok(Stmt::Expr(Expr::Call {
                        name: format!("__struct__{}", name),
                        args,
                    }))
                } else if matches!(self.peek(), Some(Token::LParen)) {
                    self.advance(); // (
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Stmt::Expr(Expr::Call { name, args }))
                } else {
                    // Just an identifier expression?
                    Ok(Stmt::Expr(Expr::Variable(name)))
                }
            }
            // ========== OS-LEVEL STATEMENTS (v3.1-OS) ==========
            Some(Token::CliKw) => {
                self.advance();
                Ok(Stmt::Cli)
            }
            Some(Token::StiKw) => {
                self.advance();
                Ok(Stmt::Sti)
            }
            Some(Token::HltKw) => {
                self.advance();
                Ok(Stmt::Hlt)
            }
            Some(Token::IretKw) => {
                self.advance();
                Ok(Stmt::Iret)
            }
            Some(Token::CpuidKw) => {
                self.advance();
                Ok(Stmt::Cpuid)
            }
            // org 0x7C00
            Some(Token::OrgKw) => {
                self.advance();
                let addr_expr = self.parse_expression()?;
                let address = match addr_expr {
                    Expr::Number(n) => n as u64,
                    _ => 0,
                };
                Ok(Stmt::OrgDirective { address })
            }
            // align 16
            Some(Token::AlignKw) => {
                self.advance();
                let align_expr = self.parse_expression()?;
                let alignment = match align_expr {
                    Expr::Number(n) => n as u64,
                    _ => 1,
                };
                Ok(Stmt::AlignDirective { alignment })
            }
            // int_call(0x10)
            Some(Token::IntCallKw) => {
                self.advance();
                self.expect(Token::LParen)?;
                let vec_expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                let vector = match vec_expr {
                    Expr::Number(n) => n as u8,
                    _ => 0,
                };
                Ok(Stmt::IntCall { vector })
            }
            // write_mem(addr, value)
            Some(Token::WriteMemKw) => {
                self.advance();
                self.expect(Token::LParen)?;
                let addr = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let value = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Stmt::MemWrite { addr, value })
            }
            // port_out(port, value)
            Some(Token::PortOutKw) => {
                self.advance();
                self.expect(Token::LParen)?;
                let port = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let value = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Stmt::PortOut { port, value })
            }
            // far_jump(selector, offset)
            Some(Token::FarJumpKw) => {
                self.advance();
                self.expect(Token::LParen)?;
                let sel_expr = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let off_expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                let selector = match sel_expr {
                    Expr::Number(n) => n as u16,
                    _ => 0x08,
                };
                let offset = match off_expr {
                    Expr::Number(n) => n as u32,
                    _ => 0,
                };
                Ok(Stmt::FarJump { selector, offset })
            }
            // raw { 0xEB, 0xFE }
            Some(Token::RawBlockKw) => {
                self.advance();
                self.expect(Token::LBrace)?;
                let mut bytes = Vec::new();
                while !matches!(self.peek(), Some(Token::RBrace) | None) {
                    let byte_expr = self.parse_expression()?;
                    if let Expr::Number(n) = byte_expr {
                        bytes.push(n as u8);
                    }
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(Stmt::RawBlock { bytes })
            }
            // reg rax = value
            Some(Token::RegKw) => {
                self.advance();
                let reg_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                self.expect(Token::Equals)?;
                let value = self.parse_expression()?;
                Ok(Stmt::RegAssign { reg_name, value })
            }
            // ========== LABELS Y JUMPS (v3.3-Boot) ==========
            // label name:
            Some(Token::LabelKw) => {
                self.advance();
                let name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                // Optional colon after label name
                if matches!(self.peek(), Some(Token::Colon)) {
                    self.advance();
                }
                Ok(Stmt::LabelDef { name })
            }
            // jmp label_name
            Some(Token::JmpKw) => {
                self.advance();
                let label = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                Ok(Stmt::JumpTo { label })
            }
            // jz label_name
            Some(Token::JzKw) => {
                self.advance();
                let label = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                Ok(Stmt::JumpIfZero { label })
            }
            // jnz label_name
            Some(Token::JnzKw) => {
                self.advance();
                let label = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                Ok(Stmt::JumpIfNotZero { label })
            }
            // jc label_name
            Some(Token::JcKw) => {
                self.advance();
                let label = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                Ok(Stmt::JumpIfCarry { label })
            }
            // jnc label_name
            Some(Token::JncKw) => {
                self.advance();
                let label = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                Ok(Stmt::JumpIfNotCarry { label })
            }
            // db 0x55, 0xAA or db "string"
            Some(Token::DbKw) => {
                self.advance();
                let mut bytes = Vec::new();
                // Check if it's a string
                if let Some(Token::String(s)) = self.peek().cloned() {
                    self.advance();
                    bytes.extend(s.as_bytes());
                } else {
                    // Parse comma-separated bytes
                    loop {
                        let byte_expr = self.parse_expression()?;
                        if let Expr::Number(n) = byte_expr {
                            bytes.push(n as u8);
                        }
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                Ok(Stmt::DataBytes { bytes })
            }
            // dw 0x1234, 0x5678
            Some(Token::DwKw) => {
                self.advance();
                let mut words = Vec::new();
                loop {
                    let word_expr = self.parse_expression()?;
                    if let Expr::Number(n) = word_expr {
                        words.push(n as u16);
                    }
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                Ok(Stmt::DataWords { words })
            }
            // dd 0x12345678
            Some(Token::DdKw) => {
                self.advance();
                let mut dwords = Vec::new();
                loop {
                    let dword_expr = self.parse_expression()?;
                    if let Expr::Number(n) = dword_expr {
                        dwords.push(n as u32);
                    }
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                Ok(Stmt::DataDwords { dwords })
            }
            // times 510 db 0
            Some(Token::TimesKw) => {
                self.advance();
                let count_expr = self.parse_expression()?;
                let count = match count_expr {
                    Expr::Number(n) => n as usize,
                    _ => 1,
                };
                // Expect 'db'
                self.expect(Token::DbKw)?;
                let byte_expr = self.parse_expression()?;
                let byte = match byte_expr {
                    Expr::Number(n) => n as u8,
                    _ => 0,
                };
                Ok(Stmt::TimesDirective { count, byte })
            }
            _ => {
                // Try parse expression (handles deref assignment: *ptr = val)
                // Check for *ptr = val
                if matches!(self.peek(), Some(Token::Star)) {
                    self.advance(); // consume *
                    let target = self.parse_primary()?;
                    if matches!(self.peek(), Some(Token::Equals)) {
                        self.advance();
                        let value = self.parse_expression()?;
                        // Represent deref assignment as Assign to special name
                        return Ok(Stmt::Assign {
                            name: format!("*deref"),
                            value: Expr::BinaryOp {
                                op: BinOp::Add,
                                left: Box::new(Expr::Deref(Box::new(target))),
                                right: Box::new(value),
                            },
                        });
                    }
                    let expr = self.parse_expression()?;
                    return Ok(Stmt::Expr(Expr::BinaryOp {
                        op: BinOp::Mul,
                        left: Box::new(target),
                        right: Box::new(expr),
                    }));
                }
                let expr = self.parse_expression()?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    // ========================================
    // Control de flujo
    // ========================================

    fn parse_if_statement(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // consume 'if'

        // Condición (puede tener paréntesis o no)
        let has_paren = matches!(self.peek(), Some(Token::LParen));
        if has_paren {
            self.advance();
        }

        let condition = self.parse_comparison()?;

        if has_paren {
            self.expect(Token::RParen)?;
        }

        // Cuerpo con llaves
        self.expect(Token::LBrace)?;
        let then_body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        // else opcional (supports else if chains)
        let else_body = if matches!(self.peek(), Some(Token::Else)) {
            self.advance();
            if matches!(self.peek(), Some(Token::If)) {
                // else if chain: parse nested if as single statement in else body
                let nested_if = self.parse_if_statement()?;
                Some(vec![nested_if])
            } else {
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                self.expect(Token::RBrace)?;
                Some(body)
            }
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // consume 'while'

        let has_paren = matches!(self.peek(), Some(Token::LParen));
        if has_paren {
            self.advance();
        }

        let condition = self.parse_comparison()?;

        if has_paren {
            self.expect(Token::RParen)?;
        }

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Stmt::While { condition, body })
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // consume 'for'

        // for i in 0..10 { } OR for x in arr { }
        let var = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err(ParseError::ExpectedToken("identifier")),
        };

        self.expect(Token::In)?;

        let first_expr = self.parse_expression()?;

        // Verificar si es un rango (0..10) o un iterable (arr)
        if matches!(self.peek(), Some(Token::DoubleDot)) {
            // Es un rango: for i in 0..10
            self.advance(); // consume ..
            let end = self.parse_expression()?;

            self.expect(Token::LBrace)?;
            let body = self.parse_block()?;
            self.expect(Token::RBrace)?;

            Ok(Stmt::For {
                var,
                start: first_expr,
                end,
                body,
            })
        } else {
            // Es un iterable: for x in arr
            self.expect(Token::LBrace)?;
            let body = self.parse_block()?;
            self.expect(Token::RBrace)?;

            Ok(Stmt::ForEach {
                var,
                iterable: first_expr,
                body,
            })
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();

        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            // Saltar newlines
            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }
            if matches!(self.peek(), Some(Token::RBrace)) {
                break;
            }
            stmts.push(self.parse_statement()?);
        }

        Ok(stmts)
    }

    // ========================================
    // Expresiones
    // ========================================

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_binary_op()?;

        if let Some(op) = self.match_comparison_op() {
            let right = self.parse_binary_op()?;
            Ok(Expr::Comparison {
                op,
                left: Box::new(left),
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn match_comparison_op(&mut self) -> Option<CmpOp> {
        match self.peek() {
            Some(Token::EqEq) => {
                self.advance();
                Some(CmpOp::Eq)
            }
            Some(Token::NotEq) => {
                self.advance();
                Some(CmpOp::Ne)
            }
            Some(Token::Less) => {
                self.advance();
                Some(CmpOp::Lt)
            }
            Some(Token::LessEq) => {
                self.advance();
                Some(CmpOp::Le)
            }
            Some(Token::Greater) => {
                self.advance();
                Some(CmpOp::Gt)
            }
            Some(Token::GreaterEq) => {
                self.advance();
                Some(CmpOp::Ge)
            }
            _ => None,
        }
    }

    fn parse_binary_op(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_postfix()?;

        while let Some(op) = self.match_binary_op() {
            let right = self.parse_postfix()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        // Handle bitwise ops at same level
        while matches!(
            self.peek(),
            Some(Token::Ampersand)
                | Some(Token::Pipe)
                | Some(Token::Caret)
                | Some(Token::LeftShift)
                | Some(Token::RightShift)
        ) {
            let bop = match self.advance() {
                Some(Token::Ampersand) => BitwiseOp::And,
                Some(Token::Pipe) => BitwiseOp::Or,
                Some(Token::Caret) => BitwiseOp::Xor,
                Some(Token::LeftShift) => BitwiseOp::LeftShift,
                Some(Token::RightShift) => BitwiseOp::RightShift,
                _ => unreachable!(),
            };
            let right = self.parse_postfix()?;
            left = Expr::BitwiseOp {
                op: bop,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Handle postfix: obj.field, obj.method(...), arr[i]
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if matches!(self.peek(), Some(Token::Dot)) {
                self.advance(); // consume '.'
                let member = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(Token::This) => "self".to_string(),
                    Some(tok) => return Err(ParseError::UnexpectedToken(tok)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                if matches!(self.peek(), Some(Token::LParen)) {
                    // method call: obj.method(args)
                    self.advance();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    // Build as Call with dotted name for the code-gen to handle
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method: member,
                        args,
                    };
                } else {
                    // field access: obj.field
                    expr = Expr::FieldAccess {
                        object: Box::new(expr),
                        field: member,
                    };
                }
            } else if matches!(self.peek(), Some(Token::LBracket)) {
                // array indexing: arr[i]
                self.advance();
                let index = self.parse_expression()?;
                self.expect(Token::RBracket)?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn match_binary_op(&mut self) -> Option<BinOp> {
        match self.peek() {
            Some(Token::Plus) => {
                self.advance();
                Some(BinOp::Add)
            }
            Some(Token::Minus) => {
                self.advance();
                Some(BinOp::Sub)
            }
            Some(Token::Star) => {
                self.advance();
                Some(BinOp::Mul)
            }
            Some(Token::Slash) => {
                self.advance();
                Some(BinOp::Div)
            }
            Some(Token::Percent) => {
                self.advance();
                Some(BinOp::Mod)
            }
            _ => None,
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.advance() {
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::Float(f)) => Ok(Expr::Float(f)),
            Some(Token::String(s)) => Ok(Expr::String(s)),
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::Minus) => {
                // Número negativo: -42, o negación: -expr
                match self.peek() {
                    Some(Token::Number(_)) => match self.advance() {
                        Some(Token::Number(n)) => Ok(Expr::Number(-n)),
                        _ => unreachable!(),
                    },
                    Some(Token::Float(_)) => match self.advance() {
                        Some(Token::Float(f)) => Ok(Expr::Float(-f)),
                        _ => unreachable!(),
                    },
                    _ => {
                        // unary minus: -(expr)
                        let inner = self.parse_primary()?;
                        Ok(Expr::BinaryOp {
                            op: BinOp::Sub,
                            left: Box::new(Expr::Number(0)),
                            right: Box::new(inner),
                        })
                    }
                }
            }
            // Unary NOT / tilde (~x or !x)
            Some(Token::Not) | Some(Token::Tilde) => {
                let inner = self.parse_primary()?;
                Ok(Expr::BitwiseNot(Box::new(inner)))
            }
            // Address-of: &var
            Some(Token::Ampersand) => {
                let inner = self.parse_primary()?;
                Ok(Expr::AddressOf(Box::new(inner)))
            }
            // Dereference: *ptr
            Some(Token::Star) => {
                let inner = self.parse_primary()?;
                Ok(Expr::Deref(Box::new(inner)))
            }
            Some(Token::Input) => {
                // input() - lee un número del teclado
                self.expect(Token::LParen)?;
                self.expect(Token::RParen)?;
                Ok(Expr::Input)
            }
            // Built-in functions v1.3.0
            Some(Token::Len) => {
                // len(expr) - longitud de array/string
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expr::Len(Box::new(expr)))
            }
            Some(Token::Int) => {
                // int(expr) - convertir a entero
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expr::IntCast(Box::new(expr)))
            }
            Some(Token::FloatCast) => {
                // float(expr) - convertir a flotante
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expr::FloatCast(Box::new(expr)))
            }
            Some(Token::Str) => {
                // str(expr) - convertir a string
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expr::StrCast(Box::new(expr)))
            }
            Some(Token::BoolCast) => {
                // bool(expr) - convertir a booleano
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expr::BoolCast(Box::new(expr)))
            }
            Some(Token::SizeofKw) => {
                // sizeof(type) or sizeof(expr)
                self.expect(Token::LParen)?;
                let arg = match self.peek() {
                    Some(Token::IntType)
                    | Some(Token::CharType)
                    | Some(Token::VoidType)
                    | Some(Token::LongType)
                    | Some(Token::ShortType)
                    | Some(Token::DoubleType)
                    | Some(Token::FloatType)
                    | Some(Token::Bool) => {
                        let type_tok = self.advance().unwrap();
                        // Check for pointer: sizeof(int*)
                        if matches!(self.peek(), Some(Token::Star)) {
                            self.advance(); // consume *
                        }
                        let type_name = match type_tok {
                            Token::IntType => "int",
                            Token::CharType => "char",
                            Token::VoidType => "void",
                            Token::LongType => "long",
                            Token::ShortType => "short",
                            Token::DoubleType => "double",
                            Token::FloatType => "float",
                            Token::Bool => "bool",
                            _ => "int",
                        };
                        SizeOfArg::Type(Type::from_c_name(type_name))
                    }
                    _ => {
                        let e = self.parse_expression()?;
                        SizeOfArg::Expr(e)
                    }
                };
                self.expect(Token::RParen)?;
                Ok(Expr::SizeOf(Box::new(arg)))
            }
            Some(Token::LBracket) => {
                // Array literal: [1, 2, 3]
                let mut elements = Vec::new();
                if !matches!(self.peek(), Some(Token::RBracket)) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::Array(elements))
            }
            Some(Token::Identifier(s)) => {
                // Check for Struct::method() or Struct::Trait::method() syntax
                let mut name = s;
                while matches!(self.peek(), Some(Token::DoubleColon)) {
                    self.advance(); // consume ::
                    match self.advance() {
                        Some(Token::Identifier(part)) => {
                            name = format!("{}::{}", name, part);
                        }
                        _ => return Err(ParseError::ExpectedToken("identifier after ::")),
                    }
                }

                if matches!(self.peek(), Some(Token::LParen)) {
                    self.advance();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call { name, args })
                } else if matches!(self.peek(), Some(Token::LBrace)) && Self::is_type_ident(&name) {
                    // Struct literal: Name { field: val, ... }
                    // Only for type-like identifiers (uppercase) to avoid
                    // ambiguity with block-delimited constructs (while x { ... })
                    self.advance(); // consume '{'
                    self.skip_newlines();
                    let mut fields = Vec::new();
                    while !matches!(self.peek(), Some(Token::RBrace) | None) {
                        self.skip_newlines();
                        let field_name = match self.advance() {
                            Some(Token::Identifier(n)) => n,
                            Some(Token::RBrace) => break,
                            Some(tok) => return Err(ParseError::UnexpectedToken(tok)),
                            None => return Err(ParseError::UnexpectedEof),
                        };
                        self.expect(Token::Colon)?;
                        let field_val = self.parse_expression()?;
                        fields.push((field_name, field_val));
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        }
                        self.skip_newlines();
                    }
                    self.expect(Token::RBrace)?;
                    let args = fields.into_iter().map(|(_, v)| v).collect();
                    Ok(Expr::Call {
                        name: format!("__struct__{}", name),
                        args,
                    })
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            Some(Token::LParen) => {
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            // ========== OS-LEVEL EXPRESSIONS (v3.1-OS) ==========
            Some(Token::ReadMemKw) => {
                // read_mem(addr)
                self.expect(Token::LParen)?;
                let addr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expr::MemRead {
                    addr: Box::new(addr),
                })
            }
            Some(Token::PortInKw) => {
                // port_in(port)
                self.expect(Token::LParen)?;
                let port = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expr::PortIn {
                    port: Box::new(port),
                })
            }
            Some(Token::RegKw) => {
                // reg(name) — read register value as expression
                self.expect(Token::LParen)?;
                let reg_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                self.expect(Token::RParen)?;
                Ok(Expr::RegRead { reg_name })
            }
            Some(Token::CpuidKw) => {
                // cpuid as expression
                Ok(Expr::CpuidExpr)
            }
            // ========== LABEL ADDRESS (v3.3-Boot) ==========
            Some(Token::LabelAddrKw) => {
                // label_addr(name) — get absolute address of a label
                self.expect(Token::LParen)?;
                let label_name = match self.advance() {
                    Some(Token::Identifier(n)) => n,
                    Some(token) => return Err(ParseError::UnexpectedToken(token)),
                    None => return Err(ParseError::UnexpectedEof),
                };
                self.expect(Token::RParen)?;
                Ok(Expr::LabelAddr { label_name })
            }
            Some(Token::This) => Ok(Expr::This),
            Some(t) => Err(ParseError::UnexpectedToken(t)),
            None => Err(ParseError::UnexpectedEof),
        }
    }
}
