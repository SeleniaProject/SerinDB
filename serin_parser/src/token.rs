use logos::Logos;

/// Position range of a token (byte offset).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Span {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

/// SQL token kinds recognised by SerinDB lexer.
#[derive(Logos, Debug, PartialEq, Clone, Copy)]
#[logos(skip r"[ \t\n\r]+", error = Error)]
pub enum Token {
    /// `SELECT` keyword.
    #[token("SELECT", ignore(ascii_case))]
    Select,
    /// `INSERT` keyword.
    #[token("INSERT", ignore(ascii_case))]
    Insert,
    /// `UPDATE` keyword.
    #[token("UPDATE", ignore(ascii_case))]
    Update,
    /// `DELETE` keyword.
    #[token("DELETE", ignore(ascii_case))]
    Delete,
    /// `FROM` keyword.
    #[token("FROM", ignore(ascii_case))]
    From,
    /// `WHERE` keyword.
    #[token("WHERE", ignore(ascii_case))]
    Where,
    /// Comma `,`.
    #[token(",")]
    Comma,
    /// Asterisk `*`.
    #[token("*")]
    Star,
    /// Semicolon `;`.
    #[token(";")]
    Semicolon,
    /// Left parenthesis `(`.
    #[token("(")]
    LParen,
    /// Right parenthesis `)`.
    #[token(")")]
    RParen,
    /// Numeric literal.
    #[regex(r"[0-9]+", |lex| lex.slice().parse())]
    Number,
    /// String literal.
    #[regex(r#"'([^']*)'"#, |lex| lex.slice()[1..lex.slice().len()-1].to_string())]
    String,
    /// Identifier (table/column).
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
    /// Unrecognised token.
    Error,
}

/// Output of the lexer containing token and span.
#[derive(Debug, Clone, PartialEq)]
pub struct LexItem {
    /// Token kind.
    pub kind: Token,
    /// Text span.
    pub span: Span,
}

/// Lexer iterator over `LexItem`s.
pub struct Lexer<'input> {
    inner: logos::Lexer<'input, Token>,
}

impl<'input> Lexer<'input> {
    /// Create new lexer from SQL text slice.
    pub fn new(source: &'input str) -> Self {
        Self {
            inner: Token::lexer(source),
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = LexItem;

    fn next(&mut self) -> Option<Self::Item> {
        let kind = self.inner.next()?;
        let span = Span {
            start: self.inner.span().start,
            end: self.inner.span().end,
        };
        Some(LexItem { kind, span })
    }
} 