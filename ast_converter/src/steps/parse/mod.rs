use crate::steps::{parse::parser::Parser, tokenize::token_stream::TokenKind};
use soul_utils::SymboolKind;

pub mod parser;

mod expect;
mod parse_conditionals;
mod parse_expression;
mod parse_function;
mod parse_group_expression;
mod parse_objects;
mod parse_type;
mod statements;

pub type Request<'a> = crate::steps::tokenize::Response<'a>;

#[derive(Debug)]
pub(crate) struct Response<'a> {
    pub parser: Parser<'a>,
}

pub const MUT_REF: TokenKind = TokenKind::Symbool(SymboolKind::And);
pub const COMMA: TokenKind = TokenKind::Symbool(SymboolKind::Comma);
pub const COLON: TokenKind = TokenKind::Symbool(SymboolKind::Colon);
pub const ASSIGN: TokenKind = TokenKind::Symbool(SymboolKind::Assign);
pub const CONST_REF: TokenKind = TokenKind::Symbool(SymboolKind::ConstRef);
pub const CURLY_OPEN: TokenKind = TokenKind::Symbool(SymboolKind::CurlyOpen);
pub const ROUND_OPEN: TokenKind = TokenKind::Symbool(SymboolKind::RoundOpen);
pub const ARROW_LEFT: TokenKind = TokenKind::Symbool(SymboolKind::LeftArray);
pub const SEMI_COLON: TokenKind = TokenKind::Symbool(SymboolKind::SemiColon);
pub const INCREMENT: TokenKind = TokenKind::Symbool(SymboolKind::DoublePlus);
pub const DECREMENT: TokenKind = TokenKind::Symbool(SymboolKind::DoubleMinus);
pub const ARROW_RIGHT: TokenKind = TokenKind::Symbool(SymboolKind::RightArray);
pub const SQUARE_OPEN: TokenKind = TokenKind::Symbool(SymboolKind::SquareOpen);
pub const CURLY_CLOSE: TokenKind = TokenKind::Symbool(SymboolKind::CurlyClose);
pub const ROUND_CLOSE: TokenKind = TokenKind::Symbool(SymboolKind::RoundClose);
pub const SQUARE_CLOSE: TokenKind = TokenKind::Symbool(SymboolKind::SquareClose);
pub const COLON_ASSIGN: TokenKind = TokenKind::Symbool(SymboolKind::ColonAssign);
pub const STAMENT_END_TOKENS: &[TokenKind] = &[
    CURLY_CLOSE,
    TokenKind::EndFile,
    TokenKind::EndLine,
    TokenKind::Symbool(SymboolKind::SemiColon),
];
