pub(crate) mod keyword;
pub(crate) mod symbol;

use crate::model::{keyword::KeyWord, symbol::Symbol};
use soul_utils::{literal::Literal, span::Span};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TokenKind {
    Literal(Literal),
    Keyword(KeyWord),
    Symbol(Symbol),
    Ident(String),
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