use crate::{
    steps::{
        parse::{
            COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN, SEMI_COLON,
            SQUARE_CLOSE, SQUARE_OPEN, parser::Parser,
        },
        tokenize::token_stream::TokenKind,
    },
    utils::try_result::{ResultTryErr, TryErr, TryError, TryNotValue, TryResult},
};
use soul_ast::abstract_syntax_tree::{
    expression_groups::{Array, NamedTuple, Tuple},
    soul_type::SoulType,
    spanned::Spanned,
};
use soul_utils::{SoulError, SoulErrorKind, SoulResult, SymboolKind};

impl<'a> Parser<'a> {
    pub fn parse_tuple(&mut self) -> SoulResult<Tuple> {
        self.expect(&ROUND_OPEN)?;

        if self.current_is(&ROUND_CLOSE) {
            self.bump();
            return Ok(Tuple { values: vec![] });
        }

        let mut values = vec![];
        loop {
            self.skip_end_lines();

            let element = self.parse_expression(&[ROUND_CLOSE, COMMA])?;
            values.push(element);

            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }
            self.expect(&COMMA)?;
        }

        self.expect(&ROUND_CLOSE)?;
        Ok(Tuple { values })
    }

    pub fn parse_array(&mut self, collection_type: Option<SoulType>) -> SoulResult<Array> {
        self.expect(&SQUARE_OPEN)?;

        let element_type = match self.try_parse_type() {
            Ok(ty) => {
                self.expect(&COLON)?;
                Some(ty)
            }
            Err(TryError::IsNotValue(_)) => None,
            Err(TryError::IsErr(err)) => return Err(err),
        };

        let mut values = vec![];
        loop {
            self.skip_end_lines();
            if self.current_is(&SQUARE_CLOSE) {
                break;
            }

            let element = self.parse_expression(&[SQUARE_CLOSE, COMMA])?;
            values.push(element);

            self.skip_end_lines();
            if self.current_is(&SQUARE_CLOSE) {
                break;
            }
            self.expect(&COMMA)?;
        }

        self.expect(&SQUARE_CLOSE)?;
        self.expect_any(&[TokenKind::EndLine, SEMI_COLON])?;
        Ok(Array {
            values,
            element_type,
            collection_type,
        })
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
        self.expect(&CURLY_OPEN).try_err()?;

        let mut values = vec![];
        let mut insert_defaults = false;
        loop {
            self.skip_end_lines();

            if self.current_is(&INSERT_DEFAULTS) {
                self.bump();
                insert_defaults = true;

                if self.current_is(&CURLY_CLOSE) {
                    break;
                }

                return TryErr(SoulError::new(
                    "token after '..' has to be '}'",
                    SoulErrorKind::InvalidEscapeSequence,
                    Some(self.new_span(start_span)),
                ));
            }

            let ident_token = self.bump_consume();
            let ident = match ident_token.kind {
                TokenKind::Ident(ident) => ident,
                _ => {
                    return TryNotValue(SoulError::new(
                        format!("expected ident but found: '{:?}'", self.token().kind),
                        SoulErrorKind::InvalidIdent,
                        Some(self.new_span(start_span)),
                    ));
                }
            };

            self.expect(&COLON).map_err(TryError::IsNotValue)?;

            let element = self
                .parse_expression(&[CURLY_CLOSE, COMMA])
                .map_err(TryError::IsNotValue)?;

            values.push((Spanned::new(ident, ident_token.span), element));

            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            self.expect(&COMMA).try_err()?;
        }

        self.expect(&CURLY_CLOSE).try_err()?;

        self.expect_any(&[TokenKind::EndLine, SEMI_COLON])
            .try_err()?;

        Ok(NamedTuple {
            values,
            insert_defaults,
        })
    }
}
