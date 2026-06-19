use ast_model::{
    expression::{Expression, ExpressionId, ExpressionKind, TypeOf, TypeofKind},
    operators::{BinaryOperator, BinaryOperatorKind, UnaryOperator, UnaryOperatorKind},
};
use soul_tokenizer::model::{Token, TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident, define_symbols,
    error::SoulResult,
    fault::Fault,
    soul_names::{Operator, Symbol},
    span::Span,
};

use crate::{
    parse::expression::precedence::Precedence,
    parser::Parser,
    utils::{ARRAY, DOT, NOT, NULL, ROUND_CLOSE, ROUND_OPEN, SQUARE_OPEN},
};

mod access;
mod conditionals;
mod group_expressions;
mod precedence;
mod primairy;

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_expression_id(
        &mut self,
        end_tokens: &[TokenKind],
    ) -> SoulResult<ExpressionId> {
        let value = self.pratt_parse_expression(Precedence::MIN, end_tokens, None)?;
        Ok(self.store.insert_expression(value))
    }

    pub(crate) fn parse_expression(&mut self, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        self.pratt_parse_expression(Precedence::MIN, end_tokens, None)
    }

    pub(crate) fn parse_primary_expression(
        &mut self,
        primary: Expression,
        end_tokens: &[TokenKind],
    ) -> SoulResult<Expression> {
        self.pratt_parse_expression(Precedence::MIN, end_tokens, Some(primary))
    }

    fn pratt_parse_expression(
        &mut self,
        min_precedence: Precedence,
        end_tokens: &[TokenKind],
        primary: Option<Expression>,
    ) -> SoulResult<Expression> {
        let start_span = self.current().span;

        let mut unary_operators = vec![];
        self.collect_unary_operators(&mut unary_operators, start_span);

        let mut left = match primary {
            Some(value) => value,
            None => self.parse_primary(end_tokens)?,
        };

        loop {
            match self.check_for_end_tokens(end_tokens) {
                Loop::None => (),
                Loop::Break => break,
                Loop::Continue => continue,
            }

            if self.current_is(&TokenKind::EndFile) {
                return Err(Fault::error(
                    "unexpected end of file while parsing expression".to_string(),
                    Some(self.span_combine(start_span)),
                ));
            }

            let token_kind = &self.current().kind;
            match token_kind {
                TokenKind::Symbol(Symbol::Dot) | TokenKind::Symbol(Symbol::SquareOpen) => (),
                _ => break,
            };

            match self.consume_expression_operator(start_span)? {
                ExpressionOperator::Access(AccessType::AccessThis) => {
                    self.access_this_expression(&mut left, start_span)?;
                    continue;
                }
                ExpressionOperator::Access(AccessType::AccessIndex) => {
                    self.access_index_expression(&mut left, start_span)?;
                    continue;
                }
                _ => break,
            }
        }

        left = self.apply_prefix_operators(left, unary_operators);

        loop {
            match self.check_for_end_tokens(end_tokens) {
                Loop::None => (),
                Loop::Break => break,
                Loop::Continue => continue,
            }

            if self.current_is(&TokenKind::EndFile) {
                return Err(Fault::error(
                    "unexpected end of file while parsing expression".to_string(),
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
                    let next_min_precedence = precedence.next();
                    let right =
                        self.pratt_parse_expression(next_min_precedence, end_tokens, None)?;
                    let span = self.span_combine(start_span);
                    left = Expression::new_binary(
                        self.store.insert_expression(left),
                        operator,
                        self.store.insert_expression(right),
                        span,
                    );
                }
                ExpressionOperator::TypeOf {
                    kind,
                    binding,
                } => {

                    let typeof_ = TypeOf {
                        kind,
                        binding,
                        binding_id: None,
                        value: self.store.insert_expression(left),
                    };
                    left = Expression::new(
                        ExpressionKind::TypeOf(typeof_),
                        self.span_combine(start_span),
                    )
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn check_for_end_tokens(&mut self, end_tokens: &[TokenKind]) -> Loop {
        if self.current_is(&TokenKind::EndLine) && self.current_is_any(end_tokens) {
            let saved = self.tokens.current_position();
            self.skip_end_lines();
            if self.current_is(&DOT) {
                return Loop::Continue;
            }
            self.goto(saved);
        }

        if self.current_is_any(end_tokens) {
            return Loop::Break;
        }

        self.skip_end_lines();
        if self.current_is_any(end_tokens) {
            return Loop::Break;
        }

        return Loop::None;
    }

    fn current_precedence(&mut self) -> Precedence {
        match &self.current().kind {
            TokenKind::Ident(ident) => {
                if let Some(keyword) = KeyWord::from_str(ident) {
                    Precedence::new(keyword.precedence())
                } else {
                    Precedence::MIN
                }
            }
            TokenKind::Keyword(keyword) => Precedence::new(keyword.precedence()),
            TokenKind::Symbol(symbool_kind) => {
                if let Some(bin) = try_to_binary_operator(symbool_kind) {
                    let peek = self.peek();
                    Precedence::new(self.try_multi_binary(&peek, bin).precedence())
                } else if let Some(access) = AccessType::from_symbool(*symbool_kind) {
                    Precedence::new(access.precedence())
                } else if let Some(unary) = try_to_unary_operator(symbool_kind) {
                    Precedence::new(unary.precedence())
                } else {
                    Precedence::MIN
                }
            }
            _ => Precedence::MIN,
        }
    }

    fn apply_prefix_operators(
        &mut self,
        mut left: Expression,
        prefix_operators: Vec<(Span, UnaryKinds)>,
    ) -> Expression {
        for (span, unary) in prefix_operators.into_iter().rev() {
            let id = self.store.insert_expression(left);
            left = match unary {
                UnaryKinds::UnaryOperator(unary) => Expression::new_unary(unary, id, span),
                UnaryKinds::Ref { mutable } => Expression::new_ref(mutable, id, span),
                UnaryKinds::Deref => Expression::new_deref(id, span),
            };
        }

        left
    }

    fn parse_sizeof(&mut self, left_id: ExpressionId) -> SoulResult<Expression> {
        let span = self.store.expressions[left_id].span;
        Ok(Expression::new(
            ExpressionKind::Sizeof(left_id),
            self.span_combine(span),
        ))
    }

    fn consume_expression_operator(&mut self, start_span: Span) -> SoulResult<ExpressionOperator> {
        fn get_invalid_error(token: &Token) -> SoulResult<ExpressionOperator> {
            Err(Fault::error(
                format!("`{}` is not a valid operator", token.kind.display()),
                Some(token.span),
            ))
        }

        match &self.current().kind {
            TokenKind::Ident(ident) => match KeyWord::from_str(ident.as_str()) {
                Some(KeyWord::Typeof) => {
                    return self.parse_typeof_operator(start_span);
                }
                _ => get_invalid_error(self.current()),
            },
            TokenKind::Keyword(KeyWord::Typeof) => {
                return self.parse_typeof_operator(start_span);
            }
            TokenKind::Symbol(sym) => {
                if let Some(access) = AccessType::from_symbool(*sym) {
                    self.bump();
                    return Ok(ExpressionOperator::Access(access));
                } else if let Some(mut binary) = try_to_binary_operator(sym) {
                    self.bump();
                    binary = self.try_consume_multi_binary(binary);

                    return Ok(ExpressionOperator::Binary(BinaryOperator::new(
                        binary,
                        self.span_combine(start_span),
                    )));
                }

                get_invalid_error(self.current())
            }
            _ => get_invalid_error(self.current()),
        }
    }

    fn try_consume_multi_binary(&mut self, binary: BinaryOperatorKind) -> BinaryOperatorKind {
        let bin = self.try_multi_binary(self.current(), binary);
        match bin {
            BinaryOperatorKind::Pow | BinaryOperatorKind::LogAnd => self.bump(),
            _ => (),
        };
        bin
    }

    fn try_multi_binary(&self, current: &Token, binary: BinaryOperatorKind) -> BinaryOperatorKind {
        let symbol = match &current.kind {
            TokenKind::Symbol(val) => *val,
            _ => return binary,
        };

        match binary {
            BinaryOperatorKind::Mul => {
                if let Some(Operator::Mul) = Operator::from_symbool(symbol) {
                    BinaryOperatorKind::Pow
                } else {
                    binary
                }
            }
            BinaryOperatorKind::BitAnd => {
                if let Some(Operator::BitAnd) = Operator::from_symbool(symbol) {
                    BinaryOperatorKind::LogAnd
                } else {
                    binary
                }
            }
            _ => binary,
        }
    }

    fn parse_typeof_operator(&mut self, _start_span: Span) -> SoulResult<ExpressionOperator> {
        self.expect(&TokenKind::Keyword(KeyWord::Typeof))?;

        let kind = match &self.current().kind {
            TokenKind::Ident(_) => {
                let type_name = self.try_bump_consume_ident()?;
                self.expect(&TokenKind::Symbol(Symbol::Dot))?;
                let variant_name = self.try_bump_consume_ident()?;
                TypeofKind::Union { type_name, variant_name }
            }
            &NULL => {
                self.bump();
                TypeofKind::Null
            }
            &NOT if self.peek_is(&NULL) => {
                self.bump();
                self.bump();
                TypeofKind::NotNull
            }
            _ => return Err(Fault::error(
                format!(
                    "expected ident or `null` or `!null` but got {}", 
                    self.current().kind.display(),
                ),
                Some(self.current().span)
            )),
        };
        

        let binding = if self.current_is(&TokenKind::Symbol(Symbol::RoundOpen)) {
            self.bump();
            let name = self.try_bump_consume_ident()?;
            self.expect(&TokenKind::Symbol(Symbol::RoundClose))?;
            Some(name)
        } else {
            None
        };

        if matches!(kind, TypeofKind::Null) && binding.is_some() {
            
            let span = binding.map(|b| b.span())
                .unwrap_or(self.current().span);
            
            return Err(Fault::error(
                format!("`{}` can not have binding", KeyWord::Null.as_str()),
                Some(span)
            ))
        }

        Ok(ExpressionOperator::TypeOf {
            kind,
            binding,
        })
    }

    fn parse_new_ptr(&mut self, start_span: Span) -> SoulResult<Expression> {
        self.expect(&ROUND_OPEN)?;
        let inner =
            self.parse_expression_id(&[ROUND_CLOSE, TokenKind::EndLine, TokenKind::EndFile])?;
        self.expect(&ROUND_CLOSE)?;
        Ok(Expression::new(
            ExpressionKind::New(inner),
            self.span_combine(start_span),
        ))
    }

    fn parse_new_array(&mut self, start_span: Span) -> SoulResult<Expression> {
        const START: &[TokenKind] = &[SQUARE_OPEN, ARRAY];

        if !self.current_is_any(START) {
            return Err(self.get_expect_any_error(START));
        }

        let array = self.parse_array(None)?;
        Ok(Expression::new(
            ExpressionKind::NewArray(array.value),
            self.span_combine(start_span),
        ))
    }

    /// Collect prefix operators before parse_primary so that postfix operators
    /// (`.`, `()`, `[]`) bind tighter — which is standard language semantics.
    /// `@*expr` → outer `@` wraps the result of inner `*`; we apply them in
    /// reverse order below so the outermost prefix wraps the innermost.
    fn collect_unary_operators(&mut self, unarys: &mut Vec<(Span, UnaryKinds)>, start_span: Span) {
        while let TokenKind::Symbol(symbol) = &self.current().kind {
            match self.expect_unary_kind(start_span, *symbol) {
                Ok(unary_kind) => {
                    self.bump();
                    unarys.push((self.span_combine(start_span), unary_kind));
                }
                Err(_) => break,
            }
        }
    }

    fn expect_unary_kind(&mut self, start_span: Span, symbool: Symbol) -> SoulResult<UnaryKinds> {
        let op = match Operator::from_symbool(symbool) {
            Some(val) => val,
            None => {
                return Err(Fault::error(
                    format!("`{}` is not a valid operator", symbool.as_str()),
                    Some(self.span_combine(start_span)),
                ));
            }
        };

        if let Some(unary) = op.to_unary() {
            return Ok(UnaryKinds::UnaryOperator(UnaryOperator::new(
                unary,
                self.span_combine(start_span),
            )));
        }

        match op {
            Operator::Mul => Ok(UnaryKinds::Deref),
            Operator::BitAnd => {
                let mutable = matches!(self.peek().kind, TokenKind::Keyword(KeyWord::Mut));
                if mutable {
                    self.bump();
                }
                Ok(UnaryKinds::Ref { mutable })
            }
            _ => Err(Fault::error(
                format!("`{}` is not a valid unary operator", op.as_str()),
                Some(self.span_combine(start_span)),
            )),
        }
    }
}

fn try_to_binary_operator(symbol: &Symbol) -> Option<BinaryOperatorKind> {
    match Operator::from_symbool(*symbol).map(|el| el.to_binary()) {
        Some(Some(val)) => Some(val),
        _ => None,
    }
}

fn try_to_unary_operator(symbol: &Symbol) -> Option<UnaryOperatorKind> {
    match Operator::from_symbool(*symbol).map(|el| el.to_unary()) {
        Some(Some(val)) => Some(val),
        _ => None,
    }
}

enum UnaryKinds {
    UnaryOperator(UnaryOperator),
    Ref { mutable: bool },
    Deref,
}

enum ExpressionOperator {
    Binary(BinaryOperator),
    Access(AccessType),
    TypeOf {
        kind: TypeofKind,
        binding: Option<Ident>,
    },
}

define_symbols!(
    /// Access operators for accessing members or elements of values.
    ///
    /// These keywords represent different ways to access fields, methods, or
    /// indexed elements.
    pub enum AccessType {
        /// Access method or field of lvalue (`.`).
        AccessThis => ".", Symbol::Dot, u8::MAX,
        /// Access element by index of lvalue (`[`).
        AccessIndex => "[", Symbol::SquareOpen, u8::MAX,
    }
);

enum Loop {
    None,
    Continue,
    Break,
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

            Operator::Not | Operator::AtSign => return None,
        })
    }
}
