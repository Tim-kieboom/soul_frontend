use std::mem::swap;

use ast_model::{
    block::{Block, BlockId}, expression::{
        Array, Binding, Constructor, Expression, ExpressionId, ExpressionKind, MatchMethod, MatchMethodArm, TypeOf, VariableExpression
    }, literal::Literal, operators::{BinaryOperator, BinaryOperatorKind, UnaryOperator, UnaryOperatorKind}, statements::Statement
};
use soul_tokenizer::model::{Token, TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident, TypeModifier,
    collections::try_result::{ToResult, TryError},
    define_symbols,
    error::SoulResult,
    fault::Fault,
    literal::{Number, StringLiteral, TokenLiteral as tokenLiteral},
    soul_error_internal,
    soul_names::{Operator, Symbol},
    span::{Span, Spanned},
};

use crate::{
    parse::expression::precedence::Precedence,
    parser::Parser,
    utils::{
        ARRAY, ARROW_LEFT, COLON, CURLY_CLOSE, CURLY_OPEN, DOT, LAMBDA_ARROW, ROUND_CLOSE, ROUND_OPEN, SQUARE_CLOSE, SQUARE_OPEN
    },
};

mod conditionals;
mod group_expressions;
mod precedence;

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_expression_id(
        &mut self,
        end_tokens: &[TokenKind],
    ) -> SoulResult<ExpressionId> {
        let value = self.pratt_parse_expression(Precedence::MIN, end_tokens)?;
        Ok(self.store.insert_expression(value))
    }

    pub(crate) fn parse_expression(&mut self, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        self.pratt_parse_expression(Precedence::MIN, end_tokens)
    }

    fn pratt_parse_expression(
        &mut self,
        min_precedence: Precedence,
        end_tokens: &[TokenKind],
    ) -> SoulResult<Expression> {
        let start_span = self.token().span;

        let mut prefix_operators = vec![];
        self.collect_prefix_operators(&mut prefix_operators, start_span);

        let mut left = self.parse_primary(end_tokens)?;

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

            let token_kind = &self.token().kind;
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
        
        left = self.apply_prefix_operators(left, prefix_operators);

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
                    let right = self.pratt_parse_expression(next_min_precedence, end_tokens)?;
                    let span = self.span_combine(start_span);
                    left = Expression::new_binary(
                        self.store.insert_expression(left), 
                        operator, 
                        self.store.insert_expression(right), 
                        span,
                    );
                }
                ExpressionOperator::TypeOf { 
                    type_name, 
                    variant_name, 
                    binding, 
                } => {
                    let typeof_ = TypeOf {
                        type_name,
                        binding,
                        variant_name,
                        binding_id: None,
                        value: self.store.insert_expression(left),
                    };
                    left = Expression::new(
                        ExpressionKind::TypeOf(typeof_), 
                        self.span_combine(start_span)
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
            self.go_to(saved);
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
        match &self.token().kind {
            TokenKind::Ident(ident) => {
                if let Some(keyword) = KeyWord::from_str(ident) {
                    Precedence::new(keyword.precedence())
                } else {
                    Precedence::MIN
                }
            }
            TokenKind::Symbol(symbool_kind) => {
                if let Some(bin) = try_to_binary_operator(symbool_kind) {
                    let peek = self.peek();
                    Precedence::new(
                        self.try_multi_binary(&peek, bin).precedence()
                    )
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

    fn access_index_expression(
        &mut self,
        left: &mut Expression,
        start_span: Span,
    ) -> SoulResult<()> {
        let index = self.parse_expression_id(&[SQUARE_CLOSE, TokenKind::EndLine, TokenKind::EndFile])?;
        self.expect(&SQUARE_CLOSE)?;

        let mut value = Expression::error();
        swap(left, &mut value);

        let id = self.store.insert_expression(value);
        *left = Expression::new_index(id, index, self.span_combine(start_span));
        Ok(())
    }

    fn access_this_expression(
        &mut self,
        left: &mut Expression,
        start_span: Span,
    ) -> SoulResult<()> {
        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_define().merge_to_result()?
        } else {
            vec![]
        };

        if self.current_is(&ROUND_OPEN) {
            let mut value = Expression::error();
            swap(left, &mut value);

            let name = match value.node {
                ExpressionKind::Variable(VariableExpression { name, .. }) => name,
                _ => return Err(Fault::error(
                    "should be ident",
                    Some(value.span)
                ))
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

        let ident = self.try_bump_consume_ident()?;

        if self.current_is(&CURLY_OPEN) {
            let (binding, body) = self.parse_match_method_arm(start_span)?;

            if let ExpressionKind::MatchMethod(ref mut method) = left.node {
                method.arms.push(MatchMethodArm {
                    variant_name: ident,
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
                        variant_name: ident,
                        binding,
                        body,
                    }],
                };
                *left = Expression::new(
                    ExpressionKind::MatchMethod(method),
                    self.span_combine(start_span),
                );
            }

            return Ok(());
        }

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

    fn parse_field_access(&mut self, left: ExpressionId, ident: Ident) -> SoulResult<Expression> {
        match KeyWord::from_str(ident.as_str()) {
            Some(KeyWord::Sizeof) => self.parse_sizeof(left, ident),
            _ => Ok(Expression::new_field(self.store, left, ident)),
        }
    }

    fn parse_sizeof(&mut self, left_id: ExpressionId, ident: Ident) -> SoulResult<Expression> {
        let left = &self.store.expressions[left_id];
        let left_span = left.span;
        match &left.node {
            ExpressionKind::Variable(VariableExpression { name, .. }) => {
                let span = name.span();
                let ty = self.type_from_ident(name.clone(), vec![]);
                Ok(Expression::new(
                    ExpressionKind::Sizeof(ty),
                    span.combine(left_span),
                ))
            }
            _ => Err(Fault::error(
                "can only do '.sizeof' to types",
                Some(ident.span()),
            )),
        }
    }

    fn parse_match_method_arm(
        &mut self,
        start_span: Span,
    ) -> SoulResult<(Option<Binding>, BlockId)> {
        let save_pos = self.tokens.current_position();

        self.expect(&CURLY_OPEN)?;
        self.skip_end_lines();

        let Ok(ident) = self.try_bump_consume_ident() else {
            self.go_to(save_pos);
            let body = self.parse_block(TypeModifier::Mut)?;
            return Ok((None, body));
        };

        self.skip_end_lines();
        if self.current_is(&LAMBDA_ARROW) {
            self.bump();
            let expression =
                self.parse_expression_id(&[CURLY_CLOSE, TokenKind::EndLine, TokenKind::EndFile])?;
            self.expect(&CURLY_CLOSE)?;
            let statement = self
                .store
                .insert_statement(Statement::from_expression(self.store, expression, false));
            let block = self.store.insert_block(Block {
                modifier: TypeModifier::Mut,
                statements: vec![statement],
                scope_id: None,
                node_id: None,
                span: self.span_combine(start_span),
            });
            return Ok((Some(Binding::new(ident)), block));
        }

        self.go_to(save_pos);
        let body = self.parse_block(TypeModifier::Mut)?;
        Ok((None, body))
    }

    fn consume_expression_operator(&mut self, start_span: Span) -> SoulResult<ExpressionOperator> {
        fn get_invalid_error(token: &Token) -> SoulResult<ExpressionOperator> {
            Err(Fault::error(
                format!("'{}' is not a valid operator", token.kind.display()),
                Some(token.span),
            ))
        }

        match &self.token().kind {
            TokenKind::Ident(ident) => match KeyWord::from_str(ident.as_str()) {
                Some(KeyWord::Typeof) => {
                    self.bump();
                    let type_name = self.try_bump_consume_ident()?;
                    self.expect(&TokenKind::Symbol(Symbol::Dot))?;
                    let variant_name = self.try_bump_consume_ident()?;
                    let binding = if self.current_is(&TokenKind::Symbol(Symbol::RoundOpen)) {
                        self.bump();
                        let name = self.try_bump_consume_ident()?;
                        self.expect(&TokenKind::Symbol(Symbol::RoundClose))?;
                        Some(name)
                    } else {
                        None
                    };
                    return Ok(ExpressionOperator::TypeOf {
                        type_name,
                        variant_name,
                        binding,
                    });
                }
                _ => get_invalid_error(self.token()),
            },
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

                get_invalid_error(self.token())
            }

            _ => get_invalid_error(self.token()),
        }
    }

    fn try_consume_multi_binary(&mut self, binary: BinaryOperatorKind) -> BinaryOperatorKind {
        let bin = self.try_multi_binary(self.token(), binary);
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

    fn parse_primary(&mut self, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        let start_span = self.token().span;

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
                return Err(soul_error_internal!("tuple not yet impl", Some(start_span)));
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
            TokenKind::Literal(tokenLiteral::Char(char)) => {
                let char = *char;
                self.bump();
                Expression::new_literal(Literal::Char(char), start_span)
            }
            TokenKind::Literal(tokenLiteral::String(_)) => {
                let token = self.bump_consume();
                let string = match token.kind {
                    TokenKind::Literal(tokenLiteral::String(val)) => val,
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
            TokenKind::Literal(tokenLiteral::Number(num)) => {
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

    fn parse_primary_ident(
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
                    Ok(val) => return Ok(Expression::from_function_call(val)),
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

    fn parse_primary_keyword(&mut self, start_span: Span) -> SoulResult<Option<Expression>> {
        let ident = self.try_token_as_ident_str()?;
        match KeyWord::from_str(ident) {
            Some(keyword) => self.parse_keyword_primary(start_span, keyword),
            None => Ok(None),
        }
    }

    fn parse_keyword_primary(&mut self, start_span: Span, keyword: KeyWord) -> SoulResult<Option<Expression>> {
        Ok(Some(match keyword {
            KeyWord::If => self.parse_if()?,
            KeyWord::Match => self.parse_match()?,

            KeyWord::True | KeyWord::False => {
                let value = keyword == KeyWord::True;
                self.bump();
                Expression::new_literal(Literal::Bool(value), self.token().span)
            }

            KeyWord::Null => {
                self.bump();
                Expression::new(ExpressionKind::Null(None), self.token().span)
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
                    &SQUARE_OPEN => self.parse_new_array(start_span)?,
                    _ => {
                        return Err(Fault::error(
                            "expected '(' or ':[' after 'new'".to_string(),
                            Some(self.token().span),
                        ));
                    }
                }
            }

            _ => return Ok(None),
        }))
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
        self.expect(&COLON)?;
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
    fn collect_prefix_operators(
        &mut self,
        prefix_ops: &mut Vec<(Span, UnaryKinds)>,
        start_span: Span,
    ) {
        while let TokenKind::Symbol(symbol) = &self.token().kind {
            match self.expect_unary_kind(start_span, *symbol) {
                Ok(unary_kind) => {
                    self.bump();
                    prefix_ops.push((self.span_combine(start_span), unary_kind));
                }
                Err(_) => break,
            }
        }
    }

    fn expect_unary_kind(&self, start_span: Span, symbool: Symbol) -> SoulResult<UnaryKinds> {
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
            Operator::BitAnd => Ok(UnaryKinds::Ref { mutable: true }),
            Operator::ConstRef => Ok(UnaryKinds::Ref { mutable: false }),
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
    match Operator::from_symbool(*symbol).map(|el| el.to_unary())  {
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
        type_name: Ident,
        variant_name: Ident,
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

            Operator::Not | Operator::ConstRef => return None,
        })
    }
}
