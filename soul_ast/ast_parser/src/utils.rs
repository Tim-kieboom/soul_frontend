use std::path::Path;

use ast_model::NodeId;
use soul_tokenizer::{
    TokenStreamPosition,
    model::{Token, TokenKind, keyword::KeyWord},
};
use soul_utils::{
    Ident, TypeModifier, error::SoulResult, fault::Fault, soul_names::Symbol, span::Span,
};

use crate::parser::Parser;

pub const AS_STR: &str = KeyWord::As.as_str();

pub const IF: TokenKind = TokenKind::Keyword(KeyWord::If);
pub const NOT: TokenKind = TokenKind::Symbol(Symbol::Not);
pub const DOT: TokenKind = TokenKind::Symbol(Symbol::Dot);
pub const REF: TokenKind = TokenKind::Symbol(Symbol::And);
pub const FOR: TokenKind = TokenKind::Keyword(KeyWord::For);
pub const STAR: TokenKind = TokenKind::Symbol(Symbol::Star);
pub const PUB: TokenKind = TokenKind::Keyword(KeyWord::Pub);
pub const MUT: TokenKind = TokenKind::Keyword(KeyWord::Mut);
pub const ELSE: TokenKind = TokenKind::Keyword(KeyWord::Else);
pub const IMPL: TokenKind = TokenKind::Keyword(KeyWord::Impl);
pub const NULL: TokenKind = TokenKind::Keyword(KeyWord::Null);
pub const COMMA: TokenKind = TokenKind::Symbol(Symbol::Comma);
pub const ARRAY: TokenKind = TokenKind::Symbol(Symbol::Array);
pub const COLON: TokenKind = TokenKind::Symbol(Symbol::Colon);
pub const PASS: TokenKind = TokenKind::Keyword(KeyWord::Pass);
pub const COPY: TokenKind = TokenKind::Keyword(KeyWord::Copy);
pub const POINTER: TokenKind = TokenKind::Symbol(Symbol::Star);
pub const MATCH: TokenKind = TokenKind::Keyword(KeyWord::Match);
pub const ASSIGN: TokenKind = TokenKind::Symbol(Symbol::Assign);
pub const CONST: TokenKind = TokenKind::Keyword(KeyWord::Const);
pub const IN: TokenKind = TokenKind::Keyword(KeyWord::InForLoop);
pub const STRUCT: TokenKind = TokenKind::Keyword(KeyWord::Struct);
pub const IMPORT: TokenKind = TokenKind::Keyword(KeyWord::Import);
pub const SIZEOF: TokenKind = TokenKind::Keyword(KeyWord::Sizeof);
pub const LITERAL: TokenKind = TokenKind::Keyword(KeyWord::Literal);
pub const OPTIONAL: TokenKind = TokenKind::Symbol(Symbol::Question);
pub const CURLY_OPEN: TokenKind = TokenKind::Symbol(Symbol::CurlyOpen);
pub const ROUND_OPEN: TokenKind = TokenKind::Symbol(Symbol::RoundOpen);
pub const ARROW_LEFT: TokenKind = TokenKind::Symbol(Symbol::LeftArray);
pub const SEMI_COLON: TokenKind = TokenKind::Symbol(Symbol::SemiColon);
pub const ARROW_RIGHT: TokenKind = TokenKind::Symbol(Symbol::RightArray);
pub const SQUARE_OPEN: TokenKind = TokenKind::Symbol(Symbol::SquareOpen);
pub const CURLY_CLOSE: TokenKind = TokenKind::Symbol(Symbol::CurlyClose);
pub const ROUND_CLOSE: TokenKind = TokenKind::Symbol(Symbol::RoundClose);
pub const LAMBDA_ARROW: TokenKind = TokenKind::Symbol(Symbol::LambdaArrow);
pub const SQUARE_CLOSE: TokenKind = TokenKind::Symbol(Symbol::SquareClose);
pub const COLON_ASSIGN: TokenKind = TokenKind::Symbol(Symbol::ColonAssign);
pub const DOUBLE_QUESTION: TokenKind = TokenKind::Symbol(Symbol::DoubleQuestion);
pub const DOUBLE_DOT: TokenKind = TokenKind::Symbol(Symbol::DoubleDot);
pub const STAMENT_END_TOKENS: &[TokenKind] = &[
    CURLY_CLOSE,
    TokenKind::EndFile,
    TokenKind::EndLine,
    TokenKind::Symbol(Symbol::SemiColon),
];

