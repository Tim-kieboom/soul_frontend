use parser_models::ast::{
    BinaryOperator, BinaryOperatorKind, Expression, ExpressionHelpers, ExpressionKind, Literal,
    UnaryOperator, UnaryOperatorKind,
};
use soul_tokenizer::{Number, Token, TokenKind};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{AccessType, KeyWord, Operator, TypeModifier},
    span::Span,
    symbool_kind::SymbolKind,
    try_result::ToResult,
};

use crate::parser::{
    Parser,
    parse_utils::{
        ARROW_LEFT, COLON, CURLY_OPEN, DECREMENT, INCREMENT, ROUND_OPEN, SQUARE_CLOSE, SQUARE_OPEN,
    },
};

mod parse_condition;
mod parse_expression_group;

impl<'a> Parser<'a> {
    pub(crate) fn parse_expression(&mut self, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        self.pratt_parse_expression(0, end_tokens)
    }

    fn pratt_parse_expression(
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
                    Some(self.span_combine(start_span)),
                ));
            }

            let precedence = self.current_precedence();

            // If precedence is lower than the minimum required, stop parsing more operators here
            if precedence < min_precedence {
                break;
            }

            match self.consume_expression_operator(start_span)? {
                ExpressionOperator::Binary(operator) => {
                    let next_min_precedence = precedence + 1;
                    let right = self.pratt_parse_expression(next_min_precedence, end_tokens)?;
                    left =
                        Expression::new_binary(left, operator, right, self.span_combine(start_span))
                }
                ExpressionOperator::Access(AccessType::AccessThis) => {
                    todo!("impl access this")
                }
                ExpressionOperator::Access(AccessType::AccessIndex) => {
                    let index = self.parse_expression(&[
                        SQUARE_CLOSE,
                        TokenKind::EndLine,
                        TokenKind::EndFile,
                    ])?;
                    self.expect(&SQUARE_CLOSE)?;
                    left = Expression::new_index(left, index, self.span_combine(start_span))
                }
            }
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;

        let expression = match &self.token().kind {
            &CURLY_OPEN => {
                let block = self.parse_block(TypeModifier::Mut)?;
                Expression::new_block(block, self.span_combine(start_span))
            }
            &SQUARE_OPEN => {
                let array = self.parse_array(None)?;
                Expression::from_array(array)
            }
            &ROUND_OPEN => {
                return Err(SoulError::new(
                    "tuple not yet impl",
                    SoulErrorKind::InternalError,
                    Some(start_span),
                ));
            }
            TokenKind::Symbol(symbol) => {
                let unary = self.expect_unary_kind(start_span, *symbol)?;
                self.bump();

                let rvalue = self.parse_primary()?;
                let span = self.span_combine(start_span);
                match unary {
                    UnaryKinds::UnaryOperator(unary) => Expression::new_unary(unary, rvalue, span),
                    UnaryKinds::Ref { mutable } => Expression::new_ref(mutable, rvalue, span),
                    UnaryKinds::Deref => Expression::new_deref(rvalue, span),
                }
            }
            TokenKind::Ident(_) => self.parse_primary_ident(start_span)?,
            TokenKind::CharLiteral(char) => {
                let char = *char;
                self.bump();
                Expression::new_literal(Literal::Char(char), start_span)
            }
            TokenKind::StringLiteral(_) => {
                let token = self.bump_consume();
                let string = match token.kind {
                    TokenKind::StringLiteral(val) => val,
                    _ => unreachable!(),
                };
                Expression::new_literal(Literal::Str(string), token.span)
            }
            TokenKind::Number(num) => {
                let number = match num {
                    Number::Int(val) => Literal::Int(*val),
                    Number::Uint(val) => Literal::Uint(*val),
                    Number::Float(val) => Literal::Float(*val),
                };
                self.bump();
                Expression::new_literal(number, start_span)
            }
            other => {
                return Err(SoulError::new(
                    format!("'{}' is invalid as start of expression", other.display(),),
                    SoulErrorKind::InvalidTokenKind,
                    Some(start_span),
                ));
            }
        };

        if self.current_is_any(&[INCREMENT, DECREMENT]) {
            let operator = match self.token().kind {
                INCREMENT => UnaryOperator::new(
                    UnaryOperatorKind::Increment { before_var: false },
                    self.span_combine(start_span),
                ),
                DECREMENT => UnaryOperator::new(
                    UnaryOperatorKind::Decrement { before_var: false },
                    self.span_combine(start_span),
                ),
                _ => unreachable!(),
            };

            self.bump();
            return Ok(Expression::new_unary(
                operator,
                expression,
                self.span_combine(start_span),
            ));
        }

        Ok(expression)
    }

    fn parse_primary_ident(&mut self, start_span: Span) -> SoulResult<Expression> {
        let str = self.try_token_as_ident_str()?;
        if str == "true" || str == "false" {
            let value = str == "true";
            self.bump();
            return Ok(Expression::new_literal(
                Literal::Bool(value),
                self.token().span,
            ));
        }

        match KeyWord::from_str(str) {
            Some(KeyWord::If) => return self.parse_if(),
            Some(KeyWord::While) => return self.parse_while(),

            Some(KeyWord::Fall)
            | Some(KeyWord::Break)
            | Some(KeyWord::Return)
            | Some(KeyWord::Continue) => {
                return Err(SoulError::new(
                    format!("can not have {} in expression", str),
                    SoulErrorKind::InvalidContext,
                    Some(self.token().span),
                ));
            }

            _ => (),
        };

        let ident = self.try_bump_consume_ident()?;

        let peek = self.peek();
        Ok(match &self.token().kind {
            &COLON if peek.kind == SQUARE_OPEN => {
                return Err(SoulError::new(
                    "collectionType array not yet impl",
                    SoulErrorKind::InternalError,
                    Some(ident.get_span()),
                ));
            }
            &ROUND_OPEN | &ARROW_LEFT => {
                let function_call = self
                    .try_parse_function_call(start_span, None, &ident)
                    .merge_to_result()?;

                Expression::from_function_call(function_call)
            }

            _ => {
                let span = ident.get_span();
                Expression::new(
                    ExpressionKind::Variable {
                        ident,
                        resolved: None,
                        id: None,
                    },
                    span,
                )
            }
        })
    }

    fn expect_unary_kind(&self, start_span: Span, symbool: SymbolKind) -> SoulResult<UnaryKinds> {
        let op = match Operator::from_symbool(symbool) {
            Some(val) => val,
            None => return Err(SoulError::new(
                format!("'{}' is not a valid operator", symbool.as_str()),
                SoulErrorKind::InvalidOperator,
                Some(self.span_combine(start_span)),
            ))
        };

        match op.to_unary() {
            Some(unary) => {
                return Ok(UnaryKinds::UnaryOperator(
                    UnaryOperator::new(unary, self.span_combine(start_span))
                ))
            }
            None => (),
        }

        match op {
            Operator::Mul => Ok(UnaryKinds::Deref),
            Operator::BitAnd => Ok(UnaryKinds::Ref { mutable: true }),
            Operator::ConstRef => Ok(UnaryKinds::Ref { mutable: false }),
            _ => Err(SoulError::new(
                format!("'{}' is not a valid unary operator", op.as_str()),
                SoulErrorKind::InvalidOperator,
                Some(self.span_combine(start_span)),
            ))
        }
    }

    fn current_precedence(&self) -> usize {
        match &self.token().kind {
            TokenKind::Ident(ident) => {
                if let Some(keyword) = KeyWord::from_str(ident) {
                    keyword.precedence()
                } else {
                    0
                }
            }
            TokenKind::Symbol(symbool_kind) => {
                if let Some(access) = AccessType::from_symbool(*symbool_kind) {
                    access.precedence()
                } else if let Some(op) = Operator::from_symbool(*symbool_kind) {
                    op.precedence()
                } else {
                    0
                }
            }
            _ => 0,
        }
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
            TokenKind::Symbol(sym) => {
                if let Some(access) = AccessType::from_symbool(*sym) {
                    self.bump();
                    return Ok(ExpressionOperator::Access(access));
                } else if let Some(Some(binary)) =
                    Operator::from_symbool(*sym).map(|el| el.to_binary())
                {
                    self.bump();
                    return Ok(ExpressionOperator::Binary(BinaryOperator::new(
                        binary,
                        self.span_combine(start_span),
                    )));
                }

                get_invalid_error(self.token())
            }

            _ => get_invalid_error(self.token()),
        }
    }
}

