use crate::{
    steps::{
        parse::{
            ARROW_LEFT, ASSIGN, COLON, COLON_ASSIGN, CURLY_CLOSE, CURLY_OPEN, ROUND_OPEN,
            SEMI_COLON, STAMENT_END_TOKENS, parser::Parser,
        },
        tokenize::token_stream::TokenKind,
    },
    utils::try_result::{ResultTryErr, ToResult, TryErr, TryError, TryNotValue, TryOk, TryResult},
};
use models::{
    abstract_syntax_tree::{
        expression::{Expression, ExpressionKind, ReturnKind},
        operator::{Binary, BinaryOperator, BinaryOperatorKind},
        soul_type::SoulType,
        statment::{Assignment, Ident, Statement, StatementKind, Variable},
    },
    error::{SoulError, SoulErrorKind, SoulResult, Span},
    soul_names::{self, AssignType, KeyWord, TypeModifier},
};
use std::{
    iter::{self},
    sync::LazyLock,
};

impl<'a> Parser<'a> {
    pub(crate) fn parse_statement(&mut self) -> SoulResult<Statement> {
        let begin_position = self.current_position();
        let start_span = self.token().span;

        self.skip_till(STAMENT_END_TOKENS);

        let possible_kind = match &self.token().kind {
            TokenKind::Ident(ident) => {
                if let Some(modifier) = soul_names::TypeModifier::from_str(ident) {
                    self.try_parse_statement_modifier(start_span, modifier)
                } else if let Some(keyword) = soul_names::KeyWord::from_str(ident) {
                    self.parse_stament_keyword(start_span, keyword)
                } else {
                    TryOk(self.parse_statement_ident(start_span)?)
                }
            }
            &CURLY_OPEN => TryOk(Statement::new_block(
                self.parse_block(TypeModifier::Mut)?,
                self.new_span(start_span),
            )),
            TokenKind::Unknown(char) => {
                return Err(SoulError::new(
                    format!("unknown char: '{char}'"),
                    SoulErrorKind::InvalidChar,
                    Some(start_span),
                ));
            }
            _ => TryNotValue(SoulError::empty()),
        };

        match possible_kind {
            Ok(val) => Ok(val),
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue(_)) => match self.parse_expression(STAMENT_END_TOKENS) {
                Ok(expression) => Ok(Statement::new(
                    StatementKind::Expression(expression),
                    self.new_span(start_span),
                )),
                Err(err) => {
                    self.go_to(begin_position);
                    Err(err)
                }
            },
        }
    }

    pub(crate) fn skip_over_statement(&mut self) {
        let mut curly_bracket_stack = 0usize;

        while !self.current_is(&TokenKind::EndFile) {
            self.bump();

            if self.current_is(&CURLY_OPEN) {
                curly_bracket_stack = curly_bracket_stack.saturating_add(1)
            } else if self.current_is(&CURLY_CLOSE) {
                curly_bracket_stack = curly_bracket_stack.saturating_sub(1)
            }

            if self.current_is_any(STAMENT_END_TOKENS) && curly_bracket_stack == 0 {
                return;
            }
        }
    }

    fn parse_statement_ident(&mut self, start_span: Span) -> SoulResult<Statement> {
        const FUNCTION_IDS: &[TokenKind] = &[ROUND_OPEN, ARROW_LEFT];
        const DECLARATION_IDS: &[TokenKind] = &[COLON, COLON_ASSIGN];

        let peek = self.peek();

        if FUNCTION_IDS.contains(&peek.kind) {
            let ident_token = self.bump_consume();
            let ident = match ident_token.kind {
                TokenKind::Ident(val) => val,
                _ => unreachable!(),
            };

            let result =
                self.try_parse_function_declaration(start_span, TypeModifier::Mut, None, ident);

            match result {
                Ok(val) => Ok(val),
                Err(TryError::IsErr(err)) => Err(err),
                Err(TryError::IsNotValue(err)) => {
                    let (ident, _ty) = (err.0, err.1);
                    self.try_parse_function_call(start_span, None, ident)
                        .merge_to_result()
                        .map(Statement::from_function_call)
                }
            }
        } else if DECLARATION_IDS.contains(&peek.kind) {
            let ident_token = self.bump_consume();
            let ident = match ident_token.kind {
                TokenKind::Ident(val) => val,
                _ => unreachable!(),
            };

            let (ty, assign_type) = if self.current_is(&COLON) {
                self.bump();
                let ty = match self.try_parse_type() {
                    Ok(val) => val,
                    Err(TryError::IsErr(err)) | Err(TryError::IsNotValue(err)) => return Err(err),
                };
                self.expect_any(&[COLON_ASSIGN, ASSIGN])?;
                (Some(ty), AssignType::Assign)
            } else {
                (None, AssignType::Assign)
            };

            let variable =
                self.parse_variable_declaration(start_span, ident.clone(), assign_type, ty)?;

            Ok(Statement::new(
                StatementKind::Variable(variable),
                self.new_span(start_span),
            ))
        } else {
            self.parse_unknown_ident(start_span)
        }
    }

    fn try_parse_statement_modifier(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
    ) -> TryResult<Statement, SoulError> {
        self.bump();

        if self.token().kind == CURLY_OPEN {
            let block = self.parse_block(modifier).try_err()?;
            return TryOk(Statement::new_block(block, self.new_span(start_span)));
        }

        if let Some(name) = self.try_consume_name(start_span).try_err()? {
            if self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
                todo!("function call/decl")
            }

            let mut ty = None;
            if self.current_is(&COLON) {
                self.bump();
                ty = Some(self.try_parse_type()?);
            }

            if self.current_is_any(STAMENT_END_TOKENS) {
                let ty = ty.unwrap_or(SoulType::none(self.token().span));
                return TryOk(Statement::new(
                    StatementKind::Variable(Variable {
                        name,
                        ty,
                        initialize_value: None,
                        node_id: None,
                    }),
                    self.new_span(start_span),
                ));
            }

            if let Some(assign_type) = try_get_assign_type(&self.token().kind) {
                let variable = self
                    .parse_variable_declaration(start_span, name.clone(), assign_type, ty)
                    .try_err()?;

                return TryOk(Statement::new(
                    StatementKind::Variable(variable),
                    self.new_span(start_span),
                ));
            } else {
                return TryErr(SoulError::new(
                    format!("'{}' should be '=' or ':='", self.token().kind.display()),
                    SoulErrorKind::InvalidAssignType,
                    Some(self.new_span(start_span)),
                ));
            }
        }

        TryErr(SoulError::new(
            format!(
                "'{}' invalid after modifier (could be ['{{' or <name>])",
                self.token().kind.display()
            ),
            SoulErrorKind::UnexpecedToken,
            Some(self.new_span(start_span)),
        ))
    }

    fn parse_stament_keyword(
        &mut self,
        start_span: Span,
        keyword: KeyWord,
    ) -> TryResult<Statement, SoulError> {
        let kind = match keyword {
            KeyWord::Use => Statement::new(
                StatementKind::UseBlock(self.parse_use_block().try_err()?),
                self.new_span(start_span),
            ),
            KeyWord::Enum => todo!("enum decl"),
            KeyWord::Class => Statement::new(
                StatementKind::Class(self.parse_class().try_err()?),
                self.new_span(start_span),
            ),
            KeyWord::Trait => Statement::new(
                StatementKind::Trait(self.parse_trait().try_err()?),
                self.new_span(start_span),
            ),
            KeyWord::Union => todo!("union decl"),
            KeyWord::Struct => Statement::new(
                StatementKind::Struct(self.parse_struct().try_err()?),
                self.new_span(start_span),
            ),
            KeyWord::If => Statement::from_expression(self.parse_if().try_err()?),
            KeyWord::For => Statement::from_expression(self.parse_for().try_err()?),
            KeyWord::Match => Statement::from_expression(self.parse_match().try_err()?),
            KeyWord::While => Statement::from_expression(self.parse_while().try_err()?),
            KeyWord::Return => {
                Statement::from_expression(self.parse_return_like(ReturnKind::Return).try_err()?)
            }
            KeyWord::Continue => {
                Statement::from_expression(self.parse_return_like(ReturnKind::Continue).try_err()?)
            }
            KeyWord::Break => {
                Statement::from_expression(self.parse_return_like(ReturnKind::Break).try_err()?)
            }
            KeyWord::Else => {
                return TryErr(SoulError::new(
                    format!(
                        "can not have '{}' without first '{}'",
                        KeyWord::Else.as_str(),
                        KeyWord::If.as_str()
                    ),
                    SoulErrorKind::InvalidExpression,
                    Some(self.new_span(start_span)),
                ));
            }

            KeyWord::Import => self.parse_import().try_err()?,
            KeyWord::Typeof
            | KeyWord::Dyn
            | KeyWord::Impl
            | KeyWord::Copy
            | KeyWord::Await
            | KeyWord::InForLoop
            | KeyWord::GenericWhere => {
                return TryErr(SoulError::new(
                    format!("keyword: '{}' invalid start to statment", keyword.as_str()),
                    SoulErrorKind::UnexpecedStatmentStart,
                    Some(self.new_span(start_span)),
                ));
            }
        };

        TryOk(kind)
    }

    fn parse_unknown_ident(&mut self, start_span: Span) -> SoulResult<Statement> {
        static ASSIGNMENT_TOKENS: LazyLock<Vec<TokenKind>> = LazyLock::new(|| {
            AssignType::SYMBOLS
                .iter()
                .map(|sym| TokenKind::Symbool(*sym))
                .chain(iter::once(TokenKind::EndLine))
                .chain(iter::once(SEMI_COLON))
                .collect()
        });

        match self.parse_single_extention_methode() {
            Ok(val) => return Ok(val),
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(_)) => (),
        }

        let lvalue = self.parse_expression(ASSIGNMENT_TOKENS.as_ref())?;
        if self.current_is_any(STAMENT_END_TOKENS) {
            return Ok(Statement::new(
                StatementKind::Expression(lvalue),
                self.new_span(start_span),
            ));
        }

        let assign_token = self.bump_consume();
        let assign = try_get_assign_type(&assign_token.kind).ok_or(SoulError::new(
            format!(
                "'{}' should be an assign symbool",
                assign_token.kind.display()
            ),
            SoulErrorKind::InvalidAssignType,
            Some(self.new_span(start_span)),
        ))?;

        let rvalue = self.parse_expression(STAMENT_END_TOKENS)?;
        let resolved_rvalue = resolve_assign_type(&lvalue, assign, assign_token.span, rvalue);

        self.bump();

        Ok(Statement::new(
            StatementKind::Assignment(Assignment {
                left: lvalue,
                right: resolved_rvalue,
            }),
            self.new_span(start_span),
        ))
    }

    fn parse_single_extention_methode(&mut self) -> TryResult<Statement, ()> {
        let start_span = self.token().span;
        let begin_position = self.current_position();

        let ty = match self.try_parse_type() {
            Ok(val) => val,
            Err(TryError::IsErr(err)) => return TryErr(err),
            Err(TryError::IsNotValue(_)) => return TryNotValue(()),
        };

        let modifier = ty.modifier.unwrap_or(TypeModifier::Mut);
        let callee = Some(ty);

        let ident_token = self.bump_consume();
        let name = match ident_token.kind {
            TokenKind::Ident(val) => val,
            _ => {
                self.go_to(begin_position);
                return TryNotValue(());
            }
        };

        match self.try_parse_function_declaration(start_span, modifier, callee, name) {
            Ok(val) => TryOk(val),
            Err(TryError::IsErr(err)) => TryErr(err),
            Err(TryError::IsNotValue(_)) => {
                self.go_to(begin_position);
                TryNotValue(())
            }
        }
    }

    fn parse_variable_declaration(
        &mut self,
        start_span: Span,
        variable_name: Ident,
        assign_type: AssignType,
        ty: Option<SoulType>,
    ) -> SoulResult<Variable> {
        let expression = match assign_type {
            AssignType::Declaration | AssignType::Assign => {
                self.bump();
                self.parse_expression(STAMENT_END_TOKENS)?
            }
            other => {
                return Err(SoulError::new(
                    format!(
                        "'{}' is not valid for variable declaration (can use ['=', ':='])",
                        other.as_str()
                    ),
                    SoulErrorKind::InvalidAssignType,
                    Some(self.new_span(start_span)),
                ));
            }
        };

        let variable = Variable {
            ty: ty.unwrap_or(SoulType::none(self.token().span)),
            name: variable_name,
            initialize_value: Some(expression),
            node_id: None,
        };
        Ok(variable)
    }
}

