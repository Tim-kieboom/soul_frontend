use crate::steps::{parse::{parse_statement::{COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN, SEMI_COLON}, parser::{Parser, TryError, TryResult}}, tokenize::token_stream::TokenKind};
use models::{abstract_syntax_tree::expression_groups::{NamedTuple, Tuple}, error::{SoulError, SoulErrorKind, SoulResult}, symbool_kind::SymboolKind};

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

    pub fn try_parse_named_tuple(&mut self) -> TryResult<NamedTuple, SoulError> {
        let begin_position = self.current_position();
        let result = self.inner_try_parse_named_tuple();
        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    fn inner_try_parse_named_tuple(&mut self) -> TryResult<NamedTuple, SoulError> {
        const INSERT_DEFAULTS: TokenKind = TokenKind::Symbool(SymboolKind::DoubleDot);
        let start_span = self.token().span;
        self.expect(&CURLY_OPEN)
            .map_err(|err| TryError::IsNotValue(err))?;

        let mut values = vec![];
        let mut insert_defaults = false;
        loop {

            self.skip_end_lines();

            if self.current_is(&INSERT_DEFAULTS) {
                self.bump();
                insert_defaults = true;

                if self.current_is(&CURLY_CLOSE) {
                    break
                }

                return Err(TryError::IsErr(SoulError::new(
                    "token after '..' has to be '}'",
                    SoulErrorKind::InvalidEscapeSequence,
                    Some(self.new_span(start_span)),
                )))
            }

            let ident_token = self.bump_consume();
            let ident = match ident_token.kind {
                TokenKind::Ident(ident) => ident,
                _ => return Err(TryError::IsNotValue(SoulError::new(
                    format!("expected ident but found: '{:?}'", self.token().kind), 
                    SoulErrorKind::InvalidIdent, 
                    Some(self.new_span(start_span)),
                )))
            };

            self.expect(&COLON)
                .map_err(|err| TryError::IsNotValue(err))?;

            let element = self.parse_expression(&[CURLY_CLOSE, COMMA])
                .map_err(|err| TryError::IsNotValue(err))?;

            values.push((ident, element));

            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break
            }

            self.expect(&COMMA)
                .map_err(|err| TryError::IsErr(err))?;
        }

        self.expect(&CURLY_CLOSE)
            .map_err(|err| TryError::IsErr(err))?;

        self.expect_any(&[TokenKind::EndLine, SEMI_COLON])
            .map_err(|err| TryError::IsErr(err))?;

        Ok(
            NamedTuple{values, insert_defaults}
        )
    }
}