enum UnaryKinds {
    UnaryOperator(UnaryOperator),
    Ref{mutable: bool},
    Deref,
}

enum ExpressionOperator {
    Binary(BinaryOperator),
    Access(AccessType),
}

pub trait ConvertOperator {
    fn to_unary(&self) -> Option<UnaryOperatorKind>;
    fn to_binary(&self) -> Option<BinaryOperatorKind>;
}
impl ConvertOperator for Operator {
    fn to_unary(&self) -> Option<UnaryOperatorKind> {
        Some(match self {
            Operator::Not => UnaryOperatorKind::Not,
            Operator::Sub => UnaryOperatorKind::Neg,
            Operator::Incr => UnaryOperatorKind::Increment { before_var: true },
            Operator::Decr => UnaryOperatorKind::Decrement { before_var: true },
            _ => return None,
        })
    }

    fn to_binary(&self) -> Option<BinaryOperatorKind> {
        Some(match self {
            Operator::Eq => BinaryOperatorKind::Eq,
            Operator::Mul => BinaryOperatorKind::Mul,
            Operator::Div => BinaryOperatorKind::Div,
            Operator::Mod => BinaryOperatorKind::Mod,
            Operator::Add => BinaryOperatorKind::Add,
            Operator::Sub => BinaryOperatorKind::Sub,
            Operator::Root => BinaryOperatorKind::Root,
            Operator::Power => BinaryOperatorKind::Pow,
            Operator::LessEq => BinaryOperatorKind::Le,
            Operator::GreatEq => BinaryOperatorKind::Ge,
            Operator::LessThen => BinaryOperatorKind::Lt,
            Operator::NotEq => BinaryOperatorKind::NotEq,
            Operator::Range => BinaryOperatorKind::Range,
            Operator::BitOr => BinaryOperatorKind::BitOr,
            Operator::LogOr => BinaryOperatorKind::LogOr,
            Operator::GreatThen => BinaryOperatorKind::Gt,
            Operator::BitAnd => BinaryOperatorKind::BitAnd,
            Operator::BitXor => BinaryOperatorKind::BitXor,
            Operator::LogAnd => BinaryOperatorKind::LogAnd,

            Operator::Not | Operator::Incr | Operator::Decr | Operator::ConstRef => return None,
        })
    }
}
