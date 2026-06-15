use ast_model::{
    block::{Block, BlockId},
    statements::{Statement, StatementId},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    TypeModifier,
    collections::try_result::{
        ResultMapNotValue, ResultTryErr, ResultTryNotValue, TryErr, TryError, TryNotValue, TryOk,
        TryResult,
    },
    error::SoulResult,
    fault::Fault,
    soul_names::Symbol,
    span::Span,
};

use crate::{
    parser::Parser,
    utils::{
        ARROW_LEFT, COLON, COLON_ASSIGN, CURLY_CLOSE, CURLY_OPEN, ROUND_OPEN, SEMI_COLON,
        SQUARE_OPEN, STAMENT_END_TOKENS, STAMENT_SKIP_TOKENS, STAR,
    },
};

mod assign;
mod from_keyword;
mod from_modfier;
mod import;
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

    pub(crate) fn parse_statement_id(&mut self) -> SoulResult<StatementId> {
        let value = self.parse_statement()?;
        Ok(self.store.insert_statement(value))
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
        Ok(self.store.insert_block(Block {
            modifier,
            statements,
            node_id: None,
            scope_id: None,
            span: self.span_combine(start_span),
        }))
    }

    pub(super) fn inner_parse_statement(&mut self) -> SoulResult<Statement> {
        let begin_position = self.tokens.current_position();

        self.skip_while_any(STAMENT_SKIP_TOKENS);
        let start_span = self.token().span;

        let possible_kind = match &self.token().kind {
            TokenKind::Ident(_) => self.try_parse_from_ident(start_span),
            &CURLY_OPEN => {
                let block = self.parse_block(TypeModifier::Mut)?;
                let span = self.span_combine(start_span);
                let semicolon = self.ends_semicolon();

                TryOk(Statement::new_block(self.store, block, span, semicolon))
            }
            &SQUARE_OPEN => self
                .try_parse_methode(start_span)
                .map_try_not_value(|_| Fault::empty()),
            &STAR => return self.parse_assign(start_span),
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
                Ok(Statement::from_expression(self.store, val, semicolon))
            }
            Err(err) => {
                self.go_to(begin_position);
                Err(err)
            }
        }
    }

    fn try_parse_from_ident(&mut self, start_span: Span) -> TryResult<Statement, Fault> {
        let ident = self.try_token_as_ident_str().try_err()?;
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
        let begin = self.tokens.current_position();
        let result = self.inner_parse_methode(start_span);
        if result.is_err() {
            self.go_to(begin);
        }

        result
    }

    fn inner_parse_methode(&mut self, start_span: Span) -> TryResult<Statement, ()> {
        let modifier = self.try_bump_type_modiffier().unwrap_or(TypeModifier::Mut);

        let methode_type = match self.try_parse_type() {
            Ok(val) => val,
            Err(TryError::IsErr(err)) => return TryErr(err),
            _ => return TryNotValue(()),
        };

        let name = self.try_bump_consume_ident().try_not_value()?;
        match self.try_parse_function_declaration(start_span, modifier, methode_type, name) {
            Ok(val) => TryOk(Statement::from_function(val)),
            Err(TryError::IsErr(err)) => TryErr(err),
            Err(TryError::IsNotValue(_)) => TryNotValue(()),
        }
    }
}
