// ============================================================
// C99 Lexer for ADead-BIB C Frontend
// ============================================================
// Tokenizes C source code into CToken stream
// Pure Rust — no external dependencies
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum CToken {
    // C Keywords
    Auto,
    Break,
    Case,
    Char,
    Const,
    Continue,
    Default,
    Do,
    Double,
    Else,
    Enum,
    Extern,
    Float,
    For,
    Goto,
    If,
    Inline,
    Int,
    Long,
    Register,
    Restrict,
    Return,
    Short,
    Signed,
    Sizeof,
    Static,
    Struct,
    Switch,
    Typedef,
    Union,
    Unsigned,
    Void,
    Volatile,
    While,
    Bool, // _Bool / bool
    Complex, // _Complex

    // Identifiers and literals
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    Bang,
    Question,
    Assign,        // =
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=
    AmpAssign,     // &=
    PipeAssign,    // |=
    CaretAssign,   // ^=
    LShiftAssign,  // <<=
    RShiftAssign,  // >>=
    EqEq,          // ==
    NotEq,         // !=
    Less,
    Greater,
    LessEq,
    GreaterEq,
    AndAnd,     // &&
    OrOr,       // ||
    LShift,     // <<
    RShift,     // >>
    PlusPlus,   // ++
    MinusMinus, // --
    Arrow,      // ->
    Dot,        // .

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Colon,
    Ellipsis,

    // EOF
    Eof,
}

pub struct CLexer {
    input: Vec<char>,
    position: usize,
    current_char: Option<char>,
    pub line: usize,
    pub column: usize,
    pub token_start_line: usize,
}

