use std::str::FromStr;

use ast_model::{
    expression::{Array, Expression, ExpressionId, ExpressionKind, StringFormat},
    literal::Literal,
};
use soul_tokenizer::model::{StringFormatTag, TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident, TypeModifier,
    collections::try_result::TryError,
    error::SoulResult,
    fault::Fault,
    literal::{Number, StringLiteral, TokenLiteral},
    soul_error_internal,
    soul_names::Symbol,
    span::{Span, Spanned},
};

use crate::{
    parser::Parser,
    utils::{
        ARRAY, ARROW_LEFT, COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, DOT, ROUND_CLOSE, ROUND_OPEN,
        SQUARE_OPEN,
    },
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_primary(&mut self, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        let start_span = self.token().span;

        // Try lambda first: `param => body`
        let saved = self.tokens.current_position();
        if let Some(lambda) = self.try_parse_lambda(start_span) {
            return Ok(lambda);
        }
        self.goto(saved);

        if self.current_is(&ROUND_OPEN) && self.peek_is(&ROUND_CLOSE) {
            self.bump();
            self.bump();
            return Ok(Expression::new(
                ExpressionKind::None(self.alloc_node()),
                self.span_combine(start_span),
            ));
        }

        let expression = match &self.token().kind {
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
                let expr = self.parse_expression(&[COMMA, ROUND_CLOSE])?;
                if self.current_is(&COMMA) && !matches!(expr.node, ExpressionKind::Tuple(_)) {
                    let mut values = vec![self.forest.store.insert_expression(expr)];
                    loop {
                        self.skip_end_lines();
                        if !self.current_is(&COMMA) {
                            break;
                        }
                        self.bump();
                        self.skip_end_lines();
                        let expr = self.parse_expression(&[COMMA, ROUND_CLOSE])?;
                        values.push(self.forest.store.insert_expression(expr));
                    }
                    self.expect(&ROUND_CLOSE)?;
                    Expression::new(ExpressionKind::Tuple(values), self.span_combine(start_span))
                } else {
                    self.expect(&ROUND_CLOSE)?;
                    expr
                }
            }
            &ARRAY => {
                self.bump();
                let arr = Array {
                    id: self.alloc_node(),
                    collection_type: None,
                    element_type: None,
                    values: vec![],
                };
                Expression::from_array(Spanned::new(arr, start_span))
            }
            TokenKind::Ident(_) => self.parse_primary_ident(end_tokens, start_span)?,
            TokenKind::Types(_types) => {
                let name = _types.as_str().to_string();
                self.bump();
                if self.current_is(&ARROW_LEFT) {
                    match self.parse_generic_define() {
                        Ok(_generics) => (),
                        Err(TryError::IsNotValue(_)) => (),
                        Err(TryError::IsErr(err)) => return Err(err),
                    }
                }
                Expression::new_variable(self.alloc_node(), Ident::new(name, start_span))
            }
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
                Expression::new_literal(self.alloc_node(), Literal::Char(char), start_span)
            }
            TokenKind::Literal(TokenLiteral::String(_)) => {
                let token = self.bump_consume();
                let string = match token.kind {
                    TokenKind::Literal(TokenLiteral::String(val)) => val,
                    _ => unreachable!(),
                };
                match string {
                    StringLiteral::Cstr(string) => Expression::new_literal(
                        self.alloc_node(),
                        Literal::Cstr(string),
                        token.span,
                    ),
                    StringLiteral::Str(string) => {
                        Expression::new_literal(self.alloc_node(), Literal::Str(string), token.span)
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
                Expression::new_literal(self.alloc_node(), number, start_span)
            }
            TokenKind::StringFormat(fmt) => {
                let to_string = match fmt {
                    StringFormatTag::F => true,
                    StringFormatTag::Fstr => false,
                };

                let string_format = self.parse_fstring_body()?;
                Expression::new(
                    ExpressionKind::StringFormat(StringFormat {
                        to_string,
                        parts: string_format.0,
                        trailing: string_format.1,
                    }),
                    self.span_combine(start_span),
                )
            }
            &DOT => {
                self.bump();
                match self.token().kind {
                    ROUND_OPEN => self.parse_tuple_expression()?,
                    CURLY_OPEN => self.parse_named_tuple_expression()?,
                    _ => {
                        return Err(Fault::error(
                            format!(
                                "`{}` is invalid as start of expression",
                                Symbol::Dot.as_str()
                            ),
                            Some(start_span),
                        ));
                    }
                }
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

    fn parse_tuple_expression(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect(&ROUND_OPEN)?;
        let mut values = vec![];
        loop {
            self.skip_end_lines();
            values.push(self.parse_expression_id(&[COMMA, ROUND_CLOSE])?);
            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }
            self.bump();
        }
        self.expect(&ROUND_CLOSE)?;
        Ok(Expression::new(
            ExpressionKind::Tuple(values),
            self.span_combine(start_span),
        ))
    }

    fn parse_named_tuple_expression(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect(&CURLY_OPEN)?;
        let mut values = vec![];
        loop {
            self.skip_end_lines();
            let ident = self.try_bump_consume_ident()?;
            self.expect(&COLON)?;
            values.push((ident, self.parse_expression_id(&[COMMA, CURLY_CLOSE])?));
            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }
            self.bump();
        }
        self.expect(&CURLY_CLOSE)?;
        Ok(Expression::new(
            ExpressionKind::NamedTuple(values),
            self.span_combine(start_span),
        ))
    }

    fn parse_fstring_body(&mut self) -> SoulResult<(Vec<(String, ExpressionId)>, String)> {
        self.bump(); // consume StringFormat tag

        let mut parts = vec![];
        let mut trailing = String::new();

        loop {
            match &self.token().kind {
                TokenKind::FStringPart(text) => {
                    let text = text.clone();
                    self.bump();

                    match &self.token().kind {
                        TokenKind::Symbol(Symbol::CurlyOpen) => {
                            self.bump();
                            let expr_id = self.parse_expression_id(&[
                                TokenKind::Symbol(Symbol::CurlyClose),
                                TokenKind::EndLine,
                                TokenKind::EndFile,
                            ])?;
                            self.tokens.set_fstr_mode(true);
                            self.expect(&CURLY_CLOSE)?;
                            parts.push((text, expr_id));
                        }
                        _ => {
                            trailing = text;
                        }
                    }
                }
                TokenKind::FStringEnd => {
                    self.bump();
                    break;
                }
                _ => {
                    return Err(Fault::error(
                        "expected format string part or end of format string",
                        Some(self.token().span),
                    ));
                }
            }
        }

        Ok((parts, trailing))
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
        match &self.token().kind {
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
                        if self.current_is(&CURLY_OPEN) {
                            return self
                                .parse_struct_contructor(ident, generics, start_span)
                                .map(Expression::from_struct_contructor);
                        }
                        return Ok(Expression::new_variable(self.alloc_node(), ident));
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

        Ok(Expression::new_variable(self.alloc_node(), ident))
    }

    pub(super) fn parse_keyword_primary(
        &mut self,
        start_span: Span,
        keyword: KeyWord,
    ) -> SoulResult<Option<Expression>> {
        Ok(Some(match keyword {
            KeyWord::If => self.parse_if()?,
            KeyWord::Match => self.parse_match()?,
            KeyWord::For => self.parse_for_loop().map(Expression::from_for)?,

            KeyWord::True | KeyWord::False => {
                let value = keyword == KeyWord::True;
                self.bump();
                Expression::new_literal(self.alloc_node(), Literal::Bool(value), self.token().span)
            }

            KeyWord::Null => {
                self.bump();
                Expression::new(ExpressionKind::Null(self.alloc_node()), self.token().span)
            }

            KeyWord::Undefined => {
                self.bump();
                Expression::new(
                    ExpressionKind::Undefined(self.alloc_node()),
                    self.token().span,
                )
            }

            KeyWord::Break | KeyWord::Return | KeyWord::Continue => {
                return Err(Fault::error(
                    format!("can not have {} in expression", keyword.as_str()),
                    Some(self.token().span),
                ));
            }

            KeyWord::New => {
                self.bump();
                match &self.token().kind {
                    &ROUND_OPEN => self.parse_new_ptr(start_span)?,
                    &SQUARE_OPEN | &ARRAY => self.parse_new_array(start_span)?,
                    _ => {
                        return Err(Fault::error(
                            "expected '(' or ':[' after 'new'".to_string(),
                            Some(self.token().span),
                        ));
                    }
                }
            }

            KeyWord::Intrinsic | KeyWord::Task | KeyWord::Spawn => {
                let is_block = keyword == KeyWord::Task || keyword == KeyWord::Spawn;
                let name = keyword.as_str();
                self.bump();
                if is_block && let Ok(primary) = self.try_parse_keyword_block(start_span) {
                    return Ok(Some(primary));
                }
                Expression::new_variable(
                    self.alloc_node(),
                    Ident::new(name.to_string(), start_span),
                )
            }

            _ => return Ok(None),
        }))
    }

    fn try_parse_keyword_block(&mut self, start_span: Span) -> SoulResult<Expression> {
        if !self.current_is(&CURLY_OPEN) {
            return Err(Fault::error(
                "expected block after keyword".to_string(),
                Some(self.token().span),
            ));
        }
        let block = self.parse_block(TypeModifier::Mut)?;
        Ok(Expression::new_block(block, self.span_combine(start_span)))
    }

    fn parse_primary_keyword(&mut self, start_span: Span) -> SoulResult<Option<Expression>> {
        let ident = self.try_token_as_ident_str()?;
        match KeyWord::from_str(ident) {
            Ok(keyword) => self.parse_keyword_primary(start_span, keyword),
            _ => Ok(None),
        }
    }
}
