// ============================================================
// ADead-BIB C++ Frontend — Lexer / Tokenizer
// ============================================================
// Tokenizes C++11/14/17/20 source code into CppToken stream
// Handles: keywords, identifiers, literals, operators, comments
//
// Sin GCC. Sin LLVM. Sin Clang. Solo ADead-BIB. 💀🦈
// ============================================================

#[derive(Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum CppToken {
    // Literals
    IntLiteral(i64),
    UIntLiteral(u64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),

    // Identifier
    Identifier(String),

    // C keywords (shared with C)
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
    Int,
    Long,
    Register,
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
    Bool,
    Inline,

    // C++ keywords
    Class,
    Namespace,
    Using,
    New,
    Delete,
    This,
    Virtual,
    Override,
    Final,
    Public,
    Private,
    Protected,
    Friend,
    Operator,
    Template,
    Typename,
    Try,
    Catch,
    Throw,
    Noexcept,
    Nullptr,
    Constexpr,
    Static_assert,
    Explicit,
    Mutable,
    Thread_local,
    Alignas,
    Alignof,
    Decltype,
    Typeid,
    Concept,
    Requires,
    Co_await,
    Co_yield,
    Co_return,
    Consteval,
    Constinit,
    Char8_t,
    Char16_t,
    Char32_t,
    Wchar_t,

    // C++ cast keywords
    StaticCast,
    DynamicCast,
    ConstCast,
    ReinterpretCast,

    // C++ special
    True,
    False,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    AmpAssign,
    PipeAssign,
    CaretAssign,
    ShlAssign,
    ShrAssign,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Spaceship, // <=> C++20
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    And,
    Or,
    Not, // &&, ||, !
    Increment,
    Decrement, // ++, --
    Arrow,
    Dot,
    DotStar,
    ArrowStar, // ->, ., .*, ->*
    Scope,     // ::
    Ellipsis,  // ...
    Question,
    Colon, // ? :

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LAngle,
    RAngle, // < > (also used for templates)
    Semicolon,
    Comma,
    Hash, // # (preprocessor)

    // Special
    Eof,
}

pub struct CppLexer {
    chars: Vec<char>,
    pos: usize,
    pub line: usize,
    pub column: usize,
}

