use crate::{
    steps::{
        parse::{
            ARROW_LEFT, COLON, CURLY_OPEN, DECREMENT, INCREMENT, ROUND_OPEN, SQUARE_CLOSE,
            SQUARE_OPEN, STAMENT_END_TOKENS, parser::Parser,
        },
        tokenize::token_stream::{Number, Token, TokenKind},
    },
    utils::try_result::{ToResult, TryError},
};
use models::{
    abstract_syntax_tree::{
        expression::{Expression, ExpressionKind, FieldAccess, Index, ReturnKind, ReturnLike},
        expression_groups::ExpressionGroup,
        function::StructConstructor,
        literal::Literal,
        operator::{Binary, BinaryOperator, Unary, UnaryOperator, UnaryOperatorKind},
        soul_type::{SoulType, TypeKind},
        statment::Ident,
    },
    error::{SoulError, SoulErrorKind, SoulResult, Span},
    soul_names::{self, AccessType, KeyWord, Operator, TypeModifier},
    symbool_kind::SymboolKind,
};

impl<'a> Parser<'a> {
    pub(crate) fn parse_expression(&mut self, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        let expression = self.pratt_parse_precedence(0, end_tokens)?;
        Ok(expression)
    }

    pub(crate) fn parse_return_like(&mut self, kind: ReturnKind) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(kind.as_keyword().as_str())?;

        let value = if self.current_is_any(STAMENT_END_TOKENS) {
            None
        } else {
            Some(Box::new(self.parse_expression(STAMENT_END_TOKENS)?))
        };

