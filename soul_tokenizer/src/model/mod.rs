pub mod keyword;
pub mod symbol;
pub mod types;

use crate::model::{keyword::{KeyWord}, types::Types, symbol::Symbol};
use soul_utils::{literal::Literal, span::Span};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TokenKind {
    Literal(Literal),
    Keyword(KeyWord),
    Symbol(Symbol),
    Ident(String),
    Types(Types),
    EndLine,
    EndFile,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}