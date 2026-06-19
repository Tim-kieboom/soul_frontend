use ast_model::{
    expression::{Array, Expression, ExpressionKind},
    literal::Literal,
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    TypeModifier,
    collections::try_result::TryError,
    error::SoulResult,
    fault::Fault,
    literal::{Number, StringLiteral, TokenLiteral},
    soul_error_internal,
    span::{Span, Spanned},
};

use crate::{
    parser::Parser,
    utils::{ARRAY, ARROW_LEFT, COLON, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN, SQUARE_OPEN},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_primary(&mut self, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        let start_span = self.current().span;

        let expression = match &self.current().kind {
            &CURLY_OPEN => {
                let block = self.parse_block(TypeModifier::Mut)?;
                Expression::new_block(block, self.span_combine(start_span))
            }
            &SQUARE_OPEN => {
                let array = self.parse_array(None)?;
                Expression::from_any_array(array)
            }
            &ROUND_OPEN => {
                self.bump();
                let expr =
                    self.parse_expression(&[ROUND_CLOSE, TokenKind::EndLine, TokenKind::EndFile])?;
                self.expect(&ROUND_CLOSE)?;
                expr
            }
            &ARRAY => {
                self.bump();
                let arr = Array {
                    id: None,
                    collection_type: None,
                    element_type: None,
                    values: vec![],
                };
                Expression::from_array(Spanned::new(arr, start_span))
            }
            TokenKind::Ident(_) => self.parse_primary_ident(end_tokens, start_span)?,
            TokenKind::Keyword(keyword) => {
                let kw = *keyword;
                match self.parse_keyword_primary(start_span, kw)? {
                    Some(expr) => expr,
                    None => {
                        return Err(Fault::error(
                            format!("`{}` is invalid as start of expression", kw.as_str()),
                            Some(start_span),
                        ));
                    }
                }
            }
            TokenKind::Literal(TokenLiteral::Char(char)) => {
                let char = *char;
                self.bump();
                Expression::new_literal(Literal::Char(char), start_span)
            }
            TokenKind::Literal(TokenLiteral::String(_)) => {
                let token = self.bump_consume();
                let string = match token.kind {
                    TokenKind::Literal(TokenLiteral::String(val)) => val,
                    _ => unreachable!(),
                };
                match string {
                    StringLiteral::Cstr(string) => {
                        Expression::new_literal(Literal::Cstr(string), token.span)
                    }
                    StringLiteral::Str(string) => {
                        Expression::new_literal(Literal::Str(string), token.span)
                    }
                }
            }
            TokenKind::Literal(TokenLiteral::Number(num)) => {
                let number = match num {
                    Number::Int(val) => Literal::Int(*val as i128),
                    Number::Uint(val) => Literal::Uint(*val as u128),
                    Number::Float(val) => Literal::Float(*val),
                };
                self.bump();
                Expression::new_literal(number, start_span)
            }
            other => {
                return Err(Fault::error(
                    format!("`{}` is invalid as start of expression", other.display(),),
                    Some(start_span),
                ));
            }
        };

        Ok(expression)
    }

    pub(super) fn parse_primary_ident(
        &mut self,
        end_tokens: &[TokenKind],
        start_span: Span,
    ) -> SoulResult<Expression> {
        if let Some(primary) = self.parse_primary_keyword(start_span)? {
            return Ok(primary);
        }

        let ident = self.try_bump_consume_ident()?;
        let span = ident.span();

        let peek = self.peek();
        match &self.current().kind {
            &COLON if peek.kind == SQUARE_OPEN => {
                return Err(soul_error_internal!(
                    "collectionType array not yet impl",
                    Some(span)
                ));
            }
            &ROUND_OPEN | &ARROW_LEFT => {
                match self.try_parse_function_call(start_span, None, &ident) {
                    Ok(val) => return Ok(val),
                    Err(TryError::IsNotValue(_)) => (),
                    Err(TryError::IsErr(err)) => return Err(err),
                };

                match self.parse_generic_define() {
                    Ok(generics) => {
                        return self
                            .parse_struct_contructor(ident, generics, start_span)
                            .map(Expression::from_struct_contructor);
                    }
                    Err(TryError::IsNotValue(_)) => (),
                    Err(TryError::IsErr(err)) => return Err(err),
                }
            }
            &CURLY_OPEN if !end_tokens.contains(&CURLY_OPEN) => {
                return self
                    .parse_struct_contructor(ident, vec![], start_span)
                    .map(Expression::from_struct_contructor);
            }
            _ => (),
        };

        Ok(Expression::new_variable(ident))
    }

    pub(super) fn parse_keyword_primary(
        &mut self,
        start_span: Span,
        keyword: KeyWord,
    ) -> SoulResult<Option<Expression>> {
        Ok(Some(match keyword {
            KeyWord::If => self.parse_if()?,
            KeyWord::Match => self.parse_match()?,

            KeyWord::True | KeyWord::False => {
                let value = keyword == KeyWord::True;
                self.bump();
                Expression::new_literal(Literal::Bool(value), self.current().span)
            }

            KeyWord::Null => {
                self.bump();
                Expression::new(ExpressionKind::Null(None), self.current().span)
            }

            KeyWord::Undefined => {
                self.bump();
                Expression::new(ExpressionKind::Undefined(None), self.current().span)
            }

            KeyWord::Break | KeyWord::Return | KeyWord::Continue => {
                return Err(Fault::error(
                    format!("can not have {} in expression", keyword.as_str()),
                    Some(self.current().span),
                ));
            }

            KeyWord::New => {
                self.bump();
                match &self.current().kind {
                    &ROUND_OPEN => self.parse_new_ptr(start_span)?,
                    &SQUARE_OPEN | &ARRAY => self.parse_new_array(start_span)?,
                    _ => {
                        return Err(Fault::error(
                            "expected '(' or ':[' after 'new'".to_string(),
                            Some(self.current().span),
                        ));
                    }
                }
            }

            _ => return Ok(None),
        }))
    }

    fn parse_primary_keyword(&mut self, start_span: Span) -> SoulResult<Option<Expression>> {
        let ident = self.try_token_as_ident_str()?;
        match KeyWord::from_str(ident) {
            Some(keyword) => self.parse_keyword_primary(start_span, keyword),
            None => Ok(None),
        }
    }
}
