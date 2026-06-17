use ast_model::{
    FunctionKind,
    block::{Block, BlockId},
    statements::{Statement, StatementId},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    TypeModifier,
    collections::try_result::{ResultTryErr, TryErr, TryError, TryNotValue, TryOk, TryResult},
    error::SoulResult,
    fault::Fault,
    soul_names::Symbol,
    span::Span,
};

use crate::{
    parser::Parser, utils::{
        ARROW_LEFT, COLON, COLON_ASSIGN, CURLY_CLOSE, CURLY_OPEN, DOT, ROUND_OPEN, SEMI_COLON,
        STAMENT_END_TOKENS, STAMENT_SKIP_TOKENS, STAR,
    }
};

mod assign;
mod from_keyword;
mod from_modfier;
mod import;
mod objects;
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

        let peek = self.peek();
        match &peek.kind {
            &ROUND_OPEN | &ARROW_LEFT => self.parse_any_function().try_err(),
            &COLON | &COLON_ASSIGN => self.parse_variable().try_err(),
            _ => self.parse_from_unknown_ident(start_span).try_err(),
        }
    }

    fn parse_from_unknown_ident(&mut self, start_span: Span) -> SoulResult<Statement> {
        
        if self.current_is(&DOT) {
            let ident = self.try_bump_consume_ident()?;
            if ident.as_str() != "This" {
                return Err(Fault::error(
                    format!("`{}` invalid", self.token().kind.display()),
                    Some(self.span_combine(start_span)),
                ));
            }

            let this = self.current.this_type.take();
            let result = match &this {
                Some(ty) => self.parse_function_contructor(ty, TypeModifier::Mut),
                None => Err(Fault::error(
                    "contructor function should habe methode type",
                    Some(self.span_combine(start_span)),
                )),
            };
            self.current.this_type = this;
            return result
                .map(|spanned| {
                    spanned.map(|func| self.store.insert_function(FunctionKind::Normal(func)))
                })
                .map(Statement::from_function);
        }
        
        self.parse_assign_or_expression(start_span)
    }
}
