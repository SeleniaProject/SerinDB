use crate::ast::{Select, SelectItem, Statement};
use crate::token::{Lexer, Token};
use thiserror::Error;

/// Parsing error with location info.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Unexpected end-of-input.
    #[error("unexpected end of input")]
    Eof,
    /// Unexpected token.
    #[error("unexpected token: {0:?}")]
    Unexpected(Token),
}

/// Parse an SQL string into an AST [`Statement`].
pub fn parse(sql: &str) -> Result<Statement, ParseError> {
    let mut lex = Lexer::new(sql).peekable();
    match lex.peek().ok_or(ParseError::Eof)?.kind {
        Token::Select => parse_select(&mut lex),
        tok => Err(ParseError::Unexpected(tok)),
    }
}

fn parse_select(
    lex: &mut std::iter::Peekable<impl Iterator<Item = crate::token::LexItem>>,
) -> Result<Statement, ParseError> {
    // consume SELECT
    lex.next();

    // Handle projection
    let mut projection = Vec::new();
    loop {
        let item = match lex.peek().ok_or(ParseError::Eof)?.kind {
            Token::Star => {
                lex.next();
                SelectItem::Star
            }
            Token::Number => {
                let num: i64 = lex.next().unwrap().span.start as i64; // placeholder parse slice later
                SelectItem::Number(num)
            }
            tok => return Err(ParseError::Unexpected(tok)),
        };
        projection.push(item);

        match lex.peek() {
            Some(item) if item.kind == Token::Comma => {
                lex.next();
                continue;
            }
            _ => break,
        }
    }

    // Optional SEMICOLON
    if let Some(item) = lex.peek() {
        if item.kind == Token::Semicolon {
            lex.next();
        }
    }

    Ok(Statement::Select(Select { projection }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_select() {
        let stmt = parse("SELECT *;").unwrap();
        match stmt {
            Statement::Select(sel) => {
                assert_eq!(sel.projection, vec![SelectItem::Star]);
            }
            _ => panic!("expected select"),
        }
    }
} 