impl CppLexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> (Vec<CppToken>, Vec<usize>) {
        let mut tokens = Vec::new();
        let mut lines = Vec::new();
        loop {
            let line = self.line;
            let tok = self.next_token();
            if tok == CppToken::Eof {
                tokens.push(tok);
                lines.push(line);
                break;
            }
            tokens.push(tok);
            lines.push(line);
        }
        (tokens, lines)
    }

    fn peek(&self) -> char {
        self.chars.get(self.pos).copied().unwrap_or('\0')
    }

    fn peek_at(&self, offset: usize) -> char {
        self.chars.get(self.pos + offset).copied().unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let ch = self.peek();
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() && self.peek().is_ascii_whitespace() {
            self.advance();
        }
    }

    fn skip_line_comment(&mut self) {
        while self.pos < self.chars.len() && self.peek() != '\n' {
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) {
        self.advance(); // skip /
        self.advance(); // skip *
        while self.pos + 1 < self.chars.len() {
            if self.peek() == '*' && self.peek_at(1) == '/' {
                self.advance();
                self.advance();
                return;
            }
            self.advance();
        }
    }

    fn skip_preprocessor_line(&mut self) {
        let mut line_str = String::new();
        while self.pos < self.chars.len() {
            let ch = self.peek();
            if ch == '\n' {
                // Check if this was a line marker from gcc (e.g. `# 12 "file.c"`)
                let parts: Vec<&str> = line_str.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(num) = parts[1].parse::<usize>() {
                        // The advance() below will consume \n and add 1
                        self.line = num.saturating_sub(1);
                    }
                }
                self.advance();
                break;
            }
            if ch == '\\' && self.peek_at(1) == '\n' {
                self.advance(); // consume \
                self.advance(); // consume \n
                continue;
            }
            line_str.push(ch);
            self.advance();
        }
    }

    fn read_identifier(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.chars.len()
            && (self.peek().is_ascii_alphanumeric() || self.peek() == '_')
        {
            self.advance();
        }
        self.chars[start..self.pos].iter().collect()
    }

    fn read_number(&mut self) -> CppToken {
        let start = self.pos;
        let mut is_float = false;
        let mut is_hex = false;

        // Hex
        if self.peek() == '0' && (self.peek_at(1) == 'x' || self.peek_at(1) == 'X') {
            self.advance(); // 0
            self.advance(); // x
            is_hex = true;
            while self.pos < self.chars.len() && self.peek().is_ascii_hexdigit() {
                self.advance();
            }
        } else if self.peek() == '0' && (self.peek_at(1) == 'b' || self.peek_at(1) == 'B') {
            // C++14 binary literals: 0b10101010
            self.advance(); // 0
            self.advance(); // b
            let bin_start = self.pos;
            while self.pos < self.chars.len() && matches!(self.peek(), '0' | '1' | '\'') {
                self.advance();
            }
            let bin_str: String = self.chars[bin_start..self.pos].iter()
                .filter(|c| **c != '\'')
                .collect();
            // Skip suffixes
            while self.pos < self.chars.len() && matches!(self.peek(), 'u' | 'U' | 'l' | 'L') {
                self.advance();
            }
            let val = i64::from_str_radix(&bin_str, 2).unwrap_or(0);
            return CppToken::IntLiteral(val);
        } else {
            // Decimal / float
            while self.pos < self.chars.len() && self.peek().is_ascii_digit() {
                self.advance();
            }
            if self.peek() == '.' && self.peek_at(1).is_ascii_digit() {
                is_float = true;
                self.advance(); // .
                while self.pos < self.chars.len() && self.peek().is_ascii_digit() {
                    self.advance();
                }
            }
            // Scientific notation
            if self.peek() == 'e' || self.peek() == 'E' {
                is_float = true;
                self.advance();
                if self.peek() == '+' || self.peek() == '-' {
                    self.advance();
                }
                while self.pos < self.chars.len() && self.peek().is_ascii_digit() {
                    self.advance();
                }
            }
        }

        // Skip suffixes: u, U, l, L, ll, LL, f, F
        let mut is_unsigned = false;
        loop {
            match self.peek() {
                'u' | 'U' => {
                    is_unsigned = true;
                    self.advance();
                }
                'l' | 'L' => {
                    self.advance();
                }
                'f' | 'F' => {
                    is_float = true;
                    self.advance();
                }
                _ => break,
            }
        }

        let text: String = self.chars[start..self.pos].iter().collect();
        let clean: String = text
            .chars()
            .filter(|c| !matches!(c, 'u' | 'U' | 'l' | 'L' | 'f' | 'F'))
            .collect();

        if is_float {
            CppToken::FloatLiteral(clean.parse().unwrap_or(0.0))
        } else if is_hex {
            let hex_str = clean.trim_start_matches("0x").trim_start_matches("0X");
            if is_unsigned {
                CppToken::UIntLiteral(u64::from_str_radix(hex_str, 16).unwrap_or(0))
            } else {
                CppToken::IntLiteral(i64::from_str_radix(hex_str, 16).unwrap_or(0))
            }
        } else if is_unsigned {
            CppToken::UIntLiteral(clean.parse().unwrap_or(0))
        } else {
            CppToken::IntLiteral(clean.parse().unwrap_or(0))
        }
    }

    fn read_string(&mut self) -> String {
        self.advance(); // skip opening "
        let mut s = String::new();
        while self.pos < self.chars.len() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance();
                match self.peek() {
                    'n' => {
                        s.push('\n');
                        self.advance();
                    }
                    't' => {
                        s.push('\t');
                        self.advance();
                    }
                    'r' => {
                        s.push('\r');
                        self.advance();
                    }
                    '\\' => {
                        s.push('\\');
                        self.advance();
                    }
                    '"' => {
                        s.push('"');
                        self.advance();
                    }
                    '\'' => {
                        s.push('\'');
                        self.advance();
                    }
                    '0' => {
                        s.push('\0');
                        self.advance();
                    }
                    'x' => {
                        self.advance();
                        let mut hex = String::new();
                        for _ in 0..2 {
                            if self.peek().is_ascii_hexdigit() {
                                hex.push(self.advance());
                            }
                        }
                        if let Ok(n) = u8::from_str_radix(&hex, 16) {
                            s.push(n as char);
                        }
                    }
                    other => {
                        s.push(other);
                        self.advance();
                    }
                }
            } else {
                s.push(self.advance());
            }
        }
        if self.peek() == '"' {
            self.advance();
        }
        s
    }

    fn read_char_literal(&mut self) -> char {
        self.advance(); // skip '
        let ch = if self.peek() == '\\' {
            self.advance();
            match self.advance() {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '0' => '\0',
                other => other,
            }
        } else {
            self.advance()
        };
        if self.peek() == '\'' {
            self.advance();
        }
        ch
    }

    fn keyword_or_ident(&mut self, word: &str) -> CppToken {
        match word {
            // C keywords
            "auto" => CppToken::Auto,
            "break" => CppToken::Break,
            "case" => CppToken::Case,
            "char" => CppToken::Char,
            "const" => CppToken::Const,
            "continue" => CppToken::Continue,
            "default" => CppToken::Default,
            "do" => CppToken::Do,
            "double" => CppToken::Double,
            "else" => CppToken::Else,
            "enum" => CppToken::Enum,
            "extern" => CppToken::Extern,
            "float" => CppToken::Float,
            "for" => CppToken::For,
            "goto" => CppToken::Goto,
            "if" => CppToken::If,
            "int" => CppToken::Int,
            "long" => CppToken::Long,
            "register" => CppToken::Register,
            "return" => CppToken::Return,
            "short" => CppToken::Short,
            "signed" => CppToken::Signed,
            "sizeof" => CppToken::Sizeof,
            "static" => CppToken::Static,
            "struct" => CppToken::Struct,
            "switch" => CppToken::Switch,
            "typedef" => CppToken::Typedef,
            "union" => CppToken::Union,
            "unsigned" => CppToken::Unsigned,
            "void" => CppToken::Void,
            "volatile" => CppToken::Volatile,
            "while" => CppToken::While,
            "inline" => CppToken::Inline,
            "bool" | "_Bool" => CppToken::Bool,

            // C++ keywords
            "class" => CppToken::Class,
            "namespace" => CppToken::Namespace,
            "using" => CppToken::Using,
            "new" => CppToken::New,
            "delete" => CppToken::Delete,
            "this" => CppToken::This,
            "virtual" => CppToken::Virtual,
            "override" => CppToken::Override,
            "final" => CppToken::Final,
            "public" => CppToken::Public,
            "private" => CppToken::Private,
            "protected" => CppToken::Protected,
            "friend" => CppToken::Friend,
            "operator" => CppToken::Operator,
            "template" => CppToken::Template,
            "typename" => CppToken::Typename,
            "try" => CppToken::Try,
            "catch" => CppToken::Catch,
            "throw" => CppToken::Throw,
            "noexcept" => CppToken::Noexcept,
            "nullptr" => CppToken::Nullptr,
            "constexpr" => CppToken::Constexpr,
            "static_assert" => CppToken::Static_assert,
            "explicit" => CppToken::Explicit,
            "mutable" => CppToken::Mutable,
            "thread_local" => CppToken::Thread_local,
            "alignas" => CppToken::Alignas,
            "alignof" => CppToken::Alignof,
            "decltype" => CppToken::Decltype,
            "typeid" => CppToken::Typeid,
            "concept" => CppToken::Concept,
            "requires" => CppToken::Requires,
            "co_await" => CppToken::Co_await,
            "co_yield" => CppToken::Co_yield,
            "co_return" => CppToken::Co_return,
            "consteval" => CppToken::Consteval,
            "constinit" => CppToken::Constinit,
            "char8_t" => CppToken::Char8_t,
            "char16_t" => CppToken::Char16_t,
            "char32_t" => CppToken::Char32_t,
            "wchar_t" => CppToken::Wchar_t,

            // C++ casts
            "static_cast" => CppToken::StaticCast,
            "dynamic_cast" => CppToken::DynamicCast,
            "const_cast" => CppToken::ConstCast,
            "reinterpret_cast" => CppToken::ReinterpretCast,

            // Boolean literals
            "true" => CppToken::True,
            "false" => CppToken::False,

            // __declspec(...) — skip entirely (MSVC extension)
            "__declspec" => {
                if self.pos < self.chars.len() && self.chars[self.pos] == '(' {
                    self.advance(); // (
                    let mut depth = 1i32;
                    while self.pos < self.chars.len() && depth > 0 {
                        match self.chars[self.pos] {
                            '(' => { depth += 1; self.pos += 1; }
                            ')' => { depth -= 1; self.pos += 1; }
                            _ => { self.pos += 1; }
                        }
                    }
                }
                return self.next_token();
            }

            // __attribute__((...)) — skip entirely (GCC extension)
            "__attribute__" => {
                if self.pos < self.chars.len() && self.chars[self.pos] == '(' {
                    self.advance(); // (
                    let mut depth = 1i32;
                    while self.pos < self.chars.len() && depth > 0 {
                        match self.chars[self.pos] {
                            '(' => { depth += 1; self.pos += 1; }
                            ')' => { depth -= 1; self.pos += 1; }
                            _ => { self.pos += 1; }
                        }
                    }
                }
                return self.next_token();
            }

            // SAL annotations — skip entirely (MSVC source annotations)
            "_Use_decl_annotations_" | "_In_" | "_Out_" | "_Inout_"
            | "_In_reads_" | "_In_reads_opt_" | "_In_reads_bytes_"
            | "_Out_writes_" | "_Out_writes_opt_" | "_Out_writes_bytes_"
            | "_Outptr_" | "_Outptr_result_maybenull_" | "_Outptr_result_nullonfailure_"
            | "_Ret_maybenull_" | "_Check_return_"
            | "_In_opt_" | "_Out_opt_" | "_Inout_opt_"
            | "_Pre_" | "_Post_" | "_Deref_" | "_Null_terminated_"
            | "_COM_Outptr_" | "_Field_size_" | "_Field_size_bytes_"
            | "__in" | "__out" | "__inout" | "__in_opt" | "__out_opt" => {
                // If followed by ( ), skip the parenthesized args too
                if self.pos < self.chars.len() && self.chars[self.pos] == '(' {
                    self.advance();
                    let mut depth = 1i32;
                    while self.pos < self.chars.len() && depth > 0 {
                        match self.chars[self.pos] {
                            '(' => { depth += 1; self.pos += 1; }
                            ')' => { depth -= 1; self.pos += 1; }
                            _ => { self.pos += 1; }
                        }
                    }
                }
                return self.next_token();
            }

            // _countof — treat as sizeof equivalent
            "_countof" => CppToken::Sizeof,

            // WINAPI, CALLBACK, APIENTRY — calling conventions, ignore
            "WINAPI" | "CALLBACK" | "APIENTRY" | "STDCALL" | "__stdcall"
            | "__cdecl" | "__fastcall" | "__thiscall" | "__vectorcall" => {
                return self.next_token();
            }

            // Identifier
            _ => CppToken::Identifier(word.to_string()),
        }
    }

    pub fn next_token(&mut self) -> CppToken {
        loop {
            self.skip_whitespace();
            if self.pos >= self.chars.len() {
                return CppToken::Eof;
            }

            // Comments
            if self.peek() == '/' && self.peek_at(1) == '/' {
                self.skip_line_comment();
                continue;
            }
            if self.peek() == '/' && self.peek_at(1) == '*' {
                self.skip_block_comment();
                continue;
            }

            // Preprocessor — skip entire line
            if self.peek() == '#' {
                self.skip_preprocessor_line();
                continue;
            }

            break;
        }

        if self.pos >= self.chars.len() {
            return CppToken::Eof;
        }

        let ch = self.peek();

        // Wide/Unicode string literal prefixes: L"", u"", U"", u8""
        if (ch == 'L' || ch == 'U') && self.peek_at(1) == '"' {
            self.advance(); // skip prefix
            return CppToken::StringLiteral(self.read_string());
        }
        if ch == 'u' && self.peek_at(1) == '"' {
            self.advance(); // skip u
            return CppToken::StringLiteral(self.read_string());
        }
        if ch == 'u' && self.peek_at(1) == '8' && self.peek_at(2) == '"' {
            self.advance(); // skip u
            self.advance(); // skip 8
            return CppToken::StringLiteral(self.read_string());
        }
        // Wide/Unicode char literal prefixes: L'x', u'x', U'x'
        if (ch == 'L' || ch == 'U' || ch == 'u') && self.peek_at(1) == '\'' {
            self.advance(); // skip prefix
            return CppToken::CharLiteral(self.read_char_literal());
        }

        // Identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == '_' {
            let word = self.read_identifier();
            return self.keyword_or_ident(&word);
        }

        // Numbers
        if ch.is_ascii_digit() {
            return self.read_number();
        }
        // Float starting with dot: .5
        if ch == '.' && self.peek_at(1).is_ascii_digit() {
            return self.read_number();
        }

        // String literals (including raw string R"(...)")
        if ch == '"' {
            return CppToken::StringLiteral(self.read_string());
        }
        // Raw string prefix
        if ch == 'R' && self.peek_at(1) == '"' {
            self.advance(); // skip R
            return CppToken::StringLiteral(self.read_string());
        }

        // Character literal
        if ch == '\'' {
            return CppToken::CharLiteral(self.read_char_literal());
        }

        // Multi-character operators
        self.advance();
        match ch {
            '+' => {
                if self.peek() == '+' {
                    self.advance();
                    return CppToken::Increment;
                }
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::PlusAssign;
                }
                CppToken::Plus
            }
            '-' => {
                if self.peek() == '-' {
                    self.advance();
                    return CppToken::Decrement;
                }
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::MinusAssign;
                }
                if self.peek() == '>' {
                    self.advance();
                    if self.peek() == '*' {
                        self.advance();
                        return CppToken::ArrowStar;
                    }
                    return CppToken::Arrow;
                }
                CppToken::Minus
            }
            '*' => {
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::StarAssign;
                }
                CppToken::Star
            }
            '/' => {
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::SlashAssign;
                }
                CppToken::Slash
            }
            '%' => {
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::PercentAssign;
                }
                CppToken::Percent
            }
            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::Eq;
                }
                CppToken::Assign
            }
            '!' => {
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::Ne;
                }
                CppToken::Not
            }
            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    if self.peek() == '>' {
                        self.advance();
                        return CppToken::Spaceship;
                    }
                    return CppToken::Le;
                }
                if self.peek() == '<' {
                    self.advance();
                    if self.peek() == '=' {
                        self.advance();
                        return CppToken::ShlAssign;
                    }
                    return CppToken::Shl;
                }
                CppToken::Lt
            }
            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::Ge;
                }
                if self.peek() == '>' {
                    self.advance();
                    if self.peek() == '=' {
                        self.advance();
                        return CppToken::ShrAssign;
                    }
                    return CppToken::Shr;
                }
                CppToken::Gt
            }
            '&' => {
                if self.peek() == '&' {
                    self.advance();
                    return CppToken::And;
                }
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::AmpAssign;
                }
                CppToken::Amp
            }
            '|' => {
                if self.peek() == '|' {
                    self.advance();
                    return CppToken::Or;
                }
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::PipeAssign;
                }
                CppToken::Pipe
            }
            '^' => {
                if self.peek() == '=' {
                    self.advance();
                    return CppToken::CaretAssign;
                }
                CppToken::Caret
            }
            '~' => CppToken::Tilde,
            '?' => CppToken::Question,
            ':' => {
                if self.peek() == ':' {
                    self.advance();
                    return CppToken::Scope;
                }
                CppToken::Colon
            }
            '.' => {
                if self.peek() == '.' && self.peek_at(1) == '.' {
                    self.advance();
                    self.advance();
                    return CppToken::Ellipsis;
                }
                if self.peek() == '*' {
                    self.advance();
                    return CppToken::DotStar;
                }
                CppToken::Dot
            }
            '(' => CppToken::LParen,
            ')' => CppToken::RParen,
            '[' => CppToken::LBracket,
            ']' => CppToken::RBracket,
            '{' => CppToken::LBrace,
            '}' => CppToken::RBrace,
            ';' => CppToken::Semicolon,
            ',' => CppToken::Comma,
            '#' => CppToken::Hash,
            _ => {
                // Skip unknown character
                CppToken::Identifier(ch.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_cpp() {
        let tokens = CppLexer::new("int main() { return 0; }").tokenize().0;
        assert_eq!(tokens[0], CppToken::Int);
        assert_eq!(tokens[1], CppToken::Identifier("main".to_string()));
        assert_eq!(tokens[2], CppToken::LParen);
        assert_eq!(tokens[3], CppToken::RParen);
        assert_eq!(tokens[4], CppToken::LBrace);
        assert_eq!(tokens[5], CppToken::Return);
        assert_eq!(tokens[6], CppToken::IntLiteral(0));
        assert_eq!(tokens[7], CppToken::Semicolon);
        assert_eq!(tokens[8], CppToken::RBrace);
    }

    #[test]
    fn test_cpp_keywords() {
        let tokens = CppLexer::new("class Foo : public Base { virtual void f(); };")
            .tokenize()
            .0;
        assert_eq!(tokens[0], CppToken::Class);
        assert_eq!(tokens[1], CppToken::Identifier("Foo".to_string()));
        assert_eq!(tokens[2], CppToken::Colon);
        assert_eq!(tokens[3], CppToken::Public);
        assert_eq!(tokens[4], CppToken::Identifier("Base".to_string()));
        assert_eq!(tokens[5], CppToken::LBrace);
        assert_eq!(tokens[6], CppToken::Virtual);
    }

    #[test]
    fn test_scope_operator() {
        let tokens = CppLexer::new("std::cout << x;").tokenize().0;
        assert_eq!(tokens[0], CppToken::Identifier("std".to_string()));
        assert_eq!(tokens[1], CppToken::Scope);
        assert_eq!(tokens[2], CppToken::Identifier("cout".to_string()));
        assert_eq!(tokens[3], CppToken::Shl);
    }

    #[test]
    fn test_lambda() {
        let tokens = CppLexer::new("[&](int x) { return x * 2; }").tokenize().0;
        assert_eq!(tokens[0], CppToken::LBracket);
        assert_eq!(tokens[1], CppToken::Amp);
        assert_eq!(tokens[2], CppToken::RBracket);
        assert_eq!(tokens[3], CppToken::LParen);
        assert_eq!(tokens[4], CppToken::Int);
    }

    #[test]
    fn test_template() {
        let tokens = CppLexer::new("template<typename T>").tokenize().0;
        assert_eq!(tokens[0], CppToken::Template);
        assert_eq!(tokens[1], CppToken::Lt);
        assert_eq!(tokens[2], CppToken::Typename);
        assert_eq!(tokens[3], CppToken::Identifier("T".to_string()));
        assert_eq!(tokens[4], CppToken::Gt);
    }

    #[test]
    fn test_nullptr_and_auto() {
        let tokens = CppLexer::new("auto x = nullptr;").tokenize().0;
        assert_eq!(tokens[0], CppToken::Auto);
        assert_eq!(tokens[1], CppToken::Identifier("x".to_string()));
        assert_eq!(tokens[2], CppToken::Assign);
        assert_eq!(tokens[3], CppToken::Nullptr);
    }

    #[test]
    fn test_arrow_and_scope() {
        let tokens = CppLexer::new("ptr->member; Foo::bar").tokenize().0;
        assert_eq!(tokens[0], CppToken::Identifier("ptr".to_string()));
        assert_eq!(tokens[1], CppToken::Arrow);
        assert_eq!(tokens[2], CppToken::Identifier("member".to_string()));
        assert_eq!(tokens[4], CppToken::Identifier("Foo".to_string()));
        assert_eq!(tokens[5], CppToken::Scope);
        assert_eq!(tokens[6], CppToken::Identifier("bar".to_string()));
    }

    #[test]
    fn test_preprocessor_skip() {
        let tokens = CppLexer::new("#include <iostream>\nint main() {}")
            .tokenize()
            .0;
        assert_eq!(tokens[0], CppToken::Int);
    }
}