impl CLexer {
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current = chars.first().copied();
        Self {
            input: chars,
            position: 0,
            current_char: current,
            line: 1,
            column: 1,
            token_start_line: 1,
        }
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current_char {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        self.position += 1;
        self.current_char = self.input.get(self.position).copied();
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.current_char {
            if ch == '\n' {
                self.advance();
                break;
            }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) {
        self.advance(); // skip *
        loop {
            match self.current_char {
                None => break,
                Some('*') => {
                    self.advance();
                    if self.current_char == Some('/') {
                        self.advance();
                        break;
                    }
                }
                _ => self.advance(),
            }
        }
    }

    fn skip_preprocessor_line(&mut self) {
        let mut line_str = String::new();
        // Skip # and entire line
        while let Some(ch) = self.current_char {
            if ch == '\n' {
                // Check if this was a line marker from gcc (e.g. `# 12 "file.c"`)
                let parts: Vec<&str> = line_str.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(num) = parts[1].parse::<usize>() {
                        // The preprocessor tells us the NEXT line is `num`
                        // Since `self.advance()` right below will consume the `\n` and increment `self.line` by 1
                        // we set `self.line` to `num - 1` so it becomes exactly `num`.
                        self.line = num.saturating_sub(1);
                    }
                }
                
                self.advance();
                break;
            }
            // Handle line continuation with backslash
            if ch == '\\' {
                self.advance();
                if self.current_char == Some('\n') {
                    self.advance();
                    continue;
                }
            }
            line_str.push(ch);
            self.advance();
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }

    fn read_number(&mut self) -> CToken {
        let mut num_str = String::new();
        let mut is_float = false;
        // Check for hex: 0x or 0X
        if self.current_char == Some('0') {
            if let Some(next) = self.peek() {
                if next == 'x' || next == 'X' {
                    self.advance(); // skip 0
                    self.advance(); // skip x
                    while let Some(ch) = self.current_char {
                        if ch.is_ascii_hexdigit() {
                            num_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    // Skip suffixes: U, L, UL, ULL, etc.
                    self.skip_int_suffix();
                    let val = i64::from_str_radix(&num_str, 16).unwrap_or(0);
                    return CToken::IntLiteral(val);
                }
            }
        }

        // Decimal or octal
        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !is_float {
                // Check it's not .. or method call
                if let Some(next) = self.peek() {
                    if next.is_ascii_digit() {
                        is_float = true;
                        num_str.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else if ch == 'e' || ch == 'E' {
                is_float = true;
                num_str.push(ch);
                self.advance();
                if self.current_char == Some('+') || self.current_char == Some('-') {
                    num_str.push(self.current_char.unwrap());
                    self.advance();
                }
            } else {
                break;
            }
        }

        if is_float {
            // Skip float suffix: f, F, l, L
            if let Some(ch) = self.current_char {
                if ch == 'f' || ch == 'F' || ch == 'l' || ch == 'L' {
                    self.advance();
                }
            }
            let val: f64 = num_str.parse().unwrap_or(0.0);
            CToken::FloatLiteral(val)
        } else {
            self.skip_int_suffix();
            let val: i64 = num_str.parse().unwrap_or(0);
            CToken::IntLiteral(val)
        }
    }

    fn skip_int_suffix(&mut self) {
        // Skip U, L, UL, LL, ULL suffixes
        while let Some(ch) = self.current_char {
            if ch == 'u' || ch == 'U' || ch == 'l' || ch == 'L' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_escape(&mut self) -> char {
        self.advance(); // skip backslash
        match self.current_char {
            Some('n') => {
                self.advance();
                '\n'
            }
            Some('t') => {
                self.advance();
                '\t'
            }
            Some('r') => {
                self.advance();
                '\r'
            }
            Some('0') => {
                self.advance();
                '\0'
            }
            Some('\\') => {
                self.advance();
                '\\'
            }
            Some('\'') => {
                self.advance();
                '\''
            }
            Some('"') => {
                self.advance();
                '"'
            }
            Some('a') => {
                self.advance();
                '\x07'
            }
            Some('b') => {
                self.advance();
                '\x08'
            }
            Some('f') => {
                self.advance();
                '\x0C'
            }
            Some('x') => {
                // Hex escape: \xFF
                self.advance();
                let mut hex = String::new();
                for _ in 0..2 {
                    if let Some(ch) = self.current_char {
                        if ch.is_ascii_hexdigit() {
                            hex.push(ch);
                            self.advance();
                        }
                    }
                }
                let val = u8::from_str_radix(&hex, 16).unwrap_or(0);
                val as char
            }
            Some(ch) => {
                self.advance();
                ch
            }
            None => '\0',
        }
    }

    fn read_string(&mut self) -> String {
        self.advance(); // skip opening "
        let mut s = String::new();
        loop {
            match self.current_char {
                None | Some('"') => {
                    self.advance(); // skip closing "
                    break;
                }
                Some('\\') => {
                    s.push(self.read_escape());
                }
                Some(ch) => {
                    s.push(ch);
                    self.advance();
                }
            }
        }
        s
    }

    fn read_char_literal(&mut self) -> char {
        self.advance(); // skip opening '
        let ch = if self.current_char == Some('\\') {
            self.read_escape()
        } else {
            let c = self.current_char.unwrap_or('\0');
            self.advance();
            c
        };
        if self.current_char == Some('\'') {
            self.advance(); // skip closing '
        }
        ch
    }

    pub fn next_token(&mut self) -> CToken {
        self.skip_whitespace();
        self.token_start_line = self.line;

        match self.current_char {
            None => CToken::Eof,

            // Preprocessor directives — skip line
            Some('#') => {
                self.skip_preprocessor_line();
                self.next_token()
            }

            // String literal
            Some('"') => {
                let s = self.read_string();
                CToken::StringLiteral(s)
            }

            // Char literal
            Some('\'') => {
                let ch = self.read_char_literal();
                CToken::CharLiteral(ch)
            }

            // Numbers
            Some(ch) if ch.is_ascii_digit() => self.read_number(),

            // Identifiers and keywords
            Some(ch) if ch.is_alphabetic() || ch == '_' => {
                let ident = self.read_identifier();
                // Wide/Unicode string literal prefixes: L"", u"", U"", u8""
                if (ident == "L" || ident == "u" || ident == "U" || ident == "u8")
                    && self.current_char == Some('"')
                {
                    let s = self.read_string();
                    return CToken::StringLiteral(s);
                }
                // Wide/Unicode char literal prefixes: L'x', u'x', U'x'
                if (ident == "L" || ident == "u" || ident == "U")
                    && self.current_char == Some('\'')
                {
                    let ch = self.read_char_literal();
                    return CToken::CharLiteral(ch);
                }
                match ident.as_str() {
                    "auto" => CToken::Auto,
                    "break" => CToken::Break,
                    "case" => CToken::Case,
                    "char" => CToken::Char,
                    "const" => CToken::Const,
                    "continue" => CToken::Continue,
                    "default" => CToken::Default,
                    "do" => CToken::Do,
                    "double" => CToken::Double,
                    "else" => CToken::Else,
                    "enum" => CToken::Enum,
                    "extern" => CToken::Extern,
                    "float" => CToken::Float,
                    "for" => CToken::For,
                    "goto" => CToken::Goto,
                    "if" => CToken::If,
                    "inline" => CToken::Inline,
                    "int" => CToken::Int,
                    "long" => CToken::Long,
                    "register" => CToken::Register,
                    "restrict" => CToken::Restrict,
                    "return" => CToken::Return,
                    "short" => CToken::Short,
                    "signed" => CToken::Signed,
                    "sizeof" => CToken::Sizeof,
                    "static" => CToken::Static,
                    "struct" => CToken::Struct,
                    "switch" => CToken::Switch,
                    "typedef" => CToken::Typedef,
                    "union" => CToken::Union,
                    "unsigned" => CToken::Unsigned,
                    "void" => CToken::Void,
                    "volatile" => CToken::Volatile,
                    "while" => CToken::While,
                    "_Bool" | "bool" => CToken::Bool,
                    "_Complex" => CToken::Complex,
                    "NULL" | "nullptr" => CToken::Identifier("NULL".to_string()),
                    _ => CToken::Identifier(ident),
                }
            }

            // Operators and punctuation
            Some('+') => {
                self.advance();
                match self.current_char {
                    Some('+') => {
                        self.advance();
                        CToken::PlusPlus
                    }
                    Some('=') => {
                        self.advance();
                        CToken::PlusAssign
                    }
                    _ => CToken::Plus,
                }
            }
            Some('-') => {
                self.advance();
                match self.current_char {
                    Some('-') => {
                        self.advance();
                        CToken::MinusMinus
                    }
                    Some('=') => {
                        self.advance();
                        CToken::MinusAssign
                    }
                    Some('>') => {
                        self.advance();
                        CToken::Arrow
                    }
                    _ => CToken::Minus,
                }
            }
            Some('*') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    CToken::StarAssign
                } else {
                    CToken::Star
                }
            }
            Some('/') => {
                self.advance();
                match self.current_char {
                    Some('/') => {
                        self.advance();
                        self.skip_line_comment();
                        self.next_token()
                    }
                    Some('*') => {
                        self.skip_block_comment();
                        self.next_token()
                    }
                    Some('=') => {
                        self.advance();
                        CToken::SlashAssign
                    }
                    _ => CToken::Slash,
                }
            }
            Some('%') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    CToken::PercentAssign
                } else {
                    CToken::Percent
                }
            }
            Some('=') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    CToken::EqEq
                } else {
                    CToken::Assign
                }
            }
            Some('!') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    CToken::NotEq
                } else {
                    CToken::Bang
                }
            }
            Some('<') => {
                self.advance();
                match self.current_char {
                    Some('<') => {
                        self.advance();
                        if self.current_char == Some('=') {
                            self.advance();
                            CToken::LShiftAssign
                        } else {
                            CToken::LShift
                        }
                    }
                    Some('=') => {
                        self.advance();
                        CToken::LessEq
                    }
                    _ => CToken::Less,
                }
            }
            Some('>') => {
                self.advance();
                match self.current_char {
                    Some('>') => {
                        self.advance();
                        if self.current_char == Some('=') {
                            self.advance();
                            CToken::RShiftAssign
                        } else {
                            CToken::RShift
                        }
                    }
                    Some('=') => {
                        self.advance();
                        CToken::GreaterEq
                    }
                    _ => CToken::Greater,
                }
            }
            Some('&') => {
                self.advance();
                match self.current_char {
                    Some('&') => {
                        self.advance();
                        CToken::AndAnd
                    }
                    Some('=') => {
                        self.advance();
                        CToken::AmpAssign
                    }
                    _ => CToken::Ampersand,
                }
            }
            Some('|') => {
                self.advance();
                match self.current_char {
                    Some('|') => {
                        self.advance();
                        CToken::OrOr
                    }
                    Some('=') => {
                        self.advance();
                        CToken::PipeAssign
                    }
                    _ => CToken::Pipe,
                }
            }
            Some('^') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    CToken::CaretAssign
                } else {
                    CToken::Caret
                }
            }
            Some('~') => {
                self.advance();
                CToken::Tilde
            }
            Some('?') => {
                self.advance();
                CToken::Question
            }
            Some('.') => {
                self.advance();
                if self.current_char == Some('.') {
                    self.advance();
                    if self.current_char == Some('.') {
                        self.advance();
                        CToken::Ellipsis
                    } else {
                        CToken::Dot // fallback
                    }
                } else {
                    CToken::Dot
                }
            }

            // Punctuation
            Some('(') => {
                self.advance();
                CToken::LParen
            }
            Some(')') => {
                self.advance();
                CToken::RParen
            }
            Some('{') => {
                self.advance();
                CToken::LBrace
            }
            Some('}') => {
                self.advance();
                CToken::RBrace
            }
            Some('[') => {
                self.advance();
                CToken::LBracket
            }
            Some(']') => {
                self.advance();
                CToken::RBracket
            }
            Some(';') => {
                self.advance();
                CToken::Semicolon
            }
            Some(',') => {
                self.advance();
                CToken::Comma
            }
            Some(':') => {
                self.advance();
                CToken::Colon
            }

            Some(ch) => {
                eprintln!(
                    "C Lexer: unexpected character '{}' at line {}:{}",
                    ch, self.line, self.column
                );
                self.advance();
                self.next_token()
            }
        }
    }

    pub fn tokenize(&mut self) -> (Vec<CToken>, Vec<usize>) {
        let mut tokens = Vec::new();
        let mut lines = Vec::new();
        loop {
            let tok = self.next_token();
            let line = self.token_start_line;
            let is_eof = tok == CToken::Eof;
            tokens.push(tok);
            lines.push(line);
            if is_eof {
                break;
            }
        }
        (tokens, lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_c() {
        let mut lexer = CLexer::new("int main() { return 0; }");
        assert_eq!(lexer.next_token(), CToken::Int);
        assert_eq!(lexer.next_token(), CToken::Identifier("main".to_string()));
        assert_eq!(lexer.next_token(), CToken::LParen);
        assert_eq!(lexer.next_token(), CToken::RParen);
        assert_eq!(lexer.next_token(), CToken::LBrace);
        assert_eq!(lexer.next_token(), CToken::Return);
        assert_eq!(lexer.next_token(), CToken::IntLiteral(0));
        assert_eq!(lexer.next_token(), CToken::Semicolon);
        assert_eq!(lexer.next_token(), CToken::RBrace);
        assert_eq!(lexer.next_token(), CToken::Eof);
    }

    #[test]
    fn test_hex_literal() {
        let mut lexer = CLexer::new("0xFF");
        assert_eq!(lexer.next_token(), CToken::IntLiteral(255));
    }

    #[test]
    fn test_string_escape() {
        let mut lexer = CLexer::new(r#""hello\nworld""#);
        assert_eq!(
            lexer.next_token(),
            CToken::StringLiteral("hello\nworld".to_string())
        );
    }

    #[test]
    fn test_operators() {
        let mut lexer = CLexer::new("++ -- -> << >> && ||");
        assert_eq!(lexer.next_token(), CToken::PlusPlus);
        assert_eq!(lexer.next_token(), CToken::MinusMinus);
        assert_eq!(lexer.next_token(), CToken::Arrow);
        assert_eq!(lexer.next_token(), CToken::LShift);
        assert_eq!(lexer.next_token(), CToken::RShift);
        assert_eq!(lexer.next_token(), CToken::AndAnd);
        assert_eq!(lexer.next_token(), CToken::OrOr);
    }

    #[test]
    fn test_preprocessor_skip() {
        let mut lexer = CLexer::new("#include <stdio.h>\nint x;");
        assert_eq!(lexer.next_token(), CToken::Int);
        assert_eq!(lexer.next_token(), CToken::Identifier("x".to_string()));
        assert_eq!(lexer.next_token(), CToken::Semicolon);
    }
}
