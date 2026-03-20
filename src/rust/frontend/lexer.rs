// Lexer para ADead-BIB
// Tokeniza el código fuente

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords - Python style (legacy)
    Def,
    Print,
    Println, // println con \n automático
    Input,   // input() para leer del teclado
    Return,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    Range,
    Break,
    Continue,
    And,
    Or,
    Not,
    True,
    False,

    // Keywords - Rust style (NEW)
    Fn,     // fn (alias de def)
    Let,    // let
    Mut,    // mut
    Const,  // const
    Pub,    // pub
    Mod,    // mod
    Use,    // use
    Struct, // struct
    Enum,   // enum
    Impl,   // impl
    Trait,  // trait
    Match,  // match
    Loop,   // loop
    Ref,    // ref / &
    Move,   // move
    Box,    // Box
    Vec,    // Vec
    Option, // Option
    Result, // Result
    Some,   // Some
    None,   // None
    Ok,     // Ok
    Err,    // Err
    Unsafe, // unsafe
    Where,  // where
    Type,   // type

    // OOP Keywords
    Class,
    New,
    This,
    Super,
    Extends,
    Virtual,
    Override,
    Static,
    Abstract,
    Interface,
    Implements,

    // Advanced
    Init, // __init__
    Del,  // __del__
    Lambda,
    Null,
    Import,
    From,
    As,
    Try,
    Except,
    Finally,
    Raise,
    Assert,
    Async,
    Await,

    // Built-in functions v1.3.0
    Len,       // len() - longitud de arrays/strings
    Push,      // push() - agregar a array
    Pop,       // pop() - remover de array
    Int,       // int() - convertir a entero
    FloatCast, // float() - convertir a flotante
    Str,       // str() - convertir a string
    BoolCast,  // bool() - convertir a booleano

    // C-style types (NEW v3.0)
    IntType,    // int
    CharType,   // char
    VoidType,   // void
    LongType,   // long
    ShortType,  // short
    UnsignedKw, // unsigned
    SignedKw,   // signed
    DoubleType, // double
    FloatType,  // float (type, not cast)
    SizeofKw,   // sizeof
    TypedefKw,  // typedef
    ExternKw,   // extern
    RegisterKw, // register
    VolatileKw, // volatile
    AutoKw,     // auto
    Printf,     // printf (C-style)
    Scanf,      // scanf
    Malloc,     // malloc
    Free,       // free
    NULL,       // NULL

    // C++ style (NEW v3.1)
    Do,        // do (for do-while)
    Namespace, // namespace
    Using,     // using
    Template,  // template
    Typename,  // typename
    Private,   // private
    Protected, // protected
    Public,    // public (C++ style)
    Friend,    // friend
    Inline,    // inline
    Constexpr, // constexpr
    Delete,    // delete
    Nullptr,   // nullptr
    Bool,      // bool type
    Cout,      // cout
    Cin,       // cin
    Endl,      // endl

    // OS-Level / Machine Code keywords (NEW v3.1-OS)
    CliKw,       // cli — disable interrupts
    StiKw,       // sti — enable interrupts
    HltKw,       // hlt — halt CPU
    IretKw,      // iret — return from interrupt
    OrgKw,       // org — set origin address
    PackedKw,    // packed — packed struct attribute
    InterruptKw, // interrupt — interrupt handler attribute
    RawBlockKw,  // raw — inline raw bytes block
    WriteMemKw,  // write_mem — write to memory address
    ReadMemKw,   // read_mem — read from memory address
    PortOutKw,   // port_out — output to I/O port
    PortInKw,    // port_in — input from I/O port
    FarJumpKw,   // far_jump — far jump with segment:offset
    CpuidKw,     // cpuid — CPU identification
    IntCallKw,   // int_call — software interrupt
    AlignKw,     // align — alignment directive
    RegKw,       // reg — register access
    SegmentKw,   // segment — segment register access

    // Labels and Jumps (NEW v3.3-Boot)
    LabelKw,     // label — define a label
    LabelAddrKw, // label_addr — get absolute address of a label
    JmpKw,       // jmp — unconditional jump
    JzKw,        // jz — jump if zero
    JnzKw,       // jnz — jump if not zero
    JcKw,        // jc — jump if carry
    JncKw,       // jnc — jump if not carry
    DbKw,        // db — define bytes
    DwKw,        // dw — define words (16-bit)
    DdKw,        // dd — define dwords (32-bit)
    TimesKw,     // times — repeat directive

    // Operators adicionales
    PlusPlus,     // ++
    MinusMinus,   // --
    AndAnd,       // &&
    OrOr,         // ||
    LeftShift,    // <<
    RightShift,   // >>
    PercentEq,    // %=
    AmpEq,        // &=
    PipeEq,       // |=
    CaretEq,      // ^=
    LeftShiftEq,  // <<=
    RightShiftEq, // >>=
    Scope,        // :: (namespace scope)

    // Identifiers
    Identifier(String),

    // Literals
    Number(i64),
    Float(f64), // NEW: Floating point
    String(String),
    Char(char), // NEW: Character literal

    // Operators
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Equals,      // =
    EqEq,        // ==
    NotEq,       // !=
    Less,        // <
    LessEq,      // <=
    Greater,     // >
    GreaterEq,   // >=
    Dot,         // .
    DoubleDot,   // .. (range)
    Ampersand,   // &
    Pipe,        // |
    Caret,       // ^
    Tilde,       // ~
    Bang,        // !
    Question,    // ?
    DoubleColon, // ::
    FatArrow,    // =>
    PlusEq,      // +=
    MinusEq,     // -=
    StarEq,      // *=
    SlashEq,     // /=

    // Punctuation
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    LBrace,    // {
    RBrace,    // }
    Colon,     // :
    Comma,     // ,
    Semicolon, // ;
    Arrow,     // ->
    Newline,   // \n

    // EOF
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    current_char: Option<char>,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current = if chars.is_empty() {
            None
        } else {
            Some(chars[0])
        };

        Self {
            input: chars,
            position: 0,
            current_char: current,
            line: 1,
            column: 1,
        }
    }

    pub fn get_line(&self) -> usize {
        self.line
    }

    pub fn get_column(&self) -> usize {
        self.column
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
        if self.position >= self.input.len() {
            self.current_char = None;
        } else {
            self.current_char = Some(self.input[self.position]);
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.current_char == Some('#') {
            while let Some(ch) = self.current_char {
                if ch == '\n' {
                    break;
                }
                self.advance();
            }
        }
    }

    fn read_number(&mut self) -> i64 {
        let mut num_str = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        num_str.parse().unwrap_or(0)
    }

    fn read_number_or_float(&mut self) -> Token {
        let mut num_str = String::new();
        let mut is_float = false;

        // Verificar si es literal HEX (0x...) o binario (0b...)
        if self.current_char == Some('0') {
            num_str.push('0');
            self.advance();

            // Literal HEX: 0x...
            if self.current_char == Some('x') || self.current_char == Some('X') {
                self.advance(); // Skip 'x'
                let mut hex_str = String::new();
                while let Some(ch) = self.current_char {
                    if ch.is_ascii_hexdigit() {
                        hex_str.push(ch);
                        self.advance();
                    } else if ch == '_' {
                        // Separador estilo Rust: 0xFF_FF
                        self.advance();
                    } else {
                        break;
                    }
                }
                let value = i64::from_str_radix(&hex_str, 16).unwrap_or(0);
                return Token::Number(value);
            }

            // Literal Binario: 0b...
            if self.current_char == Some('b') || self.current_char == Some('B') {
                self.advance(); // Skip 'b'
                let mut bin_str = String::new();
                while let Some(ch) = self.current_char {
                    if ch == '0' || ch == '1' {
                        bin_str.push(ch);
                        self.advance();
                    } else if ch == '_' {
                        // Separador estilo Rust: 0b1111_0000
                        self.advance();
                    } else {
                        break;
                    }
                }
                let value = i64::from_str_radix(&bin_str, 2).unwrap_or(0);
                return Token::Number(value);
            }

            // Literal Octal: 0o... (opcional, para completitud)
            if self.current_char == Some('o') || self.current_char == Some('O') {
                self.advance(); // Skip 'o'
                let mut oct_str = String::new();
                while let Some(ch) = self.current_char {
                    if ch >= '0' && ch <= '7' {
                        oct_str.push(ch);
                        self.advance();
                    } else if ch == '_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let value = i64::from_str_radix(&oct_str, 8).unwrap_or(0);
                return Token::Number(value);
            }

            // Es solo un 0 o un número que empieza con 0
            // Continuar leyendo como número normal
        }

        // Leer parte entera (número decimal normal)
        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' {
                // Verificar si es un punto decimal o el operador ..
                let next_pos = self.position + 1;
                if next_pos < self.input.len() && self.input[next_pos] == '.' {
                    // Es el operador .., no un decimal
                    break;
                }
                is_float = true;
                num_str.push(ch);
                self.advance();
            } else if ch == '_' {
                // Separador de miles estilo Rust: 1_000_000
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            Token::Float(num_str.parse().unwrap_or(0.0))
        } else {
            Token::Number(num_str.parse().unwrap_or(0))
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }

    fn read_string(&mut self) -> String {
        self.advance(); // Skip opening "
        let mut s = String::new();
        while let Some(ch) = self.current_char {
            if ch == '"' {
                self.advance(); // Skip closing "
                break;
            }
            s.push(ch);
            self.advance();
        }
        s
    }

    pub fn next_token(&mut self) -> Token {
        // Skip whitespace and comments
        loop {
            self.skip_whitespace();
            if self.current_char == Some('#') {
                self.skip_comment();
            } else {
                break;
            }
        }

        match self.current_char {
            None => Token::Eof,

            Some('+') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::PlusEq
                } else if self.current_char == Some('+') {
                    self.advance();
                    Token::PlusPlus
                } else {
                    Token::Plus
                }
            }

            Some('-') => {
                self.advance();
                if self.current_char == Some('>') {
                    self.advance();
                    Token::Arrow
                } else if self.current_char == Some('=') {
                    self.advance();
                    Token::MinusEq
                } else if self.current_char == Some('-') {
                    self.advance();
                    Token::MinusMinus
                } else {
                    Token::Minus
                }
            }

            Some('*') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::StarEq
                } else {
                    Token::Star
                }
            }

            Some('/') => {
                self.advance();
                if self.current_char == Some('/') {
                    // Comentario estilo Rust //
                    while let Some(ch) = self.current_char {
                        if ch == '\n' {
                            break;
                        }
                        self.advance();
                    }
                    self.next_token()
                } else if self.current_char == Some('=') {
                    self.advance();
                    Token::SlashEq
                } else {
                    Token::Slash
                }
            }

            Some('=') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::EqEq
                } else {
                    Token::Equals
                }
            }

            Some('!') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::NotEq
                } else {
                    Token::Not
                }
            }

            Some('<') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::LessEq
                } else if self.current_char == Some('<') {
                    self.advance();
                    if self.current_char == Some('=') {
                        self.advance();
                        Token::LeftShiftEq
                    } else {
                        Token::LeftShift
                    }
                } else {
                    Token::Less
                }
            }

            Some('>') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::GreaterEq
                } else if self.current_char == Some('>') {
                    self.advance();
                    if self.current_char == Some('=') {
                        self.advance();
                        Token::RightShiftEq
                    } else {
                        Token::RightShift
                    }
                } else {
                    Token::Greater
                }
            }

            Some('%') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::PercentEq
                } else {
                    Token::Percent
                }
            }

            Some('&') => {
                self.advance();
                if self.current_char == Some('&') {
                    self.advance();
                    Token::AndAnd
                } else if self.current_char == Some('=') {
                    self.advance();
                    Token::AmpEq
                } else {
                    Token::Ampersand
                }
            }

            Some('|') => {
                self.advance();
                if self.current_char == Some('|') {
                    self.advance();
                    Token::OrOr
                } else if self.current_char == Some('=') {
                    self.advance();
                    Token::PipeEq
                } else {
                    Token::Pipe
                }
            }

            Some('^') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::CaretEq
                } else {
                    Token::Caret
                }
            }

            Some(',') => {
                self.advance();
                Token::Comma
            }

            Some('[') => {
                self.advance();
                Token::LBracket
            }

            Some(']') => {
                self.advance();
                Token::RBracket
            }

            Some('(') => {
                self.advance();
                Token::LParen
            }

            Some(')') => {
                self.advance();
                Token::RParen
            }

            Some(':') => {
                self.advance();
                if self.current_char == Some(':') {
                    self.advance();
                    Token::DoubleColon
                } else {
                    Token::Colon
                }
            }

            Some(';') => {
                self.advance();
                Token::Semicolon
            }

            Some('{') => {
                self.advance();
                Token::LBrace
            }

            Some('}') => {
                self.advance();
                Token::RBrace
            }

            Some('?') => {
                self.advance();
                Token::Question
            }

            Some('\n') => {
                self.advance();
                Token::Newline
            }

            Some('"') => Token::String(self.read_string()),

            Some(ch) if ch.is_ascii_digit() => self.read_number_or_float(),

            Some('\'') => {
                self.advance(); // Skip opening '
                let ch = self.current_char.unwrap_or('\0');
                self.advance();
                if self.current_char == Some('\'') {
                    self.advance(); // Skip closing '
                }
                Token::Char(ch)
            }

            Some('.') => {
                self.advance();
                if self.current_char == Some('.') {
                    self.advance();
                    Token::DoubleDot
                } else {
                    Token::Dot
                }
            }

            Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {
                let ident = self.read_identifier();
                match ident.as_str() {
                    // Python style (legacy)
                    "def" => Token::Def,
                    "print" => Token::Print,
                    "println" => Token::Println,
                    "input" => Token::Input,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "elif" => Token::Elif,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "for" => Token::For,
                    "in" => Token::In,
                    "range" => Token::Range,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "and" | "&&" => Token::And,
                    "or" | "||" => Token::Or,
                    "not" => Token::Not,
                    "true" | "True" => Token::True,
                    "false" | "False" => Token::False,

                    // Rust style (NEW)
                    "fn" => Token::Fn,
                    "let" => Token::Let,
                    "mut" => Token::Mut,
                    "const" => Token::Const,
                    "pub" => Token::Pub,
                    "mod" => Token::Mod,
                    "use" => Token::Use,
                    "struct" => Token::Struct,
                    "enum" => Token::Enum,
                    "impl" => Token::Impl,
                    "trait" => Token::Trait,
                    "match" => Token::Match,
                    "loop" => Token::Loop,
                    "ref" => Token::Ref,
                    "move" => Token::Move,
                    "Box" => Token::Box,
                    "Vec" => Token::Vec,
                    "Option" => Token::Option,
                    "Result" => Token::Result,
                    "Some" => Token::Some,
                    "None" => Token::None,
                    "Ok" => Token::Ok,
                    "Err" => Token::Err,
                    "unsafe" => Token::Unsafe,
                    "where" => Token::Where,
                    "type" => Token::Type,

                    // OOP keywords
                    "class" => Token::Class,
                    "new" => Token::New,
                    "this" | "self" => Token::This,
                    "super" => Token::Super,
                    "extends" => Token::Extends,
                    "virtual" => Token::Virtual,
                    "override" => Token::Override,
                    "static" => Token::Static,
                    "abstract" => Token::Abstract,
                    "interface" => Token::Interface,
                    "implements" => Token::Implements,

                    // Advanced
                    "__init__" => Token::Init,
                    "__del__" => Token::Del,
                    "lambda" => Token::Lambda,
                    "null" => Token::Null,
                    "import" => Token::Import,
                    "from" => Token::From,
                    "as" => Token::As,
                    "try" => Token::Try,
                    "except" => Token::Except,
                    "finally" => Token::Finally,
                    "raise" => Token::Raise,
                    "assert" => Token::Assert,
                    "async" => Token::Async,
                    "await" => Token::Await,

                    // Built-in functions v1.3.0
                    "len" => Token::Len,
                    "push" => Token::Push,
                    "pop" => Token::Pop,

                    // C-style types and keywords (NEW v3.0)
                    "int" => Token::IntType,
                    "char" => Token::CharType,
                    "void" => Token::VoidType,
                    "long" => Token::LongType,
                    "short" => Token::ShortType,
                    "unsigned" => Token::UnsignedKw,
                    "signed" => Token::SignedKw,
                    "double" => Token::DoubleType,
                    "sizeof" => Token::SizeofKw,
                    "typedef" => Token::TypedefKw,
                    "extern" => Token::ExternKw,
                    "register" => Token::RegisterKw,
                    "volatile" => Token::VolatileKw,
                    "auto" => Token::AutoKw,
                    "printf" => Token::Printf,
                    "scanf" => Token::Scanf,
                    "malloc" => Token::Malloc,
                    "free" => Token::Free,
                    "NULL" => Token::NULL,

                    // C++ style keywords (NEW v3.1)
                    "do" => Token::Do,
                    "namespace" => Token::Namespace,
                    "using" => Token::Using,
                    "template" => Token::Template,
                    "typename" => Token::Typename,
                    "private" => Token::Private,
                    "protected" => Token::Protected,
                    "public" => Token::Public,
                    "friend" => Token::Friend,
                    "inline" => Token::Inline,
                    "constexpr" => Token::Constexpr,
                    "delete" => Token::Delete,
                    "nullptr" => Token::Nullptr,
                    "bool" => Token::Bool,
                    "cout" => Token::Cout,
                    "cin" => Token::Cin,
                    "endl" => Token::Endl,

                    // Type casts (keep for compatibility)
                    "float" => Token::FloatType,
                    "str" => Token::Str,

                    // OS-Level keywords (NEW v3.1-OS)
                    "cli" => Token::CliKw,
                    "sti" => Token::StiKw,
                    "hlt" => Token::HltKw,
                    "iret" => Token::IretKw,
                    "org" => Token::OrgKw,
                    "packed" => Token::PackedKw,
                    "interrupt" => Token::InterruptKw,
                    "raw" | "raw_block" => Token::RawBlockKw,
                    "write_mem" => Token::WriteMemKw,
                    "read_mem" => Token::ReadMemKw,
                    "port_out" => Token::PortOutKw,
                    "port_in" => Token::PortInKw,
                    "far_jump" => Token::FarJumpKw,
                    "cpuid" => Token::CpuidKw,
                    "int_call" => Token::IntCallKw,
                    "align" => Token::AlignKw,
                    "reg" => Token::RegKw,
                    "segment" => Token::SegmentKw,

                    // Labels and Jumps (NEW v3.3-Boot)
                    "label" => Token::LabelKw,
                    "label_addr" => Token::LabelAddrKw,
                    "jmp" => Token::JmpKw,
                    "jz" | "je" => Token::JzKw,
                    "jnz" | "jne" => Token::JnzKw,
                    "jc" => Token::JcKw,
                    "jnc" => Token::JncKw,
                    "db" => Token::DbKw,
                    "dw" => Token::DwKw,
                    "dd" => Token::DdKw,
                    "times" => Token::TimesKw,

                    _ => Token::Identifier(ident),
                }
            }

            Some(ch) => {
                eprintln!("Carácter inesperado: {}", ch);
                self.advance();
                self.next_token()
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = matches!(token, Token::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("def main():");
        assert_eq!(lexer.next_token(), Token::Def);
        assert_eq!(lexer.next_token(), Token::Identifier("main".to_string()));
        assert_eq!(lexer.next_token(), Token::LParen);
        assert_eq!(lexer.next_token(), Token::RParen);
        assert_eq!(lexer.next_token(), Token::Colon);
    }

    #[test]
    fn test_string() {
        let mut lexer = Lexer::new(r#""Hello, World!""#);
        assert_eq!(
            lexer.next_token(),
            Token::String("Hello, World!".to_string())
        );
    }

    #[test]
    fn test_rust_style() {
        let mut lexer = Lexer::new("fn main() { let x = 42; }");
        assert_eq!(lexer.next_token(), Token::Fn);
        assert_eq!(lexer.next_token(), Token::Identifier("main".to_string()));
        assert_eq!(lexer.next_token(), Token::LParen);
        assert_eq!(lexer.next_token(), Token::RParen);
        assert_eq!(lexer.next_token(), Token::LBrace);
        assert_eq!(lexer.next_token(), Token::Let);
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 -10");
        assert_eq!(lexer.next_token(), Token::Number(42));
        assert_eq!(lexer.next_token(), Token::Float(3.14));
        assert_eq!(lexer.next_token(), Token::Minus);
        assert_eq!(lexer.next_token(), Token::Number(10));
    }

    #[test]
    fn test_comparisons() {
        let mut lexer = Lexer::new("== != < > <= >=");
        assert_eq!(lexer.next_token(), Token::EqEq);
        assert_eq!(lexer.next_token(), Token::NotEq);
        assert_eq!(lexer.next_token(), Token::Less);
        assert_eq!(lexer.next_token(), Token::Greater);
        assert_eq!(lexer.next_token(), Token::LessEq);
        assert_eq!(lexer.next_token(), Token::GreaterEq);
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("if else while for return true false");
        assert_eq!(lexer.next_token(), Token::If);
        assert_eq!(lexer.next_token(), Token::Else);
        assert_eq!(lexer.next_token(), Token::While);
        assert_eq!(lexer.next_token(), Token::For);
        assert_eq!(lexer.next_token(), Token::Return);
        assert_eq!(lexer.next_token(), Token::True);
        assert_eq!(lexer.next_token(), Token::False);
    }

    #[test]
    fn test_input() {
        let mut lexer = Lexer::new("input()");
        assert_eq!(lexer.next_token(), Token::Input);
        assert_eq!(lexer.next_token(), Token::LParen);
        assert_eq!(lexer.next_token(), Token::RParen);
    }

    #[test]
    fn test_line_tracking() {
        let mut lexer = Lexer::new("fn\nmain");
        assert_eq!(lexer.get_line(), 1);
        lexer.next_token(); // fn
        lexer.next_token(); // newline
        assert_eq!(lexer.get_line(), 2);
    }
}
