use ast_model::{
    expression::{AnyArray, Array, ArrayFiller, Expression, StructConstructor},
    soul_type::SoulType,
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident,
    collections::try_result::TryError,
    error::SoulResult,
    fault::Fault,
    soul_names::Symbol,
    span::{Span, Spanned},
};

use crate::{
    parser::Parser,
    utils::{
        ARRAY, COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, FOR, LAMBDA_ARROW, SQUARE_CLOSE, SQUARE_OPEN,
    },
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_array(
        &mut self,
        collection_type: Option<SoulType>,
    ) -> SoulResult<Spanned<AnyArray>> {
        let start_span = self.token().span;
        if self.current_is(&ARRAY) {
            self.bump();
            return Ok(Spanned::new(
                AnyArray::Array(Array::new(self.alloc_node(), collection_type)),
                self.span_combine(start_span),
            ));
        }

        self.expect(&SQUARE_OPEN)?;

        let position = self.tokens.current_position();
        let element_type = match self.try_parse_type() {
            Ok(ty) if self.current_is(&COLON) => {
                self.bump();
                self.skip_end_lines();
                Some(ty)
            }
            Ok(_) => {
                self.goto(position);
                None
            }
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(_)) => None,
        };

        if self.current_is_keyword(KeyWord::For) {
            self.parse_array_filler(collection_type, element_type, start_span)
                .map(AnyArray::from_array_filler)
        } else {
            self.parse_array_literal(collection_type, element_type, start_span)
                .map(AnyArray::from_array)
        }
    }

    pub(super) fn parse_struct_contructor(
        &mut self,
        ident: Ident,
        generics: Vec<SoulType>,
        start_span: Span,
    ) -> SoulResult<Spanned<StructConstructor>> {
        self.expect(&CURLY_OPEN)?;
        self.skip_end_lines();
        let struct_type = self.type_from_ident(ident, generics);

        if self.current_is(&CURLY_CLOSE) {
            self.bump();

            let ctor = StructConstructor {
                values: vec![],
                defaults: false,
                struct_type,
            };
            return Ok(Spanned::new(ctor, self.span_combine(start_span)));
        }

        let mut defaults = false;
        let mut values = vec![];
        loop {
            self.skip_end_lines();

            if self.current_is(&TokenKind::Symbol(Symbol::DoubleDot)) {
                if defaults {
                    return Err(Fault::error(
                        "StructConstructor already has '..'",
                        Some(self.token().span),
                    ));
                }

                defaults = true;
                self.bump();
                self.skip_end_lines();
                if !self.current_is(&CURLY_CLOSE) {
                    return Err(Fault::error(
                        "StructConstructor's '..' should only be used at the end expected '}'",
                        Some(self.token().span),
                    ));
                }
                break;
            }

            let ident = self.try_bump_consume_ident()?;
            let value = if self.current_is(&COMMA) || self.current_is(&CURLY_CLOSE) {
                let id = self.alloc_node();
                self.store
                    .insert_expression(Expression::new_variable(id, ident.clone()))
            } else {
                self.expect(&COLON)?;
                self.parse_expression_id(&[COMMA, CURLY_CLOSE])?
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
            values,
            defaults,
            struct_type,
        };
        Ok(Spanned::new(ctor, self.span_combine(start_span)))
    }

    fn parse_array_filler(
        &mut self,
        collection_type: Option<SoulType>,
        element_type: Option<SoulType>,
        start_span: Span,
    ) -> SoulResult<Spanned<ArrayFiller>> {
        self.expect(&FOR)?;
        let amount = self.parse_expression_id(&[LAMBDA_ARROW, SQUARE_CLOSE])?;
        self.expect(&LAMBDA_ARROW)?;
        let element = self.parse_expression_id(&[SQUARE_CLOSE])?;
        self.expect(&SQUARE_CLOSE)?;
        Ok(Spanned::new(
            ArrayFiller {
                amount,
                element,
                id: self.alloc_node(),
                element_type,
                collection_type,
                for_index: None,
            },
            self.span_combine(start_span),
        ))
    }

    fn parse_array_literal(
        &mut self,
        collection_type: Option<SoulType>,
        element_type: Option<SoulType>,
        start_span: Span,
    ) -> SoulResult<Spanned<Array>> {
        let mut values = vec![];
        loop {
            self.skip_end_lines();
            if self.current_is(&SQUARE_CLOSE) {
                break;
            }

            let element = self.parse_expression_id(&[SQUARE_CLOSE, COMMA])?;
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
                id: self.alloc_node(),
                collection_type,
                element_type,
                values,
            },
            self.span_combine(start_span),
        ))
    }
}
