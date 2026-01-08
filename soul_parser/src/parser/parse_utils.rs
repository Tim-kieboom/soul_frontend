use soul_tokenizer::{Token, TokenKind, TokenStreamPosition};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind, SoulResult},
    sementic_level::SementicFault,
    span::Span,
    symbool_kind::SymbolKind,
};

use crate::parser::Parser;

pub const MUT_REF: TokenKind = TokenKind::Symbol(SymbolKind::And);
pub const COMMA: TokenKind = TokenKind::Symbol(SymbolKind::Comma);
pub const ARRAY: TokenKind = TokenKind::Symbol(SymbolKind::Array);
pub const COLON: TokenKind = TokenKind::Symbol(SymbolKind::Colon);
pub const ASSIGN: TokenKind = TokenKind::Symbol(SymbolKind::Assign);
pub const POINTER: TokenKind = TokenKind::Symbol(SymbolKind::Star);
pub const OPTIONAL: TokenKind = TokenKind::Symbol(SymbolKind::Question);
pub const CONST_REF: TokenKind = TokenKind::Symbol(SymbolKind::ConstRef);
pub const CURLY_OPEN: TokenKind = TokenKind::Symbol(SymbolKind::CurlyOpen);
pub const ROUND_OPEN: TokenKind = TokenKind::Symbol(SymbolKind::RoundOpen);
pub const ARROW_LEFT: TokenKind = TokenKind::Symbol(SymbolKind::LeftArray);
pub const SEMI_COLON: TokenKind = TokenKind::Symbol(SymbolKind::SemiColon);
pub const INCREMENT: TokenKind = TokenKind::Symbol(SymbolKind::DoublePlus);
pub const DECREMENT: TokenKind = TokenKind::Symbol(SymbolKind::DoubleMinus);
pub const SQUARE_OPEN: TokenKind = TokenKind::Symbol(SymbolKind::SquareOpen);
pub const CURLY_CLOSE: TokenKind = TokenKind::Symbol(SymbolKind::CurlyClose);
pub const ROUND_CLOSE: TokenKind = TokenKind::Symbol(SymbolKind::RoundClose);
pub const SQUARE_CLOSE: TokenKind = TokenKind::Symbol(SymbolKind::SquareClose);
pub const COLON_ASSIGN: TokenKind = TokenKind::Symbol(SymbolKind::ColonAssign);
pub const STAMENT_END_TOKENS: &[TokenKind] = &[
    CURLY_CLOSE,
    TokenKind::EndFile,
    TokenKind::EndLine,
    TokenKind::Symbol(SymbolKind::SemiColon),
];

impl<'a> Parser<'a> {
    /// Returns reference to current token.
    pub(super) fn token(&self) -> &Token {
        self.tokens.current()
    }

    /// Combines start span with current token span.
    pub(super) fn span_combine(&self, start_span: Span) -> Span {
        start_span.combine(self.token().span)
    }

    /// Checks if current token matches exact kind.
    pub(super) fn current_is(&self, kind: &TokenKind) -> bool {
        &self.token().kind == kind
    }

    /// Checks if current token matches any of given kinds.
    pub(super) fn current_is_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.token().kind)
    }

    /// Checks if current token is specific identifier.
    pub(super) fn current_is_ident(&self, expected: &str) -> bool {
        match &self.token().kind {
            TokenKind::Ident(ident) => expected == ident,
            _ => false,
        }
    }

    /// Records parse error.
    pub(super) fn add_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }

    /// Saves current token stream position for backtracking.
    pub(crate) fn current_position(&self) -> TokenStreamPosition<'a> {
        self.tokens.current_position()
    }

    /// Restores token stream to saved position.
    pub(crate) fn go_to(&mut self, position: TokenStreamPosition<'a>) {
        self.tokens.set_position(position);

        #[cfg(debug_assertions)]
        {
            self.debug.current = self.token().clone();
            self.debug.current_index = self.tokens.current_token_index();
        }
    }

    /// Skips all [`TokenKind::EndLine`] tokens.
    pub(super) fn skip_end_lines(&mut self) {
        self.skip_till(&[TokenKind::EndLine])
    }

    /// Skips tokens matching any of given kinds.
    pub(crate) fn skip_till(&mut self, kinds: &[TokenKind]) {
        while self.current_is_any(kinds) && !self.current_is(&TokenKind::EndFile) {
            self.bump();
        }
    }

    /// Advances to next token.
    pub(super) fn bump(&mut self) {
        if let Err(err) = self.tokens.advance() {
            self.add_error(err);
        }

        #[cfg(debug_assertions)]
        {
            self.debug.current = self.token().clone();
            self.debug.current_index += 1;
        }
    }

    /// Peeks at next token without consuming.
    pub(super) fn peek(&mut self) -> Token {
        match self.tokens.peek() {
            Ok(val) => val,
            Err(err) => {
                self.add_error(err);
                self.token().clone()
            }
        }
    }

    /// Consumes current token and advances.
    pub(super) fn bump_consume(&mut self) -> Token {
        let token = match self.tokens.consume_advance() {
            (token, None) => token,
            (token, Some(err)) => {
                self.add_error(err);
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

    pub(crate) fn try_bump_consume_ident(&mut self) -> SoulResult<Ident> {
        if !matches!(self.token().kind, TokenKind::Ident(_)) {
            return Err(SoulError::new(
                format!("expected ident got {}", self.token().kind.display(),),
                SoulErrorKind::InvalidTokenKind,
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
    
    pub(crate) fn try_token_as_ident_str(&mut self) -> SoulResult<&str> {

        let token = &self.token();
        match &token.kind {
            TokenKind::Ident(val) => Ok(val),
            _ => Err(SoulError::new(
                format!("expected ident got {}", self.token().kind.display(),),
                SoulErrorKind::InvalidTokenKind,
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

    /// Expects any token from given kinds.
    pub(crate) fn expect_any(&mut self, kinds: &[TokenKind]) -> SoulResult<()> {
        if self.current_is_any(kinds) {
            self.bump();
            Ok(())
        } else {
            Err(self.get_expect_any_error(kinds))
        }
    }

    /// Expects specific identifier name.
    pub(crate) fn expect_ident(&mut self, expected: &str) -> SoulResult<()> {
        if let TokenKind::Ident(ident) = &self.token().kind {
            if ident == expected {
                self.bump();
                Ok(())
            } else {
                Err(SoulError::new(
                    format!("expected: '{}' but found: '{}'", expected, ident),
                    SoulErrorKind::InvalidIdent,
                    Some(self.token().span),
                ))
            }
        } else {
            Err(self.get_expect_error(&TokenKind::Ident(expected.to_string())))
        }
    }

    /// Creates error for expected single token kind.
    pub(super) fn get_expect_error(&self, expected: &TokenKind) -> SoulError {
        let message = format!(
            "expected: '{}' but found: '{}'",
            expected.display(),
            self.token().kind.display()
        );
        SoulError::new(
            message,
            SoulErrorKind::InvalidTokenKind,
            Some(self.token().span),
        )
    }

    /// Creates error for expected token from set.
    pub(super) fn get_expect_any_error(&self, expected: &[TokenKind]) -> SoulError {
        let mut tokens_string = String::new();
        for token in expected {
            token.write_display(&mut tokens_string)
        }

        let message = format!(
            "expected on of: ['{}'] but found: '{}'",
            tokens_string,
            self.token().kind.display()
        );
        SoulError::new(
            message,
            SoulErrorKind::InvalidTokenKind,
            Some(self.token().span),
        )
    }
}
