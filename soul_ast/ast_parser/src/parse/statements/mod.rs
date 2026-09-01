use ast_model::{
    FunctionKind,
    block::{Block, BlockId},
    expression::Binding,
    statements::{Statement, StatementId, VarPattern, Variable},
};
use soul_tokenizer::model::TokenKind;
use soul_utils::{
    Ident, TypeModifier,
    collections::try_result::{ResultTryErr, TryErr, TryError, TryNotValue, TryOk, TryResult},
    error::SoulResult,
    fault::Fault,
    soul_names::Symbol,
    span::{Attribute, Span},
};

use crate::{
    parse::statements::variable::AssignType,
    parser::Parser,
    utils::{
        ARROW_LEFT, COLON, COLON_ASSIGN, CURLY_CLOSE, CURLY_OPEN, DOT, HASH, NOT, ROUND_OPEN,
        SEMI_COLON, SQUARE_CLOSE, SQUARE_OPEN, STAMENT_END_TOKENS, STAMENT_SKIP_TOKENS, STAR,
    },
};

mod assign;
mod from_keyword;
mod from_modfier;
mod import;
mod objects;
mod use_block;
mod variable;

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_global_statements(&mut self) -> Vec<StatementId> {
        self.skip_end_lines();
        let mut global_statements = vec![];

        while !self.current_is(&TokenKind::EndFile) {
            match self.parse_statement_id() {
                Ok(val) => global_statements.push(val),
                Err(err) => {
                    self.log_fault(err);
                    self.skip_over_statement();
                }
            }

            if self.current_is(&SEMI_COLON) {
                self.bump();
            }
            self.skip_end_lines();
        }

        global_statements
    }

    pub(super) fn skip_over_statement(&mut self) {
        let mut curly_bracket_stack = 0usize;

        while !self.current_is(&TokenKind::EndFile) {
            self.bump();

            if self.current_is(&CURLY_OPEN) {
                curly_bracket_stack = curly_bracket_stack.saturating_add(1)
            }

            if self.current_is(&CURLY_CLOSE) {
                curly_bracket_stack = curly_bracket_stack.saturating_sub(1)
            }

            if self.current_is_any(STAMENT_END_TOKENS) && curly_bracket_stack == 0 {
                if self.current_is(&CURLY_CLOSE) {
                    self.bump();
                }
                return;
            }
        }
    }

    pub(crate) fn parse_statement_id(&mut self) -> SoulResult<StatementId> {
        let value = self.parse_statement()?;
        Ok(self.forest.store.insert_statement(value))
    }

    pub(crate) fn parse_statement(&mut self) -> SoulResult<Statement> {
        let statement = self.inner_parse_statement()?;
        if !statement.is_expression() && self.ends_semicolon() {
            self.log_error(
                format!(
                    "`{}` at the end of a line can only be used for expressions at the end of a block", 
                    Symbol::SemiColon.as_str()
                ),
                Some(self.token().span),
            );
        }

        Ok(statement)
    }

    pub(crate) fn parse_block(&mut self, modifier: TypeModifier) -> SoulResult<BlockId> {
        const END_TOKENS: &[TokenKind] = &[CURLY_CLOSE, TokenKind::EndFile];
        let start_span = self.token().span;

        let mut statements = vec![];
        self.expect(&CURLY_OPEN)?;
        while !self.current_is_any(END_TOKENS) {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            match self.parse_statement_id() {
                Ok(statement) => statements.push(statement),
                Err(err) => {
                    self.log_fault(err);
                    self.skip_over_statement();
                }
            }

            self.skip_while_any(&[SEMI_COLON, TokenKind::EndLine]);
        }

        self.expect(&CURLY_CLOSE)?;
        Ok(self.forest.store.insert_block(Block {
            statements,
            span: self.span_combine(start_span),
            is_const: modifier == TypeModifier::Const,
        }))
    }

    pub(super) fn inner_parse_statement(&mut self) -> SoulResult<Statement> {
        let begin_position = self.tokens.current_position();

        self.skip_while_any(STAMENT_SKIP_TOKENS);
        let start_span = self.token().span;

        if self.current_is(&HASH) {
            let (attributes, attributes_span) = self.parse_statement_attributes()?;
            let mut statement = self.inner_parse_statement()?;
            let mut combined = attributes;
            combined.append(&mut statement.meta_data.attributes);
            statement.meta_data.attributes = combined;
            statement.span = attributes_span.combine(statement.span);
            return Ok(statement);
        }

        if let STAR = self.token().kind {
            return self.parse_assign_or_expression(start_span);
        }

        match self.parse_possible_statement(start_span) {
            Ok(val) => return Ok(val),
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(_)) => (),
        };

        match self.parse_expression_id(STAMENT_END_TOKENS) {
            Ok(val) => {
                let semicolon = self.ends_semicolon();
                Ok(Statement::from_expression(
                    &self.forest.store,
                    val,
                    semicolon,
                ))
            }
            Err(err) => {
                self.goto(begin_position);
                Err(err)
            }
        }
    }

    fn parse_possible_statement(&mut self, start_span: Span) -> TryResult<Statement, Fault> {
        match &self.token().kind {
            TokenKind::Ident(_) | TokenKind::Types(_) => self.try_parse_from_ident(start_span),
            &ROUND_OPEN => {
                let saved = self.tokens.current_position();

                let Ok(pattern) = self.parse_tuple_pattern() else {
                    self.goto(saved);
                    return TryNotValue(Fault::empty());
                };

                let assign = try_assign_type(self.token());
                if !matches!(
                    assign,
                    Some(AssignType::Assign) | Some(AssignType::Declaration)
                ) {
                    self.goto(saved);
                    return TryNotValue(Fault::empty());
                }

                self.bump();
                let statement = self.parse_expression_id(STAMENT_END_TOKENS).try_err()?;
                Ok(Statement::new_variable(
                    Variable {
                        id: self.alloc_node(),
                        is_public: false,
                        pattern,
                        ty: None,
                        modifier: TypeModifier::Const,
                        initialize_value: Some(statement),
                    },
                    self.span_combine(start_span),
                ))
            }
            &CURLY_OPEN => {
                let saved = self.tokens.current_position();
                match (
                    self.parse_named_tuple_pattern(),
                    try_assign_type(self.token()),
                ) {
                    (Ok(pattern), Some(AssignType::Assign))
                    | (Ok(pattern), Some(AssignType::Declaration)) => {
                        self.bump();
                        let value = self.parse_expression_id(STAMENT_END_TOKENS).try_err()?;

                        Ok(Statement::new_variable(
                            Variable {
                                id: self.alloc_node(),
                                is_public: false,
                                pattern,
                                ty: None,
                                modifier: TypeModifier::Const,
                                initialize_value: Some(value),
                            },
                            self.span_combine(start_span),
                        ))
                    }
                    _ => {
                        self.goto(saved);
                        let block = self.parse_block(TypeModifier::Mut).try_err()?;
                        let span = self.span_combine(start_span);
                        let semicolon = self.ends_semicolon();
                        TryOk(Statement::new_block(
                            &mut self.forest.store,
                            block,
                            span,
                            semicolon,
                        ))
                    }
                }
            }
            TokenKind::Keyword(keyword) => {
                let kw = *keyword;
                match self.try_parse_from_keyword(start_span, kw) {
                    Ok(val) => TryOk(val),
                    Err(TryError::IsErr(err)) => TryErr(err),
                    Err(TryError::IsNotValue(err)) => TryNotValue(err),
                }
            }
            &STAR => unreachable!(),
            _ => TryNotValue(Fault::empty()),
        }
    }

    /// Parses one or more attribute groups `#[ ident]` / `#[ ! ident]` that precede an item.
    ///
    /// Negated markers (`#[!Trait]`) store their name with a `!` prefix.
    pub(crate) fn parse_statement_attributes(&mut self) -> SoulResult<(Vec<Attribute>, Span)> {
        let start_span = self.token().span;
        let mut attributes = Vec::new();

        while self.current_is(&HASH) {
            self.bump();

            if !self.current_is(&SQUARE_OPEN) {
                return Err(self.get_expect_error(&SQUARE_OPEN));
            }
            self.bump();

            let mut name = String::new();
            if self.current_is(&NOT) {
                self.bump();
                name.push('!');
            }

            let token = self.bump_consume();
            let name_text = match token.kind {
                TokenKind::Ident(ident) => ident,
                TokenKind::Keyword(keyword) => keyword.as_str().to_string(),
                _ => return Err(self.get_expect_ident_error("attribute name")),
            };
            name.push_str(&name_text);

            if !self.current_is(&SQUARE_CLOSE) {
                return Err(self.get_expect_error(&SQUARE_CLOSE));
            }
            self.bump();

            attributes.push(Attribute {
                name: Ident::new(name, token.span),
                values: Vec::new(),
            });
        }

        Ok((attributes, self.span_combine(start_span)))
    }

    fn try_parse_from_ident(&mut self, start_span: Span) -> TryResult<Statement, Fault> {
        let ident = self.try_token_as_ident_str().try_err()?;
        let is_this = ident == "This";
        let is_unsafe = ident == "unsafe";
        let ident_owned = ident.to_string();

        let peek = self.peek();
        match &peek.kind {
            &ROUND_OPEN | &ARROW_LEFT => self.parse_any_function().try_err(),
            &COLON | &COLON_ASSIGN => self.parse_variable().try_err(),
            TokenKind::Symbol(Symbol::DoubleColon) => self
                .parse_associated_constant(ident_owned, start_span)
                .try_err(),
            &DOT if is_this => self.parse_contructor(start_span).try_err(),
            &DOT => self.parse_extension_function(start_span).try_err(),
            &CURLY_OPEN if is_unsafe => {
                self.bump();
                let block = self.parse_block(TypeModifier::Mut).try_err()?;
                let span = self.span_combine(start_span);
                let semicolon = self.ends_semicolon();
                Ok(Statement::new_block(
                    &mut self.forest.store,
                    block,
                    span,
                    semicolon,
                ))
            }
            &CURLY_OPEN => {
                let saved = self.tokens.current_position();
                let type_name = match self.try_bump_consume_ident() {
                    Ok(name) => name,
                    Err(_) => {
                        self.goto(saved);
                        return self.parse_assign_or_expression(start_span).try_err();
                    }
                };

                match (
                    self.parse_constructor_pattern(type_name),
                    try_assign_type(self.token()),
                ) {
                    (Ok(pattern), Some(AssignType::Assign))
                    | (Ok(pattern), Some(AssignType::Declaration)) => {
                        self.bump();
                        return self
                            .parse_expression_id(STAMENT_END_TOKENS)
                            .map(|value| {
                                Statement::new_variable(
                                    Variable {
                                        id: self.alloc_node(),
                                        is_public: false,
                                        pattern,
                                        ty: None,
                                        modifier: TypeModifier::Const,
                                        initialize_value: Some(value),
                                    },
                                    self.span_combine(start_span),
                                )
                            })
                            .try_err();
                    }
                    _ => {}
                }

                self.goto(saved);
                self.parse_assign_or_expression(start_span).try_err()
            }
            _ => self.parse_assign_or_expression(start_span).try_err(),
        }
    }

    fn parse_associated_constant(
        &mut self,
        name: String,
        start_span: Span,
    ) -> SoulResult<Statement> {
        self.bump();
        self.bump();
        let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
        Ok(Statement::new_variable(
            Variable {
                id: self.alloc_node(),
                is_public: false,
                pattern: VarPattern::Simple {
                    binding: Binding::new(self.alloc_node(), Ident::new(name, start_span)),
                    modifier: TypeModifier::Const,
                },
                ty: None,
                modifier: TypeModifier::Const,
                initialize_value: Some(value),
            },
            self.span_combine(start_span),
        ))
    }

    fn parse_contructor(&mut self, start_span: Span) -> SoulResult<Statement> {
        self.bump();
        let this = self.current.this_type.take();
        let result = match &this {
            Some(ty) => self.parse_function_contructor(ty, false),
            None => Err(Fault::error(
                "contructor function should have methode type",
                Some(self.span_combine(start_span)),
            )),
        };
        self.current.this_type = this;
        result
            .map(|spanned| {
                spanned.map(|func| {
                    self.forest
                        .store
                        .insert_function(FunctionKind::Normal(func))
                })
            })
            .map(Statement::from_function)
    }

    fn parse_extension_function(&mut self, start_span: Span) -> SoulResult<Statement> {
        let position = self.tokens.current_position();
        match self.inner_parse_extension_function(start_span) {
            Ok(stmt) => Ok(stmt),
            Err(_) => {
                self.goto(position);
                self.parse_assign_or_expression(start_span)
            }
        }
    }

    fn inner_parse_extension_function(&mut self, start_span: Span) -> SoulResult<Statement> {
        let receiver_ident = match &self.token().kind {
            TokenKind::Ident(val) => Ident::new(val.clone(), self.token().span),
            TokenKind::Types(val) => Ident::new(val.as_str(), self.token().span),
            other => {
                return Err(Fault::error(
                    format!("expected ident got `{}`", other.display()),
                    Some(self.token().span),
                ));
            }
        };
        self.bump();
        self.expect(&DOT)?;
        let method_ident = self.try_bump_consume_ident()?;

        let recv_type = self.type_from_ident(receiver_ident, vec![]);
        let saved = self.current.this_type.take();
        self.current.this_type = Some(recv_type.clone());
        let result =
            self.try_parse_function_declaration_id(start_span, &recv_type, false, method_ident);
        self.current.this_type = saved;
        match result {
            Ok(spanned) => Ok(Statement::from_function(spanned)),
            Err(TryError::IsErr(fault)) => Err(fault),
            Err(TryError::IsNotValue(err)) => Err(err.fault),
        }
    }
}

pub(super) fn try_assign_type(token: &soul_tokenizer::model::Token) -> Option<AssignType> {
    match &token.kind {
        TokenKind::Symbol(val) => AssignType::from_symbool(*val),
        _ => None,
    }
}