/// Tokens that end a statement for `skip_till` during error recovery.
/// Excludes `CURLY_CLOSE` so that error recovery never consumes a closing brace.
pub const STAMENT_SKIP_TOKENS: &[TokenKind] = &[
    TokenKind::EndFile,
    TokenKind::EndLine,
    TokenKind::Symbol(Symbol::SemiColon),
];

impl<'a, 'f> Parser<'a, 'f> {
    /// Returns reference to current token.
    pub(super) fn token(&self) -> &Token {
        self.tokens.current()
    }

    /// Skips all [`TokenKind::EndLine`] tokens.
    pub(super) fn skip_end_lines(&mut self) {
        self.skip_while_any(&[TokenKind::EndLine])
    }

    /// Skips tokens matching any of given kinds.
    pub(crate) fn skip_while_any(&mut self, kinds: &[TokenKind]) {
        while self.current_is_any(kinds) && !self.current_is(&TokenKind::EndFile) {
            self.bump();
        }
    }

    /// Skips tokens matching any of given kinds.
    pub(crate) fn skip_till(&mut self, kinds: &[TokenKind]) {
        while !self.current_is_any(kinds) && !self.current_is(&TokenKind::EndFile) {
            self.bump();
        }
    }

    /// Advances to next token.
    pub(super) fn bump(&mut self) {
        if let Err(err) = self.tokens.advance() {
            self.log_fault(err);
        }

        #[cfg(debug_assertions)]
        {
            self.debug.current = self.token().clone();
            self.debug.current_index += 1;
        }
    }

    /// Peeks at next token without consuming.
    pub(super) fn try_peek(&self) -> SoulResult<Token> {
        self.tokens.peek()
    }

    /// Peeks at next token without consuming.
    pub(super) fn peek(&mut self) -> Token {
        match self.tokens.peek() {
            Ok(val) => val,
            Err(err) => {
                self.log_fault(err);
                self.token().clone()
            }
        }
    }

    /// Consumes current token and advances.
    pub(super) fn bump_consume(&mut self) -> Token {
        let token = match self.tokens.consume_advance() {
            (token, None) => token,
            (token, Some(err)) => {
                self.log_fault(err);
                token
            }
        };

        #[cfg(debug_assertions)]
        {
            self.debug.current = self.token().clone();
            self.debug.current_index += 1;
        }

        token
    }

    /// Restores token stream to saved position.
    pub(crate) fn goto(&mut self, position: TokenStreamPosition<'a>) {
        self.tokens.set_position(position);

        #[cfg(debug_assertions)]
        {
            self.debug.current = self.token().clone();
            self.debug.current_index = self.tokens.index();
        }
    }

    pub(crate) fn try_token_as_ident_str(&mut self) -> SoulResult<&str> {
        let token = &self.token();
        match &token.kind {
            TokenKind::Ident(val) => Ok(val),
            _ => Err(Fault::error(
                format!("expected ident got `{}`", self.token().kind.display()),
                Some(self.token().span),
            )),
        }
    }

    /// Expects exact token kind, errors if mismatch.
    pub(crate) fn expect(&mut self, kind: &TokenKind) -> SoulResult<()> {
        if self.current_is(kind) {
            self.bump();
            Ok(())
        } else {
            Err(self.get_expect_error(kind))
        }
    }

    /// Expects exact token kind, errors if mismatch.
    pub(crate) fn expect_ident(&mut self, ident: &str) -> SoulResult<()> {
        if self.current_is_ident(ident) {
            self.bump();
            Ok(())
        } else {
            Err(self.get_expect_ident_error(ident))
        }
    }

    /// Creates error for expected single token kind.
    pub(super) fn get_expect_ident_error(&self, string: &str) -> Fault {
        let message = format!(
            "expected: `{string}` but found: `{}`",
            self.token().kind.display()
        );
        Fault::error(message, Some(self.token().span))
    }

