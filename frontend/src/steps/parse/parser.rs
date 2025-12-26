use crate::steps::{
    parse::{Request, Response, SEMI_COLON},
    tokenize::token_stream::{Token, TokenKind, TokenStream, TokenStreamPosition},
};
use itertools::Itertools;
use models::{
    abstract_syntax_tree::{AbstractSyntaxTree, block::Block},
    error::{SoulError, SoulErrorKind, SoulResult, Span},
    soul_names::TypeModifier,
};

/// Main entry point for parsing token stream into Abstract Syntax Tree.
///
/// Creates a parser instance that processes the entire token stream into
/// statements forming the root block of the AST.
pub fn parse(request: Request) -> Response {
    Response {
        parser: Parser::new(request.token_stream),
    }
}

/// Recursive descent parser that builds AST from token stream.
///
/// Manages token consumption, error recovery, scope tracking, and debug
/// information (debug builds only). Supports position save/restore for
/// backtracking during parsing.
#[derive(Debug)]
pub struct Parser<'a> {
    #[cfg(debug_assertions)]
    current_index: usize,
    #[cfg(debug_assertions)]
    current: TokenKind,

    tokens: TokenStream<'a>,
    errors: Vec<SoulError>,
}
impl<'a> Parser<'a> {
    #[cfg(not(debug_assertions))]
    pub fn new(tokens: TokenStream<'a>) -> Self {
        Self {
            tokens,
            errors: vec![],
            scopes: ScopeBuilder::new(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn new(tokens: TokenStream<'a>) -> Self {
        Self {
            current: TokenKind::EndFile,
            current_index: 0,

            tokens,
            errors: vec![],
        }
    }

    /// Parses all statements from token stream into complete AST.
    pub fn parse_tokens(&mut self) -> AbstractSyntaxTree {
        if let Err(err) = self.tokens.initialize() {
            self.add_error(err);
        }

        #[cfg(debug_assertions)]
        {
            self.current = self.token().kind.clone();
        }

        self.skip_end_lines();
        let mut statments = vec![];

        while !self.current_is(&TokenKind::EndFile) {
            match self.parse_statement() {
                Ok(statment) => statments.push(statment),
                Err(err) => {
                    self.add_error(err);
                    self.skip_over_statement();
                }
            }

            self.skip_till(&[SEMI_COLON, TokenKind::EndLine]);
        }

        AbstractSyntaxTree {
            root: Block {
                statments,
                modifier: TypeModifier::Mut,
                scope_id: 0,
            },
        }
    }

    /// Returns all accumulated parse errors.
    ///
    /// Consumes Parser.
    pub fn comsume_errors(self) -> Vec<SoulError> {
        self.errors
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
            self.current = self.token().kind.clone();
            self.current_index = self.tokens.current_token_index();
        }
    }

    /// Records parse error.
    pub(crate) fn add_error(&mut self, err: SoulError) {
        self.errors.push(err);
    }

    /// Peeks at next token without consuming.
    pub(crate) fn peek(&mut self) -> Token {
        match self.tokens.peek() {
            Ok(val) => val,
            Err(err) => {
                self.add_error(err);
                self.token().clone()
            }
        }
    }

    /// Checks if current token matches exact kind.
    pub(crate) fn current_is(&self, kind: &TokenKind) -> bool {
        &self.token().kind == kind
    }

    /// Checks if current token is specific identifier.
    pub(crate) fn current_is_ident(&self, expected: &str) -> bool {
        match &self.token().kind {
            TokenKind::Ident(ident) => expected == ident,
            _ => false,
        }
    }

    /// Checks if current token matches any of given kinds.
    pub(crate) fn current_is_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.token().kind)
    }

    /// Advances to next token.
    pub(crate) fn bump(&mut self) {
        if let Err(err) = self.tokens.advance() {
            self.add_error(err);
        }

        #[cfg(debug_assertions)]
        {
            self.current = self.token().kind.clone();
            self.current_index += 1;
        }
    }

    /// Consumes current token and advances.
    pub(crate) fn bump_consume(&mut self) -> Token {
        let token = match self.tokens.consume_advance() {
            (token, None) => token,
            (token, Some(err)) => {
                self.add_error(err);
                token
            }
        };

        #[cfg(debug_assertions)]
        {
            self.current = self.token().kind.clone();
            self.current_index += 1;
        }

        token
    }

    /// Returns reference to current token.
    pub(crate) fn token(&self) -> &Token {
        self.tokens.current()
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
                    SoulErrorKind::InvalidAssignType,
                    Some(self.token().span),
                ))
            }
        } else {
            Err(self.get_expect_error(&TokenKind::Ident(expected.to_string())))
        }
    }

    /// Combines start span with current token span.
    pub(crate) fn new_span(&self, start_span: Span) -> Span {
        start_span.combine(self.token().span)
    }

    /// Skips all [`TokenKind::EndLine`] tokens.
    pub(crate) fn skip_end_lines(&mut self) {
        self.skip_till(&[TokenKind::EndLine])
    }

    /// Skips tokens matching any of given kinds.
    pub(crate) fn skip_till(&mut self, kinds: &[TokenKind]) {
        while self.current_is_any(kinds) && !self.current_is(&TokenKind::EndFile) {
            self.bump();
        }
    }

    /// Creates error for expected single token kind.
    pub(crate) fn get_expect_error(&self, expected: &TokenKind) -> SoulError {
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
    pub(crate) fn get_expect_any_error(&self, expected: &[TokenKind]) -> SoulError {
        let message = format!(
            "expected on of: ['{}'] but found: '{}'",
            expected.iter().map(|token| token.display()).join("', '"),
            self.token().kind.display()
        );
        SoulError::new(
            message,
            SoulErrorKind::InvalidTokenKind,
            Some(self.token().span),
        )
    }
}
