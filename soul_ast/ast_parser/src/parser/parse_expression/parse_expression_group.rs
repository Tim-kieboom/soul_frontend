use ast::{Array, Expression, SoulType, StructConstructor};
use soul_tokenizer::TokenKind;
use soul_utils::{Ident, error::{SoulError, SoulErrorKind, SoulResult}, span::{Span, Spanned}, symbool_kind::SymbolKind, try_result::TryError};

use crate::parser::{
    Parser,
    parse_utils::{COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, SQUARE_CLOSE, SQUARE_OPEN},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_array(&mut self, ty: Option<SoulType>) -> SoulResult<Spanned<Array>> {
        let start_span = self.token().span;
        self.expect(&SQUARE_OPEN)?;

        let element_type = match self.try_parse_type() {
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
        Ok(Spanned::new(
            Array {
                collection_type: ty,
                element_type,
                values,
                id: None,
            },
            self.span_combine(start_span),
        ))
    }

    pub(super) fn parse_struct_contructor(
        &mut self,
        ident: Ident,
        generics: Vec<SoulType>,
        start_span: Span,
    ) -> SoulResult<Spanned<StructConstructor>> {
        self.expect(&CURLY_OPEN)?;

        let struct_type = self.type_from_ident(ident, generics);

        let mut defaults = false;
        let mut values = vec![];
        loop {
            self.skip_end_lines();

            if self.current_is(&TokenKind::Symbol(SymbolKind::DoubleDot)) {
                if defaults {
                    return Err(SoulError::new(
                        "StructConstructor already has '..'",
                        SoulErrorKind::InvalidEscapeSequence,
                        Some(self.token().span),
                    ));
                }

                defaults = true;
                self.bump();
                self.skip_end_lines();
                if !self.current_is(&CURLY_CLOSE) {
                    return Err(SoulError::new(
                        "StructConstructor's '..' should only be used at the end expected '}'",
                        SoulErrorKind::InvalidEscapeSequence,
                        Some(self.token().span),
                    ));
                }
                break;
            }

            let ident = self.try_bump_consume_ident()?;
            let value = if self.current_is(&COMMA) || self.current_is(&CURLY_CLOSE) {
                Expression::new_variable(ident.clone())
            } else {
                self.expect(&COLON)?;
                self.parse_expression(&[COMMA, CURLY_CLOSE])?
            };

            values.push((ident, value));
            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }

            self.bump();
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            continue;
        }
        self.skip_end_lines();
        self.expect(&CURLY_CLOSE)?;

        let ctor = StructConstructor {
            id: None,
            values,
            defaults,
            struct_type,
        };
        Ok(Spanned::new(ctor, self.span_combine(start_span)))
    }

}
