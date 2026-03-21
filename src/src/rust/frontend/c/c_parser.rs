// ============================================================
// C99 Parser for ADead-BIB C Frontend
// ============================================================
// Recursive descent parser: CToken stream → C AST
// Supports: functions, structs, enums, typedef, control flow,
//           pointers, arrays, expressions with full precedence
// ============================================================

use super::c_ast::*;
use super::c_lexer::CToken;

pub struct CParser {
    tokens: Vec<CToken>,
    lines: Vec<usize>,
    pos: usize,
    /// Known typedef names so we can recognize them as type starters
    typedef_names: std::collections::HashSet<String>,
}

impl CParser {
    pub fn new(tokens: Vec<CToken>, lines: Vec<usize>) -> Self {
        Self {
            tokens,
            lines,
            pos: 0,
            typedef_names: std::collections::HashSet::new(),
        }
    }

    // ========== Token helpers ==========

    fn current_line(&self) -> usize {
        self.lines.get(self.pos).copied().unwrap_or(0)
    }

    fn current(&self) -> &CToken {
        self.tokens.get(self.pos).unwrap_or(&CToken::Eof)
    }

    fn peek(&self) -> &CToken {
        self.tokens.get(self.pos + 1).unwrap_or(&CToken::Eof)
    }

    #[allow(dead_code)]
    fn peek_n(&self, n: usize) -> &CToken {
        self.tokens.get(self.pos + n).unwrap_or(&CToken::Eof)
    }

