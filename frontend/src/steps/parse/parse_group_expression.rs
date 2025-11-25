use crate::steps::{parse::{parse_statement::{COMMA, ROUND_CLOSE, ROUND_OPEN, SEMI_COLON}, parser::Parser}, tokenize::token_stream::TokenKind};
use models::{abstract_syntax_tree::expression_groups::Tuple, error::SoulResult};

impl<'a> Parser<'a> {

    pub fn parse_tuple(&mut self) -> SoulResult<Tuple> {
        self.expect(&ROUND_OPEN)?;
        
        let mut values = vec![];

        loop {

            self.skip_end_lines();
            
            let element = self.parse_expression(&[ROUND_CLOSE, COMMA])?;
            values.push(element);

            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break
            }
            self.expect(&COMMA)?;
        }

        self.expect(&ROUND_CLOSE)?;
        self.expect_any(&[TokenKind::EndLine, SEMI_COLON])?;
        Ok(
            Tuple{values}
        )
    }
}