fn try_get_assign_type(token: &TokenKind) -> Option<AssignType> {
    if let TokenKind::Symbool(symbool) = token {
        AssignType::from_symbool(*symbool)
    } else {
        None
    }
}

fn resolve_assign_type(
    lvalue: &Expression,
    assign_type: AssignType,
    assign_span: Span,
    rvalue: Expression,
) -> Expression {
    let full_span = assign_span.combine(rvalue.span);

    let operator = match assign_type {
        AssignType::AddAssign => BinaryOperator::new(BinaryOperatorKind::Add, assign_span),
        AssignType::SubAssign => BinaryOperator::new(BinaryOperatorKind::Sub, assign_span),
        AssignType::MulAssign => BinaryOperator::new(BinaryOperatorKind::Mul, assign_span),
        AssignType::DivAssign => BinaryOperator::new(BinaryOperatorKind::Div, assign_span),
        AssignType::ModAssign => BinaryOperator::new(BinaryOperatorKind::Mod, assign_span),
        AssignType::BitOrAssign => BinaryOperator::new(BinaryOperatorKind::BitOr, assign_span),
        AssignType::BitAndAssign => BinaryOperator::new(BinaryOperatorKind::BitAnd, assign_span),
        AssignType::BitXorAssign => BinaryOperator::new(BinaryOperatorKind::BitXor, assign_span),

        AssignType::Assign | AssignType::Declaration => return rvalue,
    };

    Expression::new(
        ExpressionKind::Binary(Binary::new(lvalue.clone(), operator, rvalue)),
        full_span,
    )
}
