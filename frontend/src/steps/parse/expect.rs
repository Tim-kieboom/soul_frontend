use crate::steps::{parse::parser::Parser, tokenize::token_stream::TokenKind};
use models::{abstract_syntax_tree::statment::Ident, error::{SoulError, SoulErrorKind, SoulResult, Span}, soul_names};

const ILIGAL_NAMES: &[&[&str]] = &[
    soul_names::KeyWord::VALUES,
    soul_names::TypeModifier::VALUES,
];

impl<'a> Parser<'a> {
    
    pub(crate) fn try_consume_name(&mut self, start_span: Span) -> SoulResult<Option<Ident>> {
        
        let ident = match &self.token().kind {
            TokenKind::Ident(val) => val,
            _ => return Ok(None),
        };

        let mut names = ILIGAL_NAMES.iter()
            .copied()
            .flatten();

        if names.any(|name| name == ident) {

            return Err(
                SoulError::new(
                    format!("ident: '{}' is not allowed as name", ident), 
                    SoulErrorKind::InvalidName, 
                    Some(self.new_span(start_span)),
                )
            )
        } 

        let token = self.bump_consume();
        match token.kind {
            TokenKind::Ident(ident) => Ok(Some(ident)),
            _ => Ok(None),
        }
    }
}