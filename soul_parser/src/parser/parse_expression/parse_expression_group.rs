use parser_models::ast::{Array, NamedTuple, SoulType, Tuple};
use soul_tokenizer::TokenKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult}, soul_names::TypeModifier, span::Spanned, symbool_kind::SymbolKind, try_result::{ResultTryErr, ResultTryNotValue, TryErr, TryError, TryResult}
};

use crate::parser::{
    Parser,
    parse_utils::{
        COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN, SEMI_COLON, SQUARE_CLOSE,
        SQUARE_OPEN,
    },
};

impl<'a> Parser<'a> {
    pub fn parse_tuple(&mut self) -> SoulResult<Tuple> {
        self.expect(&ROUND_OPEN)?;

        let mut values = Tuple::new();
        if self.current_is(&ROUND_CLOSE) {
            self.bump();
            return Ok(values);
        }

        loop {
            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }
            
            let element = self.parse_expression(&[ROUND_CLOSE, COMMA])?;
            values.push(element);

            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }
            self.expect(&COMMA)?;
        }

        self.expect(&ROUND_CLOSE)?;
        Ok(values)
    }

    pub fn try_parse_named_tuple(&mut self) -> TryResult<Spanned<NamedTuple>, SoulError> {
        let begin = self.current_position();
        let result = self.inner_parse_named_tuple();
        if result.is_err() {
            self.go_to(begin);
        }

        result
    }

    pub fn parse_array(&mut self, ty: Option<SoulType>) -> SoulResult<Spanned<Array>> {
        const DEFAULT_MODIFIER: TypeModifier = TypeModifier::Mut;
        let start_span = self.token().span;
        self.expect(&SQUARE_OPEN)?;
        
        let element_type = match self.try_parse_type(DEFAULT_MODIFIER) {
            Ok(ty) => {
                self.expect(&COLON)?;
                self.skip_end_lines();
                Some(ty)
            }
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(_)) => None,
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

        self.skip_end_lines();
        self.expect(&SQUARE_CLOSE)?;
        self.expect_any(&[TokenKind::EndLine, SEMI_COLON])?;
        Ok(Spanned::new(
            Array {
                collection_type: ty,
                element_type,
                values,
            },
            self.span_combine(start_span),
        ))
    }

    fn inner_parse_named_tuple(&mut self) -> TryResult<Spanned<NamedTuple>, SoulError> {
        const INSERT_DEFAULTS: TokenKind = TokenKind::Symbol(SymbolKind::DoubleDot);
        let start_span = self.token().span;

        self.expect(&CURLY_OPEN).try_err()?;

        let mut values = vec![];
        let mut insert_defaults = false;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            if self.current_is(&INSERT_DEFAULTS) {
                self.bump();
                insert_defaults = true;

                self.skip_end_lines();
                if self.current_is(&CURLY_CLOSE) {
                    break;
                }

                return TryErr(SoulError::new(
                    "token after '..' has to be '}'",
                    SoulErrorKind::InvalidEscapeSequence,
                    Some(self.token().span),
                ));
            }

            let ident = self.try_bump_consume_ident().try_err()?;

            self.expect(&COLON).try_not_value()?;

            let element = self
                .parse_expression(&[CURLY_CLOSE, COMMA])
                .try_not_value()?;

            values.push((ident, element));

            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            self.expect(&COMMA).try_err()?;
        }

        self.skip_end_lines();
        self.expect(&CURLY_CLOSE).try_err()?;

        self.expect_any(&[TokenKind::EndLine, SEMI_COLON])
            .try_err()?;

        Ok(Spanned::new(
            NamedTuple {
                values,
                insert_defaults,
            },
            self.span_combine(start_span),
        ))
    }
}