        self.expect_any(STAMENT_END_TOKENS)?;
        Ok(Expression::new(
            ExpressionKind::ReturnLike(ReturnLike { value, kind }),
            self.new_span(start_span),
        ))
    }

    fn pratt_parse_precedence(
        &mut self,
        min_precedence: usize,
        end_tokens: &[TokenKind],
    ) -> SoulResult<Expression> {
        let start_span = self.token().span;
        let mut left = self.parse_primary()?;

        loop {
            if self.current_is_any(end_tokens) {
                break;
            }

            self.skip_end_lines();
            if self.current_is_any(end_tokens) {
                break;
            }

            if self.current_is(&TokenKind::EndFile) {
                return Err(SoulError::new(
                    "unexpected end of file while parsing expression".to_string(),
                    SoulErrorKind::UnexpecedFileEnd,
                    Some(self.new_span(start_span)),
                ));
            }

            let precedence = self.current_precedence();

            // If precedence is lower than the minimum required, stop parsing more operators here
            if precedence < min_precedence {
                break;
            }

            let operator = match self.consume_expression_operator(start_span)? {
                ExpressionOperator::Binary(val) => val,
                ExpressionOperator::Access(AccessType::AccessIndex) => {
                    left = self.parse_index(start_span, left)?;
                    continue;
                }
                ExpressionOperator::Access(AccessType::AccessThis) => {
                    left = self.parse_access(start_span, left)?;
                    continue;
                }
            };

            let next_min_precedence = precedence + 1;
            let right = self.pratt_parse_precedence(next_min_precedence, end_tokens)?;

            left = self.new_binary(start_span, left, operator, right);
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;

        let expression = match &self.token().kind {
            &CURLY_OPEN => match self.try_parse_named_tuple() {
                Ok(named_tuple) => Expression::new(
                    ExpressionKind::ExpressionGroup(ExpressionGroup::NamedTuple(named_tuple)),
                    self.new_span(start_span),
                ),
                Err(TryError::IsErr(err)) => return Err(err),
                Err(TryError::IsNotValue(_)) => {
                    let block = self.parse_block(TypeModifier::Mut)?;
                    Expression::new(ExpressionKind::Block(block), self.new_span(start_span))
                }
            },
            &SQUARE_OPEN => self.parse_array(None).map(|el| {
                Expression::new(
                    ExpressionKind::ExpressionGroup(ExpressionGroup::Array(el)),
                    self.new_span(start_span),
                )
            })?,
            &ROUND_OPEN => {
                let tuple = self.parse_tuple()?;
                let kind = if tuple.values.is_empty() {
                    ExpressionKind::Default
                } else {
                    ExpressionKind::ExpressionGroup(ExpressionGroup::Tuple(tuple))
                };

                Expression::new(kind, self.new_span(start_span))
            }
            TokenKind::Symbool(symbool) => {
                let unary = self.expect_unary(start_span, *symbool)?;
                self.bump();

                let right = self.parse_primary()?;
                self.new_unary(start_span, unary, right)
            }
            TokenKind::Ident(_) => self.parse_primary_ident(start_span)?,
            TokenKind::CharLiteral(char) => {
                let char = *char;
                self.bump();
                Expression::new_literal(Literal::Char(char), self.new_span(start_span))
            }
            TokenKind::StringLiteral(_) => {
                let token = self.bump_consume();
                let ident = match token.kind {
                    TokenKind::StringLiteral(val) => val,
                    _ => unreachable!(),
                };
                Expression::new_literal(Literal::Str(ident), self.new_span(start_span))
            }
            TokenKind::Number(Number::Int(num)) => {
                let number = *num;
                self.bump();
                Expression::new_literal(Literal::Int(number), self.new_span(start_span))
            }
            TokenKind::Number(Number::Uint(num)) => {
                let number = *num;
                self.bump();
                Expression::new_literal(Literal::Uint(number), self.new_span(start_span))
            }
            TokenKind::Number(Number::Float(num)) => {
                let number = *num;
                self.bump();
                Expression::new_literal(Literal::Float(number), self.new_span(start_span))
            }
            other => {
                return Err(SoulError::new(
                    format!("'{}' is invalid as start of expression", other.display()),
                    SoulErrorKind::InvalidAssignType,
                    Some(self.new_span(start_span)),
                ));
            }
        };

        if self.current_is_any(&[INCREMENT, DECREMENT]) {
            let operator = if self.current_is(&INCREMENT) {
                UnaryOperator::new(
                    UnaryOperatorKind::Increment { before_var: false },
                    self.new_span(start_span),
                )
            } else {
                UnaryOperator::new(
                    UnaryOperatorKind::Decrement { before_var: false },
                    self.new_span(start_span),
                )
            };

            self.bump();
            return Ok(self.new_unary(start_span, operator, expression));
        }

        Ok(expression)
    }

    fn parse_primary_ident(&mut self, start_span: Span) -> SoulResult<Expression> {
        let text = match &self.token().kind {
            TokenKind::Ident(val) => val,
            _ => {
                return Err(SoulError::new(
                    "expected ident",
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.token().span),
                ));
            }
        };

        if text == "true" || text == "false" {
            let value = text == "true";
            self.bump();
            return Ok(Expression::new(
                ExpressionKind::Literal(Literal::Bool(value)),
                self.new_span(start_span),
            ));
        }

        Ok(match KeyWord::from_str(text) {
            Some(KeyWord::If) => self.parse_if()?,
            Some(KeyWord::For) => self.parse_for()?,
            Some(KeyWord::While) => self.parse_while()?,
            Some(KeyWord::Match) => self.parse_match()?,
            Some(KeyWord::Break) => self.parse_return_like(ReturnKind::Break)?,
            Some(KeyWord::Return) => self.parse_return_like(ReturnKind::Return)?,
            Some(KeyWord::Continue) => self.parse_return_like(ReturnKind::Continue)?,
            _ => {
                let ident_token = self.bump_consume();
                let text = match ident_token.kind {
                    TokenKind::Ident(val) => val,
                    _ => unreachable!(),
                };
                let ident = Ident::new(text, ident_token.span);

                if self.current_is(&COLON) && self.peek().kind == SQUARE_OPEN {
                    self.bump();
                    let collection_type = SoulType::new(
                        None,
                        TypeKind::Stub {
                            ident,
                            resolved: None,
                        },
                        self.token().span,
                    );
                    let array = self.parse_array(Some(collection_type))?;
                    Expression::new(
                        ExpressionKind::ExpressionGroup(ExpressionGroup::Array(array)),
                        self.new_span(start_span),
                    )
                } else if self.current_is(&ROUND_OPEN) || self.current_is(&ARROW_LEFT) {
                    let function_call = self
                        .try_parse_function_call(start_span, None, &ident)
                        .merge_to_result()?;

                    Expression::with_atribute(
                        ExpressionKind::FunctionCall(function_call.node),
                        self.new_span(start_span),
                        function_call.attributes,
                    )
                } else {
                    if self.current_is(&CURLY_OPEN) {
                        let ty = SoulType::new_stub(ident, self.token().span);
                        return self.parse_struct_constructor(ty).map(|ctor| {
                            Expression::new(
                                ExpressionKind::StructConstructor(ctor),
                                self.new_span(ident_token.span),
                            )
                        });
                    }

                    Expression::new(
                        ExpressionKind::Variable {
                            ident,
                            resolved: None,
                        },
                        self.new_span(start_span),
                    )
                }
            }
        })
    }

    fn parse_struct_constructor(&mut self, ty: SoulType) -> SoulResult<StructConstructor> {
        self.try_parse_named_tuple()
            .merge_to_result()
            .map(|el| StructConstructor {
                calle: ty,
                arguments: el,
            })
    }

    fn parse_index(&mut self, start_span: Span, collection: Expression) -> SoulResult<Expression> {
        let index = self.parse_expression(&[SQUARE_CLOSE])?;
        self.expect(&SQUARE_CLOSE)?;

        Ok(Expression::new(
            ExpressionKind::Index(Index {
                collection: Box::new(collection),
                index: Box::new(index),
            }),
            self.new_span(start_span),
        ))
    }

    fn parse_access(&mut self, start_span: Span, lvalue: Expression) -> SoulResult<Expression> {
        let token = self.bump_consume();
        let text = match token.kind {
            TokenKind::Ident(val) => val,
            other => {
                return Err(SoulError::new(
                    format!("expected ident got '{}'", other.display()),
                    SoulErrorKind::InvalidTokenKind,
                    Some(token.span),
                ));
            }
        };
        let ident = Ident::new(text, token.span);

        match self.try_parse_function_call(start_span, Some(&lvalue), &ident) {
            Ok(methode) => {
                return Ok(Expression::with_atribute(
                    ExpressionKind::FunctionCall(methode.node),
                    methode.span,
                    methode.attributes,
                ));
            }
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(_)) => (),
        }

        Ok(Expression::new(
            ExpressionKind::FieldAccess(FieldAccess {
                object: Box::new(lvalue),
                field: ident,
            }),
            self.token().span,
        ))
    }

    fn expect_unary(&self, start_span: Span, symbool: SymboolKind) -> SoulResult<UnaryOperator> {
        match Operator::from_symbool(symbool) {
            Some(op) => {
                if let Some(unary) = op.to_unary() {
                    Ok(UnaryOperator::new(unary, self.new_span(start_span)))
                } else {
                    Err(SoulError::new(
                        format!("'{}' is not a valid unary operator", op.as_str()),
                        SoulErrorKind::InvalidOperator,
                        Some(self.new_span(start_span)),
                    ))
                }
            }
            None => Err(SoulError::new(
                format!("'{}' is not a valid operator", symbool.as_str()),
                SoulErrorKind::InvalidOperator,
                Some(self.new_span(start_span)),
            )),
        }
    }

    fn new_binary(
        &self,
        start_span: Span,
        left: Expression,
        operator: BinaryOperator,
        right: Expression,
    ) -> Expression {
        Expression::new(
            ExpressionKind::Binary(Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            }),
            self.new_span(start_span),
        )
    }

    fn new_unary(
        &self,
        start_span: Span,
        operator: UnaryOperator,
        expression: Expression,
    ) -> Expression {
        Expression::new(
            ExpressionKind::Unary(Unary {
                operator,
                expression: Box::new(expression),
            }),
            self.new_span(start_span),
        )
    }

    fn consume_expression_operator(&mut self, start_span: Span) -> SoulResult<ExpressionOperator> {
        
        fn get_invalid_error(token: &Token) -> SoulResult<ExpressionOperator> {
            Err(SoulError::new(
                format!("'{}' is not a valid operator", token.kind.display()),
                SoulErrorKind::InvalidOperator,
                Some(token.span),
            ))
        }

        match &self.token().kind {
            TokenKind::Symbool(sym) => {
                if let Some(access) = AccessType::from_symbool(*sym) {
                    self.bump();
                    return Ok(ExpressionOperator::Access(access));
                } else if let Some(Some(binary)) =
                    Operator::from_symbool(*sym).map(|el| el.to_binary())
                {
                    self.bump();
                    return Ok(ExpressionOperator::Binary(BinaryOperator::new(
                        binary,
                        self.new_span(start_span),
                    )));
                }

                get_invalid_error(self.token())
            }

            _ => get_invalid_error(self.token()),
        }
    }

    fn current_precedence(&self) -> usize {
        match &self.token().kind {
            TokenKind::Ident(ident) => {
                if let Some(keyword) = soul_names::KeyWord::from_str(ident) {
                    keyword.precedence()
                } else {
                    0
                }
            }
            TokenKind::Symbool(symbool_kind) => {
                if let Some(access) = soul_names::AccessType::from_symbool(*symbool_kind) {
                    access.precedence()
                } else if let Some(op) = soul_names::Operator::from_symbool(*symbool_kind) {
                    op.precedence()
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

enum ExpressionOperator {
    Binary(BinaryOperator),
    Access(AccessType),
}
