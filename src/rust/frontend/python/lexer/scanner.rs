use super::tokens::PyToken;

/// Python Lexer — indentation-aware tokenizer
pub struct PyLexer {
    source: Vec<char>,
    pos: usize,
    pub line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    pending_tokens: Vec<PyToken>,
    at_line_start: bool,
    paren_depth: usize,
}

impl PyLexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 0,
            indent_stack: vec![0],
            pending_tokens: Vec::new(),
            at_line_start: true,
            paren_depth: 0,
        }
    }

    /// Tokenize entire source into token stream
    pub fn tokenize(&mut self) -> Vec<PyToken> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            if tok == PyToken::Eof {
                while self.indent_stack.len() > 1 {
                    tokens.push(PyToken::Dedent);
                    self.indent_stack.pop();
                }
                tokens.push(PyToken::Eof);
                break;
            }
            tokens.push(tok);
        }
        tokens
    }

    /// Get next token
    pub fn next_token(&mut self) -> PyToken {
        if let Some(tok) = self.pending_tokens.pop() {
            return tok;
        }

        // Handle indentation at line start
        if self.at_line_start && self.paren_depth == 0 {
            self.at_line_start = false;
            let indent = self.count_indent();
            let current = *self.indent_stack.last().unwrap();

            if indent > current {
                self.indent_stack.push(indent);
                return PyToken::Indent;
            } else if indent < current {
                while self.indent_stack.len() > 1 && *self.indent_stack.last().unwrap() > indent {
                    self.indent_stack.pop();
                    self.pending_tokens.push(PyToken::Dedent);
                }
                if let Some(tok) = self.pending_tokens.pop() {
                    return tok;
                }
            }
        }

        self.skip_whitespace_inline();

        if self.pos >= self.source.len() {
            return PyToken::Eof;
        }

        let ch = self.source[self.pos];

        // Comments
        if ch == '#' {
            let comment = self.read_line_comment();
            return PyToken::Comment(comment);
        }

        // Newlines
        if ch == '\n' || ch == '\r' {
            self.consume_newline();
            self.at_line_start = true;
            if self.paren_depth > 0 {
                return self.next_token();
            }
            return PyToken::Newline;
        }

        // Line continuation
        if ch == '\\' && self.peek_at(1) == Some('\n') {
            self.pos += 2;
            self.line += 1;
            self.col = 0;
            return self.next_token();
        }

        // Strings
        if ch == '\'' || ch == '"'
            || (matches!(ch, 'f' | 'F' | 'b' | 'B' | 'r' | 'R' | 'u' | 'U') && self.is_string_start(1))
        {
            return self.read_string();
        }

        // Numbers
        if ch.is_ascii_digit() || (ch == '.' && self.peek_at(1).map_or(false, |c| c.is_ascii_digit())) {
            return self.read_number();
        }

        // Ellipsis
        if ch == '.' && self.peek_at(1) == Some('.') && self.peek_at(2) == Some('.') {
            self.pos += 3;
            self.col += 3;
            return PyToken::Ellipsis;
        }

        // Identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == '_' {
            return self.read_identifier();
        }

        // Operators and delimiters
        self.read_operator()
    }

    // ── Helpers ──────────────────────────────────────────

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.source.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.pos];
        self.pos += 1;
        self.col += 1;
        ch
    }

    fn count_indent(&mut self) -> usize {
        let mut indent = 0;
        while self.pos < self.source.len() {
            match self.source[self.pos] {
                ' ' => { indent += 1; self.pos += 1; self.col += 1; }
                '\t' => { indent += 4; self.pos += 1; self.col += 4; }
                '\n' | '\r' => {
                    self.consume_newline();
                    indent = 0;
                }
                '#' => {
                    while self.pos < self.source.len() && self.source[self.pos] != '\n' {
                        self.pos += 1;
                    }
                    if self.pos < self.source.len() {
                        self.consume_newline();
                    }
                    indent = 0;
                }
                _ => break,
            }
        }
        indent
    }

    fn skip_whitespace_inline(&mut self) {
        while self.pos < self.source.len() && matches!(self.source[self.pos], ' ' | '\t') {
            self.pos += 1;
            self.col += 1;
        }
    }

    fn consume_newline(&mut self) {
        if self.pos < self.source.len() && self.source[self.pos] == '\r' {
            self.pos += 1;
        }
        if self.pos < self.source.len() && self.source[self.pos] == '\n' {
            self.pos += 1;
        }
        self.line += 1;
        self.col = 0;
    }

    fn is_string_start(&self, offset: usize) -> bool {
        match self.source.get(self.pos + offset) {
            Some('\'' | '"') => true,
            Some('r' | 'R' | 'b' | 'B' | 'f' | 'F') => self.is_string_start(offset + 1),
            _ => false,
        }
    }

    fn read_line_comment(&mut self) -> String {
        let mut comment = String::new();
        self.pos += 1;
        while self.pos < self.source.len() && self.source[self.pos] != '\n' {
            comment.push(self.source[self.pos]);
            self.pos += 1;
        }
        comment.trim().to_string()
    }

    fn read_string(&mut self) -> PyToken {
        let mut is_fstring = false;
        let mut is_bytes = false;

        // Collect prefix
        while self.pos < self.source.len() {
            match self.source[self.pos] {
                'f' | 'F' => { is_fstring = true; self.pos += 1; }
                'b' | 'B' => { is_bytes = true; self.pos += 1; }
                'r' | 'R' | 'u' | 'U' => { self.pos += 1; }
                _ => break,
            }
        }

        let quote = self.source[self.pos];
        let triple = self.peek_at(1) == Some(quote) && self.peek_at(2) == Some(quote);
        let mut content = String::new();

        if triple {
            self.pos += 3;
            self.col += 3;
            while self.pos + 2 < self.source.len() {
                if self.source[self.pos] == quote
                    && self.source[self.pos + 1] == quote
                    && self.source[self.pos + 2] == quote
                {
                    self.pos += 3;
                    self.col += 3;
                    break;
                }
                if self.source[self.pos] == '\n' {
                    self.line += 1;
                    self.col = 0;
                }
                content.push(self.source[self.pos]);
                self.pos += 1;
            }
        } else {
            self.pos += 1;
            self.col += 1;
            while self.pos < self.source.len() && self.source[self.pos] != quote {
                if self.source[self.pos] == '\\' && self.pos + 1 < self.source.len() {
                    content.push(self.source[self.pos]);
                    self.pos += 1;
                    content.push(self.source[self.pos]);
                    self.pos += 1;
                    self.col += 2;
                    continue;
                }
                content.push(self.source[self.pos]);
                self.pos += 1;
                self.col += 1;
            }
            if self.pos < self.source.len() {
                self.pos += 1;
                self.col += 1;
            }
        }

        if is_fstring {
            PyToken::FStringStart(content)
        } else if is_bytes {
            PyToken::BytesLiteral(content.into_bytes())
        } else {
            PyToken::StringLiteral(content)
        }
    }

    fn read_number(&mut self) -> PyToken {
        let start = self.pos;

        // Hex, octal, binary
        if self.source[self.pos] == '0' && self.pos + 1 < self.source.len() {
            match self.source[self.pos + 1] {
                'x' | 'X' => {
                    self.pos += 2;
                    while self.pos < self.source.len()
                        && (self.source[self.pos].is_ascii_hexdigit() || self.source[self.pos] == '_')
                    {
                        self.pos += 1;
                    }
                    let s: String = self.source[start..self.pos].iter().filter(|c| **c != '_').collect();
                    self.col += self.pos - start;
                    return PyToken::IntLiteral(i64::from_str_radix(&s[2..], 16).unwrap_or(0));
                }
                'o' | 'O' => {
                    self.pos += 2;
                    while self.pos < self.source.len()
                        && (self.source[self.pos].is_ascii_digit() || self.source[self.pos] == '_')
                    {
                        self.pos += 1;
                    }
                    let s: String = self.source[start..self.pos].iter().filter(|c| **c != '_').collect();
                    self.col += self.pos - start;
                    return PyToken::IntLiteral(i64::from_str_radix(&s[2..], 8).unwrap_or(0));
                }
                'b' | 'B' => {
                    self.pos += 2;
                    while self.pos < self.source.len()
                        && matches!(self.source[self.pos], '0' | '1' | '_')
                    {
                        self.pos += 1;
                    }
                    let s: String = self.source[start..self.pos].iter().filter(|c| **c != '_').collect();
                    self.col += self.pos - start;
                    return PyToken::IntLiteral(i64::from_str_radix(&s[2..], 2).unwrap_or(0));
                }
                _ => {}
            }
        }

        let mut is_float = false;
        while self.pos < self.source.len()
            && (self.source[self.pos].is_ascii_digit() || self.source[self.pos] == '_')
        {
            self.pos += 1;
        }

        if self.pos < self.source.len() && self.source[self.pos] == '.' {
            is_float = true;
            self.pos += 1;
            while self.pos < self.source.len()
                && (self.source[self.pos].is_ascii_digit() || self.source[self.pos] == '_')
            {
                self.pos += 1;
            }
        }

        if self.pos < self.source.len() && matches!(self.source[self.pos], 'e' | 'E') {
            is_float = true;
            self.pos += 1;
            if self.pos < self.source.len() && matches!(self.source[self.pos], '+' | '-') {
                self.pos += 1;
            }
            while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        if self.pos < self.source.len() && matches!(self.source[self.pos], 'j' | 'J') {
            is_float = true;
            self.pos += 1;
        }

        let s: String = self.source[start..self.pos].iter().filter(|c| **c != '_').collect();
        self.col += self.pos - start;

        if is_float {
            let s_clean = s.trim_end_matches(|c| c == 'j' || c == 'J');
            PyToken::FloatLiteral(s_clean.parse::<f64>().unwrap_or(0.0))
        } else {
            PyToken::IntLiteral(s.parse::<i64>().unwrap_or(0))
        }
    }

    fn read_identifier(&mut self) -> PyToken {
        let start = self.pos;
        while self.pos < self.source.len()
            && (self.source[self.pos].is_ascii_alphanumeric() || self.source[self.pos] == '_')
        {
            self.pos += 1;
        }
        let word: String = self.source[start..self.pos].iter().collect();
        self.col += self.pos - start;

        match word.as_str() {
            "False"    => PyToken::False,
            "None"     => PyToken::None,
            "True"     => PyToken::True,
            "and"      => PyToken::And,
            "as"       => PyToken::As,
            "assert"   => PyToken::Assert,
            "async"    => PyToken::Async,
            "await"    => PyToken::Await,
            "break"    => PyToken::Break,
            "class"    => PyToken::Class,
            "continue" => PyToken::Continue,
            "def"      => PyToken::Def,
            "del"      => PyToken::Del,
            "elif"     => PyToken::Elif,
            "else"     => PyToken::Else,
            "except"   => PyToken::Except,
            "finally"  => PyToken::Finally,
            "for"      => PyToken::For,
            "from"     => PyToken::From,
            "global"   => PyToken::Global,
            "if"       => PyToken::If,
            "import"   => PyToken::Import,
            "in"       => PyToken::In,
            "is"       => PyToken::Is,
            "lambda"   => PyToken::Lambda,
            "nonlocal" => PyToken::Nonlocal,
            "not"      => PyToken::Not,
            "or"       => PyToken::Or,
            "pass"     => PyToken::Pass,
            "raise"    => PyToken::Raise,
            "return"   => PyToken::Return,
            "try"      => PyToken::Try,
            "while"    => PyToken::While,
            "with"     => PyToken::With,
            "yield"    => PyToken::Yield,
            "match"    => PyToken::Match,
            "case"     => PyToken::Case,
            "print"    => PyToken::Print,
            "exec"     => PyToken::Exec,
            _          => PyToken::Identifier(word),
        }
    }

    fn read_operator(&mut self) -> PyToken {
        let ch = self.advance();
        match ch {
            '(' => { self.paren_depth += 1; PyToken::LParen }
            ')' => { if self.paren_depth > 0 { self.paren_depth -= 1; } PyToken::RParen }
            '[' => { self.paren_depth += 1; PyToken::LBracket }
            ']' => { if self.paren_depth > 0 { self.paren_depth -= 1; } PyToken::RBracket }
            '{' => { self.paren_depth += 1; PyToken::LBrace }
            '}' => { if self.paren_depth > 0 { self.paren_depth -= 1; } PyToken::RBrace }
            ',' => PyToken::Comma,
            ';' => PyToken::Semicolon,
            '~' => PyToken::Tilde,
            '.' => PyToken::Dot,
            '+' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::PlusAssign }
                else { PyToken::Plus }
            }
            '-' => {
                if self.peek_at(0) == Some('>') { self.advance(); PyToken::Arrow }
                else if self.peek_at(0) == Some('=') { self.advance(); PyToken::MinusAssign }
                else { PyToken::Minus }
            }
            '*' => {
                if self.peek_at(0) == Some('*') {
                    self.advance();
                    if self.peek_at(0) == Some('=') { self.advance(); PyToken::DoubleStarAssign }
                    else { PyToken::DoubleStar }
                } else if self.peek_at(0) == Some('=') { self.advance(); PyToken::StarAssign }
                else { PyToken::Star }
            }
            '/' => {
                if self.peek_at(0) == Some('/') {
                    self.advance();
                    if self.peek_at(0) == Some('=') { self.advance(); PyToken::DoubleSlashAssign }
                    else { PyToken::DoubleSlash }
                } else if self.peek_at(0) == Some('=') { self.advance(); PyToken::SlashAssign }
                else { PyToken::Slash }
            }
            '%' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::PercentAssign }
                else { PyToken::Percent }
            }
            '@' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::AtAssign }
                else if self.peek_at(0).map_or(false, |c| c.is_ascii_alphabetic() || c == '_') {
                    let start = self.pos;
                    while self.pos < self.source.len()
                        && (self.source[self.pos].is_ascii_alphanumeric()
                            || self.source[self.pos] == '_'
                            || self.source[self.pos] == '.')
                    {
                        self.pos += 1;
                    }
                    let name: String = self.source[start..self.pos].iter().collect();
                    self.col += self.pos - start;
                    PyToken::Decorator(name)
                } else { PyToken::At }
            }
            '&' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::AmpAssign }
                else { PyToken::Ampersand }
            }
            '|' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::PipeAssign }
                else { PyToken::Pipe }
            }
            '^' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::CaretAssign }
                else { PyToken::Caret }
            }
            '<' => {
                if self.peek_at(0) == Some('<') {
                    self.advance();
                    if self.peek_at(0) == Some('=') { self.advance(); PyToken::LShiftAssign }
                    else { PyToken::LShift }
                } else if self.peek_at(0) == Some('=') { self.advance(); PyToken::LessEq }
                else { PyToken::Less }
            }
            '>' => {
                if self.peek_at(0) == Some('>') {
                    self.advance();
                    if self.peek_at(0) == Some('=') { self.advance(); PyToken::RShiftAssign }
                    else { PyToken::RShift }
                } else if self.peek_at(0) == Some('=') { self.advance(); PyToken::GreaterEq }
                else { PyToken::Greater }
            }
            '=' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::EqEq }
                else { PyToken::Assign }
            }
            '!' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::NotEq }
                else { PyToken::Identifier("!".to_string()) }
            }
            ':' => {
                if self.peek_at(0) == Some('=') { self.advance(); PyToken::ColonAssign }
                else { PyToken::Colon }
            }
            '\\' => PyToken::Backslash,
            _ => PyToken::Identifier(ch.to_string()),
        }
    }
}