    /// Creates error for expected single token kind.
    pub(super) fn get_expect_error(&self, expected: &TokenKind) -> Fault {
        let message = format!(
            "expected: `{}` but found: `{}`",
            expected.display(),
            self.token().kind.display()
        );
        Fault::error(message, Some(self.token().span))
    }

    /// Creates error for expected token from set.
    pub(super) fn get_expect_any_error(&self, expected: &[TokenKind]) -> Fault {
        let mut tokens_string = String::new();

        let last_index = expected.len().saturating_sub(1);
        for (i, token) in expected.iter().enumerate() {
            token
                .write_display(&mut tokens_string)
                .expect("no fmt error");
            if i != last_index {
                tokens_string.push_str("`, `");
            }
        }

        let message = format!(
            "expected on of: [`{}`] but found: `{}`",
            tokens_string,
            self.token().kind.display()
        );

        Fault::error(message, Some(self.token().span))
    }

    pub(crate) fn try_bump_type_modiffier(&mut self) -> Option<TypeModifier> {
        Some(match self.token().kind {
            TokenKind::Keyword(KeyWord::Mut) => {
                self.bump();
                TypeModifier::Mut
            }
            TokenKind::Keyword(KeyWord::Const) => {
                self.bump();
                TypeModifier::Const
            }
            TokenKind::Keyword(KeyWord::Literal) => {
                self.bump();
                TypeModifier::Literal
            }
            _ => return None,
        })
    }

    pub(crate) fn try_bump_consume_ident(&mut self) -> SoulResult<Ident> {
        if !matches!(self.token().kind, TokenKind::Ident(_)) {
            return Err(Fault::error(
                format!("expected ident got {}", self.token().kind.display()),
                Some(self.token().span),
            ));
        }

        let token = self.bump_consume();
        let text = match token.kind {
            TokenKind::Ident(val) => val,
            _ => unreachable!(),
        };

        Ok(Ident::new(text, token.span))
    }

    pub(crate) fn current_is_keyword(&self, expected: KeyWord) -> bool {
        match &self.token().kind {
            TokenKind::Ident(ident) => KeyWord::from_str(ident.as_str()) == Some(expected),
            TokenKind::Keyword(keyword) => *keyword == expected,
            _ => false,
        }
    }

    pub fn current_path(&self) -> &Path {
        &self.source_path
    }

    /// Checks if current token matches any of given kinds.
    pub(super) fn current_is_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.token().kind)
    }

    /// Checks if current token matches exact kind.
    pub(super) fn current_is(&self, kind: &TokenKind) -> bool {
        &self.token().kind == kind
    }

    /// Checks if next token matches exact kind.
    pub(super) fn peek_is(&self, kind: &TokenKind) -> bool {
        match self.try_peek() {
            Ok(token) => &token.kind == kind,
            Err(_) => return false,
        }
    }

    pub(super) fn peek_is_multiple(&self, kinds: &[TokenKind]) -> bool {
        let mut lexer = self.tokens.lexer().clone();
        for kind in kinds {
            let Ok(token) = lexer.next() else {
                return false;
            };

            if &token.kind != kind {
                return false;
            }
        }

        true
    }

    /// Checks if current token matches exact kind (handles both `Ident` and `Keyword` tokens).
    pub(super) fn current_is_ident(&self, string: &str) -> bool {
        match &self.token().kind {
            TokenKind::Ident(ident) => ident == string,
            _ => false,
        }
    }

    /// Combines start span with current token span.
    pub(super) fn span_combine(&self, start_span: Span) -> Span {
        start_span.combine(self.token().span)
    }

    /// checked if node is end of line and ends with a semicolon
    pub(crate) fn ends_semicolon(&mut self) -> bool {
        self.current_is(&SEMI_COLON) && self.peek().kind == TokenKind::EndLine
    }

    pub(crate) fn alloc_node(&mut self) -> NodeId {
        self.store.alloc_node()
    }

    pub(crate) fn log_fault(&mut self, fault: Fault) {
        self.context.faults.push(fault);
    }

    pub(super) fn log_error(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.context.faults.push_error(message, span);
    }
}
