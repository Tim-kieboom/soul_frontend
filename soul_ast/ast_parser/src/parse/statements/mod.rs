use ast_model::{
    FunctionKind,
    block::{Block, BlockId},
    statements::{Statement, StatementId, Variable},
};
use soul_tokenizer::model::TokenKind;
use soul_utils::{
    TypeModifier,
    collections::try_result::{ResultTryErr, TryErr, TryError, TryNotValue, TryOk, TryResult},
    error::SoulResult,
    fault::Fault,
    soul_names::Symbol,
    span::Span,
};

use crate::{
    parse::statements::variable::AssignType,
    parser::Parser,
    utils::{
        ARROW_LEFT, COLON, COLON_ASSIGN, CURLY_CLOSE, CURLY_OPEN, DOT, ROUND_OPEN, SEMI_COLON,
        STAMENT_END_TOKENS, STAMENT_SKIP_TOKENS, STAR,
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
            modifier,
            statements,
            span: self.span_combine(start_span),
        }))
    }

    pub(super) fn inner_parse_statement(&mut self) -> SoulResult<Statement> {
        let begin_position = self.tokens.current_position();

        self.skip_while_any(STAMENT_SKIP_TOKENS);
        let start_span = self.token().span;

        let possible_kind = match &self.token().kind {
            TokenKind::Ident(_) => self.try_parse_from_ident(start_span),
            &ROUND_OPEN => {
                let saved = self.tokens.current_position();
                match self.parse_tuple_pattern() {
                    Ok(pattern) => match try_assign_type(&self.token()) {
                        Some(AssignType::Assign) | Some(AssignType::Declaration) => {
                            self.bump();
                            let statement = self.parse_expression_id(STAMENT_END_TOKENS)?;
                            return Ok(Statement::new_variable(
                                Variable {
                                    id: self.alloc_node(),
                                    is_public: false,
                                    pattern,
                                    ty: None,
                                    modifier: TypeModifier::Const,
                                    initialize_value: Some(statement),
                                },
                                self.span_combine(start_span),
                            ));
                        }
                        _ => {
                            self.goto(saved);
                            TryNotValue(Fault::empty())
                        }
                    },
                    Err(_) => {
                        self.goto(saved);
                        TryNotValue(Fault::empty())
                    }
                }
            }
            &CURLY_OPEN => {
                let saved = self.tokens.current_position();
                match (
                    self.parse_named_tuple_pattern(),
                    try_assign_type(&self.token()),
                ) {
                    (Ok(pattern), Some(AssignType::Assign))
                    | (Ok(pattern), Some(AssignType::Declaration)) => {
                        self.bump();
                        match self.parse_expression_id(STAMENT_END_TOKENS) {
                            Ok(value) => {
                                return Ok(Statement::new_variable(
                                    Variable {
                                        id: self.alloc_node(),
                                        is_public: false,
                                        pattern,
                                        ty: None,
                                        modifier: TypeModifier::Const,
                                        initialize_value: Some(value),
                                    },
                                    self.span_combine(start_span),
                                ));
                            }
                            Err(err) => return Err(err),
                        }
                    }
                    _ => {
                        self.goto(saved);
                        let block = self.parse_block(TypeModifier::Mut)?;
                        let span = self.span_combine(start_span);
                        let semicolon = self.ends_semicolon();
                        TryOk(Statement::new_block(&mut self.forest.store, block, span, semicolon))
                    }
                }
            }
            &STAR => return self.parse_assign_or_expression(start_span),
            TokenKind::Keyword(keyword) => {
                let kw = *keyword;
                match self.try_parse_from_keyword(start_span, kw) {
                    Ok(val) => TryOk(val),
                    Err(TryError::IsErr(err)) => TryErr(err),
                    Err(TryError::IsNotValue(err)) => TryNotValue(err),
                }
            }
            _ => TryNotValue(Fault::empty()),
        };

        match possible_kind {
            Ok(val) => return Ok(val),
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(_)) => (),
        };

        match self.parse_expression_id(STAMENT_END_TOKENS) {
            Ok(val) => {
                let semicolon = self.ends_semicolon();
                Ok(Statement::from_expression(&self.forest.store, val, semicolon))
            }
            Err(err) => {
                self.goto(begin_position);
                Err(err)
            }
        }
    }

    fn try_parse_from_ident(&mut self, start_span: Span) -> TryResult<Statement, Fault> {
        let ident = self.try_token_as_ident_str().try_err()?;
        let is_this = ident == "This";

        let peek = self.peek();
        match &peek.kind {
            &ROUND_OPEN | &ARROW_LEFT => self.parse_any_function().try_err(),
            &COLON | &COLON_ASSIGN => self.parse_variable().try_err(),
            &DOT if is_this => self.parse_contructor(start_span).try_err(),
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
                    try_assign_type(&self.token()),
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

    fn parse_contructor(&mut self, start_span: Span) -> SoulResult<Statement> {
        self.bump();
        let this = self.current.this_type.take();
        let result = match &this {
            Some(ty) => self.parse_function_contructor(ty, TypeModifier::Mut),
            None => Err(Fault::error(
                "contructor function should have methode type",
                Some(self.span_combine(start_span)),
            )),
        };
        self.current.this_type = this;
        result
            .map(|spanned| {
                spanned.map(|func| self.forest.store.insert_function(FunctionKind::Normal(func)))
            })
            .map(Statement::from_function)
    }
}

pub(super) fn try_assign_type(token: &soul_tokenizer::model::Token) -> Option<AssignType> {
    match &token.kind {
        TokenKind::Symbol(val) => AssignType::from_symbool(*val),
        _ => None,
    }
}