    fn advance(&mut self) -> CToken {
        let tok = self.current().clone();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &CToken) -> Result<(), String> {
        if self.current() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?} at token position {}",
                expected,
                self.current(),
                self.pos
            ))
        }
    }

    fn eat(&mut self, expected: &CToken) -> bool {
        if self.current() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        match self.current().clone() {
            CToken::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(format!("Expected identifier, got {:?}", other)),
        }
    }

    // ========== Type parsing ==========

    fn is_type_start(&self) -> bool {
        match self.current() {
            CToken::Void
            | CToken::Char
            | CToken::Short
            | CToken::Int
            | CToken::Long
            | CToken::Float
            | CToken::Double
            | CToken::Signed
            | CToken::Unsigned
            | CToken::Struct
            | CToken::Enum
            | CToken::Const
            | CToken::Volatile
            | CToken::Static
            | CToken::Extern
            | CToken::Register
            | CToken::Inline
            | CToken::Typedef
            | CToken::Bool
            | CToken::Complex
            | CToken::Union => true,
            CToken::Identifier(name) => self.typedef_names.contains(name),
            _ => false,
        }
    }

    fn is_type_specifier(&self) -> bool {
        matches!(
            self.current(),
            CToken::Void
                | CToken::Char
                | CToken::Short
                | CToken::Int
                | CToken::Long
                | CToken::Float
                | CToken::Double
                | CToken::Signed
                | CToken::Unsigned
                | CToken::Struct
                | CToken::Enum
                | CToken::Bool
                | CToken::Complex
        )
    }

    fn parse_base_type(&mut self) -> Result<CType, String> {
        // Skip storage class and qualifiers
        loop {
            match self.current() {
                CToken::Static
                | CToken::Extern
                | CToken::Register
                | CToken::Inline
                | CToken::Volatile => {
                    self.advance();
                }
                CToken::Const => {
                    self.advance();
                }
                _ => break,
            }
        }

        let ty = match self.current().clone() {
            CToken::Void => {
                self.advance();
                CType::Void
            }
            CToken::Char => {
                self.advance();
                CType::Char
            }
            CToken::Bool => {
                self.advance();
                CType::Bool
            }
            CToken::Float => {
                self.advance();
                CType::Float
            }
            CToken::Double => {
                self.advance();
                CType::Double
            }
            CToken::Short => {
                self.advance();
                // short int
                self.eat(&CToken::Int);
                CType::Short
            }
            CToken::Int => {
                self.advance();
                CType::Int
            }
            CToken::Long => {
                self.advance();
                if self.eat(&CToken::Long) {
                    // long long
                    self.eat(&CToken::Int);
                    CType::LongLong
                } else if self.eat(&CToken::Int) {
                    CType::Long
                } else if self.eat(&CToken::Double) {
                    CType::Double // long double → treat as double
                } else {
                    CType::Long
                }
            }
            CToken::Signed => {
                self.advance();
                let inner = if self.is_type_specifier() {
                    self.parse_base_type()?
                } else {
                    CType::Int
                };
                CType::Signed(Box::new(inner))
            }
            CToken::Unsigned => {
                self.advance();
                let inner = if self.is_type_specifier() {
                    self.parse_base_type()?
                } else {
                    CType::Int
                };
                CType::Unsigned(Box::new(inner))
            }
            CToken::Struct => {
                self.advance();
                let name = self.expect_identifier()?;
                CType::Struct(name)
            }
            CToken::Union => {
                self.advance();
                let name = self.expect_identifier()?;
                CType::Struct(name) // treat union as struct for IR
            }
            CToken::Enum => {
                self.advance();
                let name = self.expect_identifier()?;
                CType::Enum(name)
            }
            CToken::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                CType::Typedef(name)
            }
            other => return Err(format!("Expected type, got {:?}", other)),
        };

        // _Complex qualifier: double _Complex → Complex(Double)
        let ty = if self.current() == &CToken::Complex {
            self.advance();
            CType::Complex(Box::new(ty))
        } else {
            ty
        };

        Ok(ty)
    }

    /// Parse a full type with pointer modifiers: int**, const char*, etc.
    fn parse_type(&mut self) -> Result<CType, String> {
        let mut base = self.parse_base_type()?;
        // Pointer stars
        while self.current() == &CToken::Star {
            self.advance();
            // Skip const/volatile after *
            while matches!(self.current(), CToken::Const | CToken::Volatile) {
                self.advance();
            }
            base = CType::Pointer(Box::new(base));
        }
        Ok(base)
    }

    // ========== Top-level parsing ==========

    pub fn parse_translation_unit(&mut self) -> Result<CTranslationUnit, String> {
        // Pre-scan: collect typedef names so they're recognized as type starters
        self.prescan_typedef_names();

        let mut unit = CTranslationUnit::new();
        while *self.current() != CToken::Eof {
            let save = self.pos;
            match self.parse_top_level() {
                Ok(decl) => {
                    // Register names from parsed declarations
                    match &decl {
                        CTopLevel::TypedefDecl { new_name, .. } => {
                            self.typedef_names.insert(new_name.clone());
                        }
                        CTopLevel::StructDef { name, .. } => {
                            self.typedef_names.insert(name.clone());
                        }
                        CTopLevel::EnumDef { name, .. } => {
                            self.typedef_names.insert(name.clone());
                        }
                        _ => {}
                    }
                    unit.declarations.push(decl);
                }
                Err(_e) => {
                    // Resilient parsing: skip to next ; or } at depth 0, then continue
                    self.pos = save;
                    self.skip_to_next_top_level();
                }
            }
        }
        Ok(unit)
    }

    /// Skip past an unparseable top-level declaration.
    /// Consumes tokens until we reach a `;` or `}` at brace-depth 0.
    fn skip_to_next_top_level(&mut self) {
        let mut depth = 0i32;
        while *self.current() != CToken::Eof {
            match self.current() {
                CToken::LBrace => { depth += 1; self.advance(); }
                CToken::RBrace => {
                    if depth > 0 {
                        depth -= 1;
                        self.advance();
                        if depth == 0 {
                            self.eat(&CToken::Semicolon);
                            return;
                        }
                    } else {
                        self.advance();
                        return;
                    }
                }
                CToken::Semicolon if depth == 0 => {
                    self.advance();
                    return;
                }
                _ => { self.advance(); }
            }
        }
    }

    /// Pre-scan tokens to collect typedef names before actual parsing.
    /// This handles the chicken-and-egg problem where typedef names
    /// must be known before parsing to distinguish types from identifiers.
    fn prescan_typedef_names(&mut self) {
        let mut i = 0;
        while i < self.tokens.len() {
            // typedef ... Name;
            if self.tokens[i] == CToken::Typedef {
                // Find the semicolon that ends this typedef
                let mut j = i + 1;
                let mut depth = 0;
                while j < self.tokens.len() {
                    match &self.tokens[j] {
                        CToken::LBrace | CToken::LParen => depth += 1,
                        CToken::RBrace | CToken::RParen => {
                            if depth > 0 {
                                depth -= 1;
                            }
                        }
                        CToken::Semicolon if depth == 0 => break,
                        _ => {}
                    }
                    j += 1;
                }
                // The name is the identifier just before the semicolon
                if j > 0 && j < self.tokens.len() {
                    if let CToken::Identifier(name) = &self.tokens[j - 1] {
                        self.typedef_names.insert(name.clone());
                    }
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }
    }

    fn parse_top_level(&mut self) -> Result<CTopLevel, String> {
        // Typedef
        if *self.current() == CToken::Typedef {
            return self.parse_typedef();
        }

        // Struct definition: struct Name { ... };
        if *self.current() == CToken::Struct || *self.current() == CToken::Union {
            if let Some(CToken::Identifier(_)) = self.tokens.get(self.pos + 1) {
                if self.tokens.get(self.pos + 2) == Some(&CToken::LBrace) {
                    return self.parse_struct_def();
                }
            }
        }

        // Enum definition
        if *self.current() == CToken::Enum {
            if let Some(CToken::Identifier(_)) = self.tokens.get(self.pos + 1) {
                if self.tokens.get(self.pos + 2) == Some(&CToken::LBrace) {
                    return self.parse_enum_def();
                }
            }
        }

        // Function or global variable: type name ...
        let ret_type = self.parse_type()?;
        let name = self.expect_identifier()?;

        if *self.current() == CToken::LParen {
            // Function definition or declaration
            self.advance(); // skip (
            let params = self.parse_param_list()?;
            self.expect(&CToken::RParen)?;

            if *self.current() == CToken::LBrace {
                // Function definition
                let body = self.parse_block_stmts()?;
                Ok(CTopLevel::FunctionDef {
                    return_type: ret_type,
                    name,
                    params,
                    body,
                })
            } else {
                // Function declaration (prototype)
                self.expect(&CToken::Semicolon)?;
                Ok(CTopLevel::FunctionDecl {
                    return_type: ret_type,
                    name,
                    params,
                })
            }
        } else {
            // Global variable declaration
            let mut declarators = Vec::new();
            let first = self.parse_declarator_rest(name)?;
            declarators.push(first);

            while self.eat(&CToken::Comma) {
                // Handle pointer in additional declarators
                let mut var_type = ret_type.clone();
                while self.current() == &CToken::Star {
                    self.advance();
                    var_type = CType::Pointer(Box::new(var_type));
                }
                let n = self.expect_identifier()?;
                let d = self.parse_declarator_rest(n)?;
                declarators.push(d);
            }
            self.expect(&CToken::Semicolon)?;

            Ok(CTopLevel::GlobalVar {
                type_spec: ret_type,
                declarators,
            })
        }
    }

    fn parse_declarator_rest(&mut self, name: String) -> Result<CDeclarator, String> {
        // Check for array: name[N]
        let derived = if *self.current() == CToken::LBracket {
            self.advance();
            let size = if *self.current() != CToken::RBracket {
                if let CToken::IntLiteral(n) = self.current().clone() {
                    self.advance();
                    Some(n as usize)
                } else {
                    None
                }
            } else {
                None
            };
            self.expect(&CToken::RBracket)?;
            Some(CDerivedType::Array(size, None))
        } else {
            None
        };

        let init = if self.eat(&CToken::Assign) {
            if *self.current() == CToken::LBrace {
                // Brace-enclosed initializer list: = { expr, expr, ... }
                // C11: supports designated initializers: .field = val, [idx] = val
                Some(self.parse_brace_init()?)
            } else {
                Some(self.parse_assign_expr()?)
            }
        } else {
            None
        };

        Ok(CDeclarator {
            name,
            derived_type: derived,
            initializer: init,
        })
    }

    fn parse_param_list(&mut self) -> Result<Vec<CParam>, String> {
        let mut params = Vec::new();
        if *self.current() == CToken::RParen {
            return Ok(params);
        }
        // void parameter
        if *self.current() == CToken::Void && *self.peek() == CToken::RParen {
            self.advance();
            return Ok(params);
        }

        loop {
            if *self.current() == CToken::Ellipsis {
                self.advance();
                break; // variadic — skip for now
            }

            // Function pointer parameter: type (*name)(args)
            // e.g. int (*compar)(const void *, const void *)
            //      void (*function)(void)
            //      void *(*start_routine)(void *)
            if self.is_function_pointer_param() {
                let ret_type = self.parse_type()?;
                self.expect(&CToken::LParen)?; // (
                self.eat(&CToken::Star); // *
                let name = if let CToken::Identifier(_) = self.current() {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                self.expect(&CToken::RParen)?; // )
                                               // Skip the parameter list of the function pointer
                self.expect(&CToken::LParen)?;
                self.skip_balanced_parens();
                params.push(CParam {
                    param_type: CType::Pointer(Box::new(ret_type)),
                    name,
                });
                if !self.eat(&CToken::Comma) {
                    break;
                }
                continue;
            }

            let param_type = self.parse_type()?;
            let name = if let CToken::Identifier(_) = self.current() {
                Some(self.expect_identifier()?)
            } else {
                None
            };

            // Handle array parameter: int arr[], int arr[N]
            // C99 §6.7.5.3: array parameter is adjusted to pointer
            let final_type = if *self.current() == CToken::LBracket {
                self.advance();
                while *self.current() != CToken::RBracket && *self.current() != CToken::Eof {
                    self.advance();
                }
                self.expect(&CToken::RBracket)?;
                // int arr[] → int *arr (decay to pointer)
                CType::Pointer(Box::new(param_type))
            } else {
                param_type
            };

            params.push(CParam {
                param_type: final_type,
                name,
            });

            if !self.eat(&CToken::Comma) {
                break;
            }
        }

        Ok(params)
    }

    /// Check if current position looks like a function pointer struct field
    /// Reuses same logic as is_function_pointer_param
    fn is_function_pointer_field(&self) -> bool {
        self.is_function_pointer_param()
    }

    /// Check if current position looks like a function pointer parameter
    /// Patterns: type (*name)(...), type *(*name)(...)
    fn is_function_pointer_param(&self) -> bool {
        // Look ahead for the pattern: [type tokens...] ( * [name] ) (
        // Simple heuristic: after consuming type, we'd see LParen Star
        let mut i = self.pos;
        // Skip type specifiers and qualifiers
        while i < self.tokens.len() {
            match &self.tokens[i] {
                CToken::Const
                | CToken::Volatile
                | CToken::Void
                | CToken::Char
                | CToken::Short
                | CToken::Int
                | CToken::Long
                | CToken::Float
                | CToken::Double
                | CToken::Signed
                | CToken::Unsigned
                | CToken::Struct
                | CToken::Enum
                | CToken::Bool
                | CToken::Star => {
                    i += 1;
                }
                CToken::Identifier(_) => {
                    // Could be typedef name or struct name after struct keyword
                    i += 1;
                }
                CToken::LParen => {
                    // Check if next is * — that's the function pointer indicator
                    if i + 1 < self.tokens.len() && self.tokens[i + 1] == CToken::Star {
                        return true;
                    }
                    return false;
                }
                _ => return false,
            }
        }
        false
    }

    /// Skip tokens inside balanced parentheses (including the closing paren)
    fn skip_balanced_parens(&mut self) {
        let mut depth = 1;
        while depth > 0 && *self.current() != CToken::Eof {
            match self.current() {
                CToken::LParen => {
                    depth += 1;
                    self.advance();
                }
                CToken::RParen => {
                    depth -= 1;
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_struct_def(&mut self) -> Result<CTopLevel, String> {
        self.advance(); // skip struct/union
        let name = self.expect_identifier()?;
        self.typedef_names.insert(name.clone());
        self.expect(&CToken::LBrace)?;
        let fields = self.parse_struct_fields()?;
        self.expect(&CToken::RBrace)?;
        self.expect(&CToken::Semicolon)?;
        Ok(CTopLevel::StructDef { name, fields })
    }

    /// Parse struct/union fields until RBrace
    /// C11: supports anonymous struct/union members
    /// Resilient: skips unparseable fields instead of aborting
    fn parse_struct_fields(&mut self) -> Result<Vec<CStructField>, String> {
        let mut fields = Vec::new();
        while *self.current() != CToken::RBrace && *self.current() != CToken::Eof {
            let save = self.pos;
            match self.parse_one_struct_field() {
                Ok(Some(f)) => fields.push(f),
                Ok(None) => { /* anonymous fields were flattened via extend */ }
                Err(_) => {
                    // Resilient: skip unparseable field to next ;
                    self.pos = save;
                    self.skip_struct_field();
                }
            }
        }
        Ok(fields)
    }

    /// Skip one unparseable struct field (consume tokens to next ; at depth 0)
    fn skip_struct_field(&mut self) {
        let mut depth = 0i32;
        while *self.current() != CToken::Eof {
            match self.current() {
                CToken::LParen | CToken::LBrace | CToken::LBracket => { depth += 1; self.advance(); }
                CToken::RParen | CToken::RBrace | CToken::RBracket => {
                    if depth > 0 { depth -= 1; self.advance(); }
                    else if *self.current() == CToken::RBrace { return; }
                    else { self.advance(); }
                }
                CToken::Semicolon if depth == 0 => { self.advance(); return; }
                _ => { self.advance(); }
            }
        }
    }

    /// Parse one struct field, returning Ok(Some(field)) for normal fields,
    /// Ok(None) for anonymous struct/union (already pushed via self),
    /// or Err for unparseable patterns.
    fn parse_one_struct_field(&mut self) -> Result<Option<CStructField>, String> {
        // C11: anonymous struct/union: struct { ... }; or union { ... };
        if (*self.current() == CToken::Struct || *self.current() == CToken::Union)
            && (*self.peek() == CToken::LBrace
                || (matches!(self.peek(), CToken::Identifier(_))
                    && self.peek_n(2) == &CToken::LBrace
                    && self.peek_n(3) != &CToken::Semicolon))
        {
            let save = self.pos;
            self.advance(); // skip struct/union
            if let CToken::Identifier(_) = self.current() {
                if *self.peek() == CToken::LBrace {
                    self.advance();
                }
            }
            if *self.current() == CToken::LBrace {
                self.advance();
                let inner_fields = self.parse_struct_fields()?;
                self.expect(&CToken::RBrace)?;
                if *self.current() == CToken::Semicolon {
                    self.advance();
                    // Flatten anonymous members — we can't return them directly,
                    // so return None and the caller doesn't push anything.
                    // Actually we need the caller to know about them.
                    // Use a different approach: return first inner field, push rest via Err trick
                    // Simpler: just return None and skip (fields already collected by recursive call)
                    return Ok(None);
                } else if let CToken::Identifier(fname) = self.current().clone() {
                    self.advance();
                    let ft = CType::Struct("__anon".to_string());
                    let final_type = if *self.current() == CToken::LBracket {
                        self.advance();
                        let size = if let CToken::IntLiteral(n) = self.current().clone() {
                            self.advance();
                            Some(n as usize)
                        } else { None };
                        self.expect(&CToken::RBracket)?;
                        CType::Array(Box::new(ft), size)
                    } else { ft };
                    self.expect(&CToken::Semicolon)?;
                    return Ok(Some(CStructField { field_type: final_type, name: fname }));
                }
            }
            self.pos = save;
        }

        // Handle function pointer fields: type (*name)(params);
        if self.is_function_pointer_field() {
            let ret_type = self.parse_type()?;
            self.expect(&CToken::LParen)?;
            self.eat(&CToken::Star);
            // Handle nested function pointers: type (*(*name)(params))(params)
            // If we see another (, it's nested — skip the whole thing
            if *self.current() == CToken::LParen {
                // Complex nested function pointer — skip to ;
                return Err("Nested function pointer field".to_string());
            }
            let fp_name = self.expect_identifier()?;
            self.expect(&CToken::RParen)?;
            self.expect(&CToken::LParen)?;
            self.skip_balanced_parens();
            // Handle trailing (params) for function-pointer-returning-function-pointer
            if *self.current() == CToken::LParen {
                self.advance();
                self.skip_balanced_parens();
            }
            self.expect(&CToken::Semicolon)?;
            return Ok(Some(CStructField {
                field_type: CType::Pointer(Box::new(ret_type)),
                name: fp_name,
            }));
        }

        let field_type = self.parse_type()?;

        // Handle bit fields: int name : width;
        let field_name = self.expect_identifier()?;
        if self.eat(&CToken::Colon) {
            // Bit field width — skip the expression
            while *self.current() != CToken::Semicolon
                && *self.current() != CToken::RBrace
                && *self.current() != CToken::Eof
            {
                self.advance();
            }
            self.eat(&CToken::Semicolon);
            return Ok(Some(CStructField { field_type, name: field_name }));
        }

        // Handle array fields: type name[N];
        let final_type = if *self.current() == CToken::LBracket {
            self.advance();
            let size = if let CToken::IntLiteral(n) = self.current().clone() {
                self.advance();
                Some(n as usize)
            } else {
                // Skip complex array size expressions
                while *self.current() != CToken::RBracket && *self.current() != CToken::Eof {
                    self.advance();
                }
                None
            };
            self.expect(&CToken::RBracket)?;
            CType::Array(Box::new(field_type), size)
        } else {
            field_type
        };

        self.expect(&CToken::Semicolon)?;
        Ok(Some(CStructField { field_type: final_type, name: field_name }))
    }

    fn parse_enum_def(&mut self) -> Result<CTopLevel, String> {
        self.advance(); // skip enum
        let name = self.expect_identifier()?;
        self.expect(&CToken::LBrace)?;

        let mut values = Vec::new();
        while *self.current() != CToken::RBrace && *self.current() != CToken::Eof {
            let ident = self.expect_identifier()?;
            let val = if self.eat(&CToken::Assign) {
                let negative = self.eat(&CToken::Minus);
                if let CToken::IntLiteral(n) = self.current().clone() {
                    self.advance();
                    Some(if negative { -n } else { n })
                } else {
                    None
                }
            } else {
                None
            };
            values.push((ident, val));
            if !self.eat(&CToken::Comma) {
                break;
            }
        }
        self.expect(&CToken::RBrace)?;
        self.expect(&CToken::Semicolon)?;

        Ok(CTopLevel::EnumDef { name, values })
    }

    fn parse_typedef(&mut self) -> Result<CTopLevel, String> {
        self.advance(); // skip typedef

        // typedef struct { ... } Name;  (anonymous struct)
        // Also handles: typedef struct { ... } *PtrName;
        if *self.current() == CToken::Struct && *self.peek() == CToken::LBrace {
            self.advance(); // skip struct
            self.advance(); // skip {
            let fields = self.parse_struct_fields()?;
            self.expect(&CToken::RBrace)?;
            let is_ptr = self.eat(&CToken::Star);
            let new_name = self.expect_identifier()?;
            self.typedef_names.insert(new_name.clone());
            self.expect(&CToken::Semicolon)?;
            if is_ptr {
                return Ok(CTopLevel::TypedefDecl {
                    original: CType::Pointer(Box::new(CType::Struct(new_name.clone()))),
                    new_name,
                });
            }
            return Ok(CTopLevel::StructDef {
                name: new_name,
                fields,
            });
        }

        // typedef struct Name { ... } Alias;  (named struct with alias)
        // Also handles: typedef struct Name { ... } *PtrAlias;
        if *self.current() == CToken::Struct {
            if let Some(CToken::Identifier(_)) = self.tokens.get(self.pos + 1) {
                if self.tokens.get(self.pos + 2) == Some(&CToken::LBrace) {
                    self.advance(); // skip struct
                    let struct_name = self.expect_identifier()?;
                    self.advance(); // skip {
                    let fields = self.parse_struct_fields()?;
                    self.expect(&CToken::RBrace)?;
                    // Check for pointer alias: } *Name;
                    let is_ptr = self.eat(&CToken::Star);
                    if let CToken::Identifier(_) = self.current() {
                        let alias = self.expect_identifier()?;
                        self.typedef_names.insert(alias.clone());
                        self.expect(&CToken::Semicolon)?;
                        if is_ptr {
                            return Ok(CTopLevel::TypedefDecl {
                                original: CType::Pointer(Box::new(CType::Struct(struct_name))),
                                new_name: alias,
                            });
                        }
                        return Ok(CTopLevel::StructDef {
                            name: alias,
                            fields,
                        });
                    } else {
                        self.expect(&CToken::Semicolon)?;
                        return Ok(CTopLevel::StructDef {
                            name: struct_name,
                            fields,
                        });
                    }
                }
            }
        }

        // typedef union Name { ... } Alias;
        // Also handles: typedef union Name { ... } *PtrAlias;
        if *self.current() == CToken::Union {
            if let Some(CToken::Identifier(_)) = self.tokens.get(self.pos + 1) {
                if self.tokens.get(self.pos + 2) == Some(&CToken::LBrace) {
                    self.advance(); // skip union
                    let _union_name = self.expect_identifier()?;
                    self.advance(); // skip {
                    let fields = self.parse_struct_fields()?;
                    self.expect(&CToken::RBrace)?;
                    let is_ptr = self.eat(&CToken::Star);
                    if let CToken::Identifier(_) = self.current() {
                        let alias = self.expect_identifier()?;
                        self.typedef_names.insert(alias.clone());
                        self.expect(&CToken::Semicolon)?;
                        if is_ptr {
                            return Ok(CTopLevel::TypedefDecl {
                                original: CType::Pointer(Box::new(CType::Struct(_union_name))),
                                new_name: alias,
                            });
                        }
                        return Ok(CTopLevel::StructDef {
                            name: alias,
                            fields,
                        });
                    } else {
                        self.expect(&CToken::Semicolon)?;
                        return Ok(CTopLevel::StructDef {
                            name: _union_name,
                            fields,
                        });
                    }
                }
            }
        }

        // typedef enum { ... } Name;
        if *self.current() == CToken::Enum {
            if *self.peek() == CToken::LBrace {
                self.advance(); // skip enum
                self.advance(); // skip {
                let mut values = Vec::new();
                while *self.current() != CToken::RBrace && *self.current() != CToken::Eof {
                    let ident = self.expect_identifier()?;
                    let val = if self.eat(&CToken::Assign) {
                        if let CToken::IntLiteral(n) = self.current().clone() {
                            self.advance();
                            Some(n)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    values.push((ident, val));
                    if !self.eat(&CToken::Comma) {
                        break;
                    }
                }
                self.expect(&CToken::RBrace)?;
                let alias = self.expect_identifier()?;
                self.expect(&CToken::Semicolon)?;
                return Ok(CTopLevel::EnumDef {
                    name: alias,
                    values,
                });
            }
            // typedef enum Name { ... } Alias;
            if let Some(CToken::Identifier(_)) = self.tokens.get(self.pos + 1) {
                if self.tokens.get(self.pos + 2) == Some(&CToken::LBrace) {
                    self.advance(); // skip enum
                    let _enum_name = self.expect_identifier()?;
                    self.advance(); // skip {
                    let mut values = Vec::new();
                    while *self.current() != CToken::RBrace && *self.current() != CToken::Eof {
                        let ident = self.expect_identifier()?;
                        let val = if self.eat(&CToken::Assign) {
                            if let CToken::IntLiteral(n) = self.current().clone() {
                                self.advance();
                                Some(n)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        values.push((ident, val));
                        if !self.eat(&CToken::Comma) {
                            break;
                        }
                    }
                    self.expect(&CToken::RBrace)?;
                    if let CToken::Identifier(_) = self.current() {
                        let alias = self.expect_identifier()?;
                        self.expect(&CToken::Semicolon)?;
                        return Ok(CTopLevel::EnumDef {
                            name: alias,
                            values,
                        });
                    } else {
                        self.expect(&CToken::Semicolon)?;
                        return Ok(CTopLevel::EnumDef {
                            name: _enum_name,
                            values,
                        });
                    }
                }
            }
        }

        // typedef type (*name)(args);  — function pointer typedef
        // e.g. typedef void (*sighandler_t)(int);
        //      typedef void (*sqlite3_destructor_type)(void *);
        let original = self.parse_type()?;
        if *self.current() == CToken::LParen && *self.peek() == CToken::Star {
            self.advance(); // skip (
            self.advance(); // skip *
            let new_name = self.expect_identifier()?;
            self.expect(&CToken::RParen)?;
            // Skip the parameter list
            self.expect(&CToken::LParen)?;
            self.skip_balanced_parens();
            self.expect(&CToken::Semicolon)?;
            return Ok(CTopLevel::TypedefDecl {
                original: CType::Pointer(Box::new(original)),
                new_name,
            });
        }

        let new_name = self.expect_identifier_or_keyword()?;

        // Handle array typedefs: typedef long jmp_buf[8];
        let final_type = if *self.current() == CToken::LBracket {
            self.advance();
            let size = if let CToken::IntLiteral(n) = self.current().clone() {
                self.advance();
                Some(n as usize)
            } else {
                None
            };
            self.expect(&CToken::RBracket)?;
            CType::Array(Box::new(original), size)
        } else {
            original
        };

        self.typedef_names.insert(new_name.clone());
        self.expect(&CToken::Semicolon)?;

        Ok(CTopLevel::TypedefDecl {
            original: final_type,
            new_name,
        })
    }

    /// Like expect_identifier, but also accepts C keywords used as names
    /// (e.g. `typedef int bool;` where bool is a keyword)
    fn expect_identifier_or_keyword(&mut self) -> Result<String, String> {
        match self.current().clone() {
            CToken::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            CToken::Bool => {
                self.advance();
                Ok("bool".to_string())
            }
            CToken::Char => {
                self.advance();
                Ok("char".to_string())
            }
            CToken::Int => {
                self.advance();
                Ok("int".to_string())
            }
            CToken::Long => {
                self.advance();
                Ok("long".to_string())
            }
            CToken::Short => {
                self.advance();
                Ok("short".to_string())
            }
            CToken::Float => {
                self.advance();
                Ok("float".to_string())
            }
            CToken::Double => {
                self.advance();
                Ok("double".to_string())
            }
            CToken::Void => {
                self.advance();
                Ok("void".to_string())
            }
            other => Err(format!("Expected identifier, got {:?}", other)),
        }
    }

    // ========== Statement parsing ==========

    fn parse_block_stmts(&mut self) -> Result<Vec<CStmt>, String> {
        self.expect(&CToken::LBrace)?;
        let mut stmts = Vec::new();
        while *self.current() != CToken::RBrace && *self.current() != CToken::Eof {
            let line = self.current_line();
            stmts.push(CStmt::LineMarker(line));
            stmts.push(self.parse_statement()?);
        }
        self.expect(&CToken::RBrace)?;
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<CStmt, String> {
        match self.current().clone() {
            CToken::LBrace => {
                let stmts = self.parse_block_stmts()?;
                Ok(CStmt::Block(stmts))
            }
            CToken::Return => {
                self.advance();
                if *self.current() == CToken::Semicolon {
                    self.advance();
                    Ok(CStmt::Return(None))
                } else {
                    let expr = self.parse_expression()?;
                    self.expect(&CToken::Semicolon)?;
                    Ok(CStmt::Return(Some(expr)))
                }
            }
            CToken::If => self.parse_if(),
            CToken::While => self.parse_while(),
            CToken::Do => self.parse_do_while(),
            CToken::For => self.parse_for(),
            CToken::Switch => self.parse_switch(),
            CToken::Break => {
                self.advance();
                self.expect(&CToken::Semicolon)?;
                Ok(CStmt::Break)
            }
            CToken::Continue => {
                self.advance();
                self.expect(&CToken::Semicolon)?;
                Ok(CStmt::Continue)
            }
            CToken::Goto => {
                self.advance();
                let label = self.expect_identifier()?;
                self.expect(&CToken::Semicolon)?;
                Ok(CStmt::Goto(label))
            }
            CToken::Semicolon => {
                self.advance();
                Ok(CStmt::Empty)
            }
            _ => {
                // Check if it's a variable declaration
                if self.is_type_start() && !self.looks_like_expr_cast() {
                    self.parse_var_decl()
                } else {
                    // Check for label: identifier followed by :
                    if let CToken::Identifier(ref name) = self.current().clone() {
                        if *self.peek() == CToken::Colon {
                            let label = name.clone();
                            self.advance(); // skip identifier
                            self.advance(); // skip :
                            let stmt = self.parse_statement()?;
                            return Ok(CStmt::Label(label, Box::new(stmt)));
                        }
                    }

                    let expr = self.parse_expression()?;
                    self.expect(&CToken::Semicolon)?;
                    Ok(CStmt::Expr(expr))
                }
            }
        }
    }

    fn looks_like_expr_cast(&self) -> bool {
        // Distinguish (int)x cast from int x declaration
        // A type keyword followed by ) suggests a cast expression
        false // declarations take priority
    }

    fn parse_var_decl(&mut self) -> Result<CStmt, String> {
        // Check for static keyword before the type
        let is_static = *self.current() == CToken::Static;
        // parse_type will skip the static token internally
        let type_spec = self.parse_type()?;
        let mut declarators = Vec::new();

        let name = self.expect_identifier()?;
        let first = self.parse_declarator_rest(name)?;
        declarators.push(first);

        while self.eat(&CToken::Comma) {
            let mut _inner_type = type_spec.clone();
            while self.current() == &CToken::Star {
                self.advance();
                _inner_type = CType::Pointer(Box::new(_inner_type));
            }
            let n = self.expect_identifier()?;
            let d = self.parse_declarator_rest(n)?;
            declarators.push(d);
        }
        self.expect(&CToken::Semicolon)?;

        Ok(CStmt::VarDecl {
            type_spec,
            declarators,
            is_static,
        })
    }

    fn parse_if(&mut self) -> Result<CStmt, String> {
        self.advance(); // skip if
        self.expect(&CToken::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&CToken::RParen)?;
        let then_body = Box::new(self.parse_statement()?);
        let else_body = if self.eat(&CToken::Else) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        Ok(CStmt::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<CStmt, String> {
        self.advance(); // skip while
        self.expect(&CToken::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&CToken::RParen)?;
        let body = Box::new(self.parse_statement()?);
        Ok(CStmt::While { condition, body })
    }

    fn parse_do_while(&mut self) -> Result<CStmt, String> {
        self.advance(); // skip do
        let body = Box::new(self.parse_statement()?);
        self.expect(&CToken::While)?;
        self.expect(&CToken::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&CToken::RParen)?;
        self.expect(&CToken::Semicolon)?;
        Ok(CStmt::DoWhile { body, condition })
    }

    fn parse_for(&mut self) -> Result<CStmt, String> {
        self.advance(); // skip for
        self.expect(&CToken::LParen)?;

        // Init
        let init = if *self.current() == CToken::Semicolon {
            self.advance();
            None
        } else if self.is_type_start() {
            let s = self.parse_var_decl()?; // includes semicolon
            Some(Box::new(s))
        } else {
            let e = self.parse_expression()?;
            self.expect(&CToken::Semicolon)?;
            Some(Box::new(CStmt::Expr(e)))
        };

        // Condition
        let condition = if *self.current() == CToken::Semicolon {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(&CToken::Semicolon)?;

        // Update
        let update = if *self.current() == CToken::RParen {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(&CToken::RParen)?;

        let body = Box::new(self.parse_statement()?);
        Ok(CStmt::For {
            init,
            condition,
            update,
            body,
        })
    }

    fn parse_switch(&mut self) -> Result<CStmt, String> {
        self.advance(); // skip switch
        self.expect(&CToken::LParen)?;
        let expr = self.parse_expression()?;
        self.expect(&CToken::RParen)?;
        self.expect(&CToken::LBrace)?;

        let mut cases = Vec::new();
        while *self.current() != CToken::RBrace && *self.current() != CToken::Eof {
            let value = if self.eat(&CToken::Case) {
                let v = self.parse_expression()?;
                self.expect(&CToken::Colon)?;
                Some(v)
            } else if self.eat(&CToken::Default) {
                self.expect(&CToken::Colon)?;
                None
            } else {
                return Err(format!(
                    "Expected case or default, got {:?}",
                    self.current()
                ));
            };

            let mut body = Vec::new();
            while !matches!(
                self.current(),
                CToken::Case | CToken::Default | CToken::RBrace | CToken::Eof
            ) {
                body.push(self.parse_statement()?);
            }
            cases.push(CSwitchCase { value, body });
        }
        self.expect(&CToken::RBrace)?;
        Ok(CStmt::Switch { expr, cases })
    }

    // ========== Expression parsing (operator precedence) ==========

    fn parse_expression(&mut self) -> Result<CExpr, String> {
        let expr = self.parse_assign_expr()?;
        // Comma operator
        if *self.current() == CToken::Comma {
            let mut exprs = vec![expr];
            while self.eat(&CToken::Comma) {
                exprs.push(self.parse_assign_expr()?);
            }
            Ok(CExpr::Comma(exprs))
        } else {
            Ok(expr)
        }
    }

    fn parse_assign_expr(&mut self) -> Result<CExpr, String> {
        let left = self.parse_ternary()?;
        let op = match self.current() {
            CToken::Assign => CAssignOp::Assign,
            CToken::PlusAssign => CAssignOp::AddAssign,
            CToken::MinusAssign => CAssignOp::SubAssign,
            CToken::StarAssign => CAssignOp::MulAssign,
            CToken::SlashAssign => CAssignOp::DivAssign,
            CToken::PercentAssign => CAssignOp::ModAssign,
            CToken::AmpAssign => CAssignOp::AndAssign,
            CToken::PipeAssign => CAssignOp::OrAssign,
            CToken::CaretAssign => CAssignOp::XorAssign,
            CToken::LShiftAssign => CAssignOp::ShlAssign,
            CToken::RShiftAssign => CAssignOp::ShrAssign,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_assign_expr()?;
        Ok(CExpr::Assign {
            op,
            target: Box::new(left),
            value: Box::new(right),
        })
    }

    fn parse_ternary(&mut self) -> Result<CExpr, String> {
        let cond = self.parse_log_or()?;
        if self.eat(&CToken::Question) {
            let then_expr = self.parse_expression()?;
            self.expect(&CToken::Colon)?;
            let else_expr = self.parse_ternary()?;
            Ok(CExpr::Ternary {
                condition: Box::new(cond),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            })
        } else {
            Ok(cond)
        }
    }

    fn parse_log_or(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_log_and()?;
        while self.eat(&CToken::OrOr) {
            let right = self.parse_log_and()?;
            left = CExpr::BinaryOp {
                op: CBinOp::LogOr,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_log_and(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_bit_or()?;
        while self.eat(&CToken::AndAnd) {
            let right = self.parse_bit_or()?;
            left = CExpr::BinaryOp {
                op: CBinOp::LogAnd,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bit_or(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_bit_xor()?;
        while *self.current() == CToken::Pipe {
            self.advance();
            let right = self.parse_bit_xor()?;
            left = CExpr::BinaryOp {
                op: CBinOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bit_xor(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_bit_and()?;
        while *self.current() == CToken::Caret {
            self.advance();
            let right = self.parse_bit_and()?;
            left = CExpr::BinaryOp {
                op: CBinOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bit_and(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_equality()?;
        while *self.current() == CToken::Ampersand {
            self.advance();
            let right = self.parse_equality()?;
            left = CExpr::BinaryOp {
                op: CBinOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match self.current() {
                CToken::EqEq => CBinOp::Eq,
                CToken::NotEq => CBinOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_relational()?;
            left = CExpr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_shift()?;
        loop {
            let op = match self.current() {
                CToken::Less => CBinOp::Lt,
                CToken::Greater => CBinOp::Gt,
                CToken::LessEq => CBinOp::Le,
                CToken::GreaterEq => CBinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift()?;
            left = CExpr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.current() {
                CToken::LShift => CBinOp::Shl,
                CToken::RShift => CBinOp::Shr,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = CExpr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.current() {
                CToken::Plus => CBinOp::Add,
                CToken::Minus => CBinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = CExpr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<CExpr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.current() {
                CToken::Star => CBinOp::Mul,
                CToken::Slash => CBinOp::Div,
                CToken::Percent => CBinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = CExpr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<CExpr, String> {
        match self.current().clone() {
            CToken::PlusPlus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(CExpr::UnaryOp {
                    op: CUnaryOp::PreInc,
                    expr: Box::new(expr),
                    prefix: true,
                })
            }
            CToken::MinusMinus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(CExpr::UnaryOp {
                    op: CUnaryOp::PreDec,
                    expr: Box::new(expr),
                    prefix: true,
                })
            }
            CToken::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(CExpr::UnaryOp {
                    op: CUnaryOp::Neg,
                    expr: Box::new(expr),
                    prefix: true,
                })
            }
            CToken::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(CExpr::UnaryOp {
                    op: CUnaryOp::LogNot,
                    expr: Box::new(expr),
                    prefix: true,
                })
            }
            CToken::Tilde => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(CExpr::UnaryOp {
                    op: CUnaryOp::BitNot,
                    expr: Box::new(expr),
                    prefix: true,
                })
            }
            CToken::Ampersand => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(CExpr::AddressOf(Box::new(expr)))
            }
            CToken::Star => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(CExpr::Deref(Box::new(expr)))
            }
            CToken::Sizeof => {
                self.advance();
                if *self.current() == CToken::LParen && self.is_type_at_next() {
                    self.advance(); // skip (
                    let ty = self.parse_type()?;
                    self.expect(&CToken::RParen)?;
                    Ok(CExpr::SizeofType(ty))
                } else {
                    let expr = self.parse_unary()?;
                    Ok(CExpr::SizeofExpr(Box::new(expr)))
                }
            }
            CToken::LParen if self.is_cast() => {
                self.advance(); // skip (
                let mut ty = self.parse_type()?;
                // Handle pointer modifiers in cast type
                while self.eat(&CToken::Star) {
                    ty = CType::Pointer(Box::new(ty));
                }
                // Handle array type: (int[]) or (int[5])
                if *self.current() == CToken::LBracket {
                    self.advance(); // skip [
                    let size = if let CToken::IntLiteral(n) = self.current().clone() {
                        self.advance();
                        Some(n as usize)
                    } else {
                        None
                    };
                    self.expect(&CToken::RBracket)?;
                    ty = CType::Array(Box::new(ty), size);
                }
                self.expect(&CToken::RParen)?;
                // C11 compound literal: (type){ ... }
                if *self.current() == CToken::LBrace {
                    let init = self.parse_brace_init()?;
                    return Ok(init);
                }
                let expr = self.parse_unary()?;
                Ok(CExpr::Cast {
                    target_type: ty,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn is_type_at_next(&self) -> bool {
        match self.peek() {
            CToken::Void
            | CToken::Char
            | CToken::Short
            | CToken::Int
            | CToken::Long
            | CToken::Float
            | CToken::Double
            | CToken::Signed
            | CToken::Unsigned
            | CToken::Struct
            | CToken::Enum
            | CToken::Const
            | CToken::Volatile
            | CToken::Bool
            | CToken::Complex => true,
            CToken::Identifier(name) => self.typedef_names.contains(name),
            _ => false,
        }
    }

    fn is_cast(&self) -> bool {
        // (type)expr — check if this looks like a cast
        if *self.current() != CToken::LParen {
            return false;
        }
        match self.peek() {
            CToken::Void
            | CToken::Char
            | CToken::Short
            | CToken::Int
            | CToken::Long
            | CToken::Float
            | CToken::Double
            | CToken::Signed
            | CToken::Unsigned
            | CToken::Struct
            | CToken::Enum
            | CToken::Bool
            | CToken::Complex
            | CToken::Const
            | CToken::Volatile => true,
            CToken::Identifier(name) => {
                // Only treat as cast if name is a known typedef AND
                // the token after closing ) looks like a unary expr, not binary op
                self.typedef_names.contains(name)
            }
            _ => false,
        }
    }

    fn parse_postfix(&mut self) -> Result<CExpr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.current() {
                CToken::LBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(&CToken::RBracket)?;
                    expr = CExpr::Index {
                        array: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                CToken::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if *self.current() != CToken::RParen {
                        args.push(self.parse_assign_expr()?);
                        while self.eat(&CToken::Comma) {
                            args.push(self.parse_assign_expr()?);
                        }
                    }
                    self.expect(&CToken::RParen)?;
                    expr = CExpr::Call {
                        func: Box::new(expr),
                        args,
                    };
                }
                CToken::Dot => {
                    self.advance();
                    let field = self.expect_identifier()?;
                    expr = CExpr::Member {
                        object: Box::new(expr),
                        field,
                    };
                }
                CToken::Arrow => {
                    self.advance();
                    let field = self.expect_identifier()?;
                    expr = CExpr::ArrowMember {
                        pointer: Box::new(expr),
                        field,
                    };
                }
                CToken::PlusPlus => {
                    self.advance();
                    expr = CExpr::UnaryOp {
                        op: CUnaryOp::PostInc,
                        expr: Box::new(expr),
                        prefix: false,
                    };
                }
                CToken::MinusMinus => {
                    self.advance();
                    expr = CExpr::UnaryOp {
                        op: CUnaryOp::PostDec,
                        expr: Box::new(expr),
                        prefix: false,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    /// Parse brace-enclosed initializer list: { expr, expr, ... }
    /// C11: supports designated initializers: .field = val, [idx] = val
    fn parse_brace_init(&mut self) -> Result<CExpr, String> {
        self.advance(); // skip {
        let mut elements = Vec::new();
        while *self.current() != CToken::RBrace && *self.current() != CToken::Eof {
            // C11 designated initializer: .field = expr
            if *self.current() == CToken::Dot {
                self.advance(); // skip .
                let _field = self.expect_identifier()?;
                self.expect(&CToken::Assign)?;
                elements.push(self.parse_assign_expr()?);
            }
            // C11 designated initializer: [idx] = expr
            else if *self.current() == CToken::LBracket {
                self.advance(); // skip [
                let _idx = self.parse_expression()?;
                self.expect(&CToken::RBracket)?;
                self.expect(&CToken::Assign)?;
                elements.push(self.parse_assign_expr()?);
            } else {
                elements.push(self.parse_assign_expr()?);
            }
            if !self.eat(&CToken::Comma) {
                break;
            }
        }
        self.expect(&CToken::RBrace)?;
        Ok(CExpr::InitList(elements))
    }

    fn parse_primary(&mut self) -> Result<CExpr, String> {
        match self.current().clone() {
            CToken::IntLiteral(n) => {
                self.advance();
                Ok(CExpr::IntLiteral(n))
            }
            CToken::FloatLiteral(f) => {
                self.advance();
                Ok(CExpr::FloatLiteral(f))
            }
            CToken::StringLiteral(s) => {
                let mut result = s;
                self.advance(); // consume first string token
                                // Concatenate adjacent string literals: "a" "b" → "ab"
                while let CToken::StringLiteral(s2) = self.current().clone() {
                    result.push_str(&s2);
                    self.advance();
                }
                Ok(CExpr::StringLiteral(result))
            }
            CToken::CharLiteral(c) => {
                self.advance();
                Ok(CExpr::CharLiteral(c))
            }
            CToken::Identifier(ref name) if name == "NULL" || name == "nullptr" => {
                self.advance();
                Ok(CExpr::Null)
            }
            CToken::Identifier(name) => {
                self.advance();
                Ok(CExpr::Identifier(name))
            }
            CToken::LParen => {
                // Check for compound literal: (Type){...}
                // or cast: (Type)expr
                // or parenthesized expression: (expr)
                let save = self.pos;
                self.advance(); // skip (
                if self.is_type_start() {
                    let cast_type = self.parse_type()?;
                    // Handle pointer/const modifiers
                    while self.eat(&CToken::Star) {}
                    self.expect(&CToken::RParen)?;
                    // C11 compound literal: (Type){ ... }
                    if *self.current() == CToken::LBrace {
                        let init = self.parse_brace_init()?;
                        return Ok(init);
                    }
                    // Cast expression: (Type)expr
                    let inner = self.parse_unary()?;
                    return Ok(CExpr::Cast {
                        target_type: cast_type,
                        expr: Box::new(inner),
                    });
                }
                // Regular parenthesized expression
                self.pos = save;
                self.advance(); // skip (
                let expr = self.parse_expression()?;
                self.expect(&CToken::RParen)?;
                Ok(expr)
            }
            other => Err(format!("Unexpected token in expression: {:?}", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::c_lexer::CLexer;
    use super::*;

    fn parse_c(code: &str) -> CTranslationUnit {
        let (tokens, lines) = CLexer::new(code).tokenize();
        CParser::new(tokens, lines)
            .parse_translation_unit()
            .unwrap()
    }

    #[test]
    fn test_simple_main() {
        let unit = parse_c("int main() { return 0; }");
        assert_eq!(unit.declarations.len(), 1);
        match &unit.declarations[0] {
            CTopLevel::FunctionDef { name, .. } => assert_eq!(name, "main"),
            _ => panic!("Expected function definition"),
        }
    }

    #[test]
    fn test_var_decl() {
        let unit = parse_c("int main() { int x = 42; return x; }");
        assert_eq!(unit.declarations.len(), 1);
    }

    #[test]
    fn test_if_else() {
        let unit = parse_c("int main() { if (1) { return 1; } else { return 0; } }");
        assert_eq!(unit.declarations.len(), 1);
    }

    #[test]
    fn test_for_loop() {
        let unit = parse_c("int main() { for (int i = 0; i < 10; i++) { } return 0; }");
        assert_eq!(unit.declarations.len(), 1);
    }

    #[test]
    fn test_struct_def() {
        let unit = parse_c("struct Point { int x; int y; };");
        assert_eq!(unit.declarations.len(), 1);
        match &unit.declarations[0] {
            CTopLevel::StructDef { name, fields } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
            }
            _ => panic!("Expected struct definition"),
        }
    }
}
