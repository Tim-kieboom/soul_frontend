use std::mem::swap;

use ast_model::expression::{
    Constructor, Deref, Expression, ExpressionId, ExpressionKind, MatchMethod, MatchMethodArm, MatchMethodVariant, Ref, VariableExpression
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident,
    collections::try_result::{ToResult, TryError},
    error::SoulResult,
    fault::Fault,
    soul_names::Symbol,
    span::Span,
};

use crate::{
    parser::Parser,
    utils::{ARRAY, ARROW_LEFT, CURLY_OPEN, ELSE, MUT, NOT, NULL, PASS, POINTER, REF, ROUND_OPEN, SIZEOF, SQUARE_CLOSE, SQUARE_OPEN},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn access_index_expression(
        &mut self,
        left: &mut Expression,
        start_span: Span,
    ) -> SoulResult<()> {
        let index =
            self.parse_expression_id(&[SQUARE_CLOSE, TokenKind::EndLine, TokenKind::EndFile])?;
        self.expect(&SQUARE_CLOSE)?;

        let mut value = Expression::error();
        swap(left, &mut value);

        let id = self.store.insert_expression(value);
        *left = Expression::new_index(id, index, self.span_combine(start_span));
        Ok(())
    }

    pub(super) fn access_this_expression(
        &mut self,
        left: &mut Expression,
        start_span: Span,
    ) -> SoulResult<()> {
        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_define().merge_to_result()?
        } else {
            vec![]
        };

        match &self.current().kind {
            &PASS => {
                self.bump();
                let mut value = Expression::error();
                swap(left, &mut value);

                let id = self.store.insert_expression(value);
                *left = Expression::new(
                    ExpressionKind::Pass(id), 
                    self.span_combine(start_span),
                );

                return Ok(())
            }
            &SIZEOF => {
                self.bump();
                let mut value = Expression::error();
                swap(left, &mut value);

                let id = self.store.insert_expression(value);
                *left = Expression::new(
                    ExpressionKind::Sizeof(id), 
                    self.span_combine(start_span),
                );

                return Ok(())
            }
            &REF => {
                self.bump();
                let mut expression = Expression::error();
                swap(left, &mut expression);

                let is_mutable = self.current_is(&MUT);
                if is_mutable {
                    self.bump();
                }

                let value = self.store.insert_expression(expression);
                *left = Expression::new(
                    ExpressionKind::Ref(Ref{ id: None, is_mutable, value }), 
                    self.span_combine(start_span),
                );

                return Ok(())
            }
            &POINTER => {
                self.bump();
                let mut expression = Expression::error();
                swap(left, &mut expression);

                let is_mutable = self.current_is(&MUT);
                if is_mutable {
                    self.bump();
                }

                let value = self.store.insert_expression(expression);
                *left = Expression::new(
                    ExpressionKind::Deref(Deref{ id: None, value }), 
                    self.span_combine(start_span),
                );

                return Ok(())
            }
            &ROUND_OPEN => {
                let mut value = Expression::error();
                swap(left, &mut value);

                let name = match value.node {
                    ExpressionKind::Variable(VariableExpression { name, .. }) => name,
                    _ => return Err(Fault::error("should be ident", Some(value.span))),
                };

                let arguments = self.parse_arguments()?;
                let ty = self.type_from_ident(name, generics);
                let ctor = Constructor {
                    id: None,
                    ty,
                    arguments,
                };
                *left = Expression::new(
                    ExpressionKind::Constructor(ctor),
                    self.span_combine(start_span),
                );

                return Ok(())
            }
            _ => (),
        };

        if self.current_is_any(&[SQUARE_OPEN, ARRAY]) {
            if !matches!(left.node, ExpressionKind::Variable(_)) {
                return Err(Fault::error(
                    format!("`{}` is invalid", Symbol::Dot.as_str()),
                    Some(self.current().span),
                ));
            }

            let mut value = Expression::error();
            std::mem::swap(left, &mut value);
            let name = match value.node {
                ExpressionKind::Variable(variable) => variable.name,
                _ => unreachable!(),
            };
            let collection_type = self.type_from_ident(name, generics);
            *left = Expression::from_any_array(self.parse_array(Some(collection_type))?);
            return Ok(());
        }

        let success = self.try_parse_method_arm(left, start_span)?;
        if success {
            return Ok(());
        }

        let ident = self.try_bump_consume_ident()?;

        let mut value = Expression::error();
        swap(left, &mut value);

        let id = self.store.insert_expression(value);
        *left = match self.try_parse_function_call_generic(start_span, Some(id), generics, &ident) {
            Ok(call) => Expression::from_function_call(call),
            Err(TryError::IsNotValue(_)) => self.parse_field_access(id, ident)?,
            Err(TryError::IsErr(err)) => return Err(err),
        };
        Ok(())
    }

    fn try_parse_method_arm(
        &mut self,
        left: &mut Expression,
        start_span: Span,
    ) -> SoulResult<bool> {

        let ident = match &self.current().kind {
            &ELSE if self.peek_is(&CURLY_OPEN) => {
                self.bump();
                MatchMethodVariant::Else
            }
            &NULL if self.peek_is(&CURLY_OPEN) => {
                self.bump();
                MatchMethodVariant::Null
            }
            &NOT if self.peek_is_multiple(&[NULL, CURLY_OPEN]) => {
                self.bump();
                self.bump();
                MatchMethodVariant::NotNull
            }
            TokenKind::Ident(_) if self.peek_is(&CURLY_OPEN) => {
                let ident = self.try_bump_consume_ident()?;
                MatchMethodVariant::Ident(ident)
            }
            _ => return Ok(false),
        };

        if !self.current_is(&CURLY_OPEN) {
            return Ok(false);
        }

        let (binding, body) = self.parse_match_method_arm(start_span)?;

        if let ExpressionKind::MatchMethod(ref mut method) = left.node {
            method.arms.push(MatchMethodArm {
                variant: ident,
                binding,
                body,
            });
            left.span = self.span_combine(start_span);
        } else {
            let mut value = Expression::error();
            swap(left, &mut value);

            let method = MatchMethod {
                id: None,
                scrutinee: self.store.insert_expression(value),
                arms: vec![MatchMethodArm {
                    variant: ident,
                    binding,
                    body,
                }],
            };
            *left = Expression::new(
                ExpressionKind::MatchMethod(method),
                self.span_combine(start_span),
            );
        }

        return Ok(true);
    }

    fn parse_field_access(&mut self, left: ExpressionId, ident: Ident) -> SoulResult<Expression> {
        match KeyWord::from_str(ident.as_str()) {
            Some(KeyWord::Sizeof) => self.parse_sizeof(left),
            _ => Ok(Expression::new_field(self.store, left, ident)),
        }
    }
}
