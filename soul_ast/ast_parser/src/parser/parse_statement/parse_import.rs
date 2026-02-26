use crate::parser::Parser;
use ast::Statement;
use soul_utils::error::SoulResult;

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_import(&mut self) -> SoulResult<Statement> {
        todo!("impl import parsing")
    }
}
