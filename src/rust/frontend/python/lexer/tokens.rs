#[derive(Debug, Clone, PartialEq)]
pub enum PyToken {
    // ── Keywords ──────────────────────────────────────────
    False, None, True, And, As, Assert, Async, Await,
    Break, Class, Continue, Def, Del, Elif, Else, Except,
    Finally, For, From, Global, If, Import, In, Is,
    Lambda, Nonlocal, Not, Or, Pass, Raise, Return,
    Try, While, With, Yield, Match, Case,
    Print, Exec,

    // ── Identifiers and Literals ─────────────────────────
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BytesLiteral(Vec<u8>),
    FStringStart(String),
    BoolLiteral(bool),

    // ── Operators ────────────────────────────────────────
    Plus, Minus, Star, DoubleStar, Slash, DoubleSlash,
    Percent, At, Ampersand, Pipe, Caret, Tilde,
    ColonAssign, Less, Greater, LessEq, GreaterEq, EqEq, NotEq,
    LShift, RShift,

    // ── Assignment Operators ─────────────────────────────
    Assign, PlusAssign, MinusAssign, StarAssign, SlashAssign,
    DoubleSlashAssign, PercentAssign, DoubleStarAssign,
    AmpAssign, PipeAssign, CaretAssign, LShiftAssign,
    RShiftAssign, AtAssign,

    // ── Delimiters ───────────────────────────────────────
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Comma, Colon, Semicolon, Dot, Ellipsis, Arrow, Backslash,

    // ── Indentation ──────────────────────────────────────
    Indent, Dedent, Newline,

    // ── Special ──────────────────────────────────────────
    Comment(String),
    Decorator(String),
    Eof,
}
