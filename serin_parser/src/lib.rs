//! SerinDB SQL parser crate.
#![deny(missing_docs)]

mod token;
mod ast;

pub use token::{Token, Lexer};
pub use ast::*;

pub mod parser;

pub use parser::{parse, ParseError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple_select() {
        let sql = "SELECT 1;";
        let tokens: Vec<_> = Lexer::new(sql).collect();
        let kinds: Vec<Token> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![Token::Select, Token::Number, Token::Semicolon]);
    }
} 