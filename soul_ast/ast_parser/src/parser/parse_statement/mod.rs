use ast::{Block, Statement, StatementKind};
use soul_tokenizer::TokenKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{KeyWord, TypeModifier},
    span::Span,
    symbool_kind::SymbolKind,
    try_result::{
        ResultTryErr, ResultTryNotValue, TryErr, TryError, TryNotValue, TryOk, TryResult,
    },
};

use crate::parser::{
    Parser,
    parse_utils::{
        ARROW_LEFT, COLON, COLON_ASSIGN, CURLY_CLOSE, CURLY_OPEN, ROUND_OPEN, SEMI_COLON,
        STAMENT_END_TOKENS, STAR,
    },
};

mod from_keyword;
mod from_modifier;
mod parse_assign;
mod parse_import;
mod parse_variable;

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_global_statments(&mut self) -> Vec<Statement> {
        self.skip_end_lines();
        let mut global_statements = vec![];

        while !self.current_is(&TokenKind::EndFile) {
            match self.parse_statement() {
                Ok(val) => global_statements.push(val),
                Err(err) => {
                    self.log_error(err);
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

    pub(crate) fn parse_block(&mut self, modifier: TypeModifier) -> SoulResult<Block> {
        const END_TOKENS: &[TokenKind] = &[CURLY_CLOSE, TokenKind::EndFile];
        let start_span = self.token().span;

        let mut statements = vec![];

        self.expect(&CURLY_OPEN)?;
        while !self.current_is_any(END_TOKENS) {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            match self.parse_statement() {
                Ok(statement) => statements.push(statement),
                Err(err) => {
                    self.log_error(err);
                    self.skip_over_statement();
                }
            }

            self.skip_till(&[SEMI_COLON, TokenKind::EndLine]);
        }

        self.expect(&CURLY_CLOSE)?;
        Ok(Block {
            modifier,
            statements,
            node_id: None,
            scope_id: None,
            span: self.span_combine(start_span),
        })
    }

    pub(crate) fn parse_statement(&mut self) -> SoulResult<Statement> {
        let statement = self.inner_parse_statement()?;
        if !matches!(statement.node, StatementKind::Expression { .. }) && self.ends_semicolon() {
            self.log_error(SoulError::new(
                format!("'{}' at the end of a line can only be used for expressions at the end of a block", SymbolKind::SemiColon.as_str()),
                SoulErrorKind::InvalidEscapeSequence,
                Some(self.token().span),
            ));
        }
        Ok(statement)
    }

    fn inner_parse_statement(&mut self) -> SoulResult<Statement> {
        let begin_position = self.current_position();
        let start_span = self.token().span;

        self.skip_till(STAMENT_END_TOKENS);

        let possible_kind = match &self.token().kind {
            TokenKind::Ident(_) => self.try_parse_from_ident(start_span),
            &CURLY_OPEN => TryOk(Statement::new_block(
                self.parse_block(TypeModifier::Mut)?,
                self.span_combine(start_span),
                self.ends_semicolon(),
            )),
            &STAR => return self.parse_assign(start_span),
            TokenKind::Unknown(char) => {
                return Err(SoulError::new(
                    format!("unknown character: '{char}'"),
                    SoulErrorKind::UnexpectedCharacter,
                    Some(start_span),
                ));
            }
            _ => TryNotValue(SoulError::empty()),
        };

        match possible_kind {
            Ok(val) => return Ok(val),
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(_)) => (),
        };

        match self.parse_expression(STAMENT_END_TOKENS) {
            Ok(val) => Ok(Statement::from_expression(val, self.ends_semicolon())),
            Err(err) => {
                self.go_to(begin_position);
                Err(err)
            }
        }
    }

    pub(super) fn skip_over_statement(&mut self) {
        let mut curly_bracket_stack = 0_usize;

        while !self.current_is(&TokenKind::EndFile) {
            self.bump();

            if self.current_is(&CURLY_OPEN) {
                curly_bracket_stack = curly_bracket_stack.saturating_add(1)
            }

            if self.current_is(&CURLY_CLOSE) {
                curly_bracket_stack = curly_bracket_stack.saturating_sub(1)
            }

            if self.current_is_any(STAMENT_END_TOKENS) && curly_bracket_stack == 0 {
                return;
            }
        }
    }

    fn try_parse_from_ident(&mut self, start_span: Span) -> TryResult<Statement, SoulError> {
        let ident = self.try_token_as_ident_str().try_err()?;

        if let Some(modifier) = TypeModifier::from_str(ident) {
            return self.try_parse_from_modifier(start_span, modifier);
        }

        if let Some(keyword) = KeyWord::from_str(ident) {
            return self.try_parse_from_keyword(start_span, keyword);
        }

        match &self.peek().kind {
            &ROUND_OPEN | &ARROW_LEFT => self.parse_any_function().try_err(),
            &COLON | &COLON_ASSIGN => self.parse_variable().try_err(),
            _ => self.parse_from_unknown_ident(start_span).try_err(),
        }
    }

    fn parse_from_unknown_ident(&mut self, start_span: Span) -> SoulResult<Statement> {
        match self.try_parse_methode(start_span) {
            Ok(val) => return Ok(val),
            Err(TryError::IsErr(err)) => return Err(err),
            _ => (),
        };

        self.parse_assign(start_span)
    }

    fn try_parse_methode(&mut self, start_span: Span) -> TryResult<Statement, ()> {
        let begin = self.current_position();
        let result = self.inner_parse_methode(start_span);
        if result.is_err() {
            self.go_to(begin);
        }

        result
    }

    fn inner_parse_methode(&mut self, start_span: Span) -> TryResult<Statement, ()> {
        let mut methode_type = match self.try_parse_type() {
            Ok(val) => val,
            Err(TryError::IsErr(err)) => return TryErr(err),
            _ => return TryNotValue(()),
        };

        if methode_type.modifier.is_none() {
            methode_type.modifier = Some(TypeModifier::Mut);
        }

        let name = self.try_bump_consume_ident().try_not_value()?;
        match self.try_parse_function_declaration(start_span, methode_type, name) {
            Ok(val) => TryOk(Statement::from_function(val)),
            Err(TryError::IsErr(err)) => TryErr(err),
            Err(TryError::IsNotValue(_)) => TryNotValue(()),
        }
    }
}
