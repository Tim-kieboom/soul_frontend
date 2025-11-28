use crate::steps::{parse::{Request, Response, parse_statement::SEMI_COLON}, tokenize::token_stream::{Token, TokenKind, TokenStream, TokenStreamPosition}};
use models::{abstract_syntax_tree::{AbstractSyntaxTree, block::Block, statment::Ident}, error::{SoulError, SoulErrorKind, SoulResult, Span}, scope::{scope::{ScopeId, ValueSymbol}, scope_builder::ScopeBuilder}, soul_names::TypeModifier};

pub fn parse<'a>(request: Request) -> Response {
    Response {
        parser: Parser::new(request.token_stream)
    }
}

#[derive(Debug)]
pub struct Parser<'a> {
    #[cfg(debug_assertions)]
    current_index: usize,
    #[cfg(debug_assertions)]
    current: TokenKind,

    tokens: TokenStream<'a>,
    errors: Vec<SoulError>,
    scopes: ScopeBuilder,
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
            scopes: ScopeBuilder::new(),
        }
    }

    pub fn comsume_errors(self) -> Vec<SoulError> {
        self.errors
    }

    pub fn parse_tokens(&mut self) -> AbstractSyntaxTree {
        if let Err(err) = self.tokens.initialize() {
            self.add_error(err);
        }
        
        #[cfg(debug_assertions)] {
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

            self.skip(&[SEMI_COLON, TokenKind::EndLine]);
        }

        AbstractSyntaxTree{ 
            root: Block{
                statments,
                modifier: TypeModifier::Mut,
                scope_id: self.scopes.current_scope().id,
            } 
        }
    }
    
    pub(crate) fn current_position(&self) -> TokenStreamPosition<'a> {
        self.tokens.current_position()
    }

    pub(crate) fn go_to(&mut self, position: TokenStreamPosition<'a>) {
        self.tokens.set_position(position);
    }

    pub(crate) fn push_scope(&mut self) -> ScopeId {
        self.scopes.push_scope()
    }

    pub(crate) fn add_scope_value(&mut self, name: Ident, symbol: ValueSymbol) {
        self.scopes.insert_value(name, symbol);
    }

    pub(crate) fn add_error(&mut self, err: SoulError) {
        
        #[cfg(debug_assertions)] {
            println!("{}", err.to_message());
            panic!();
        }
        
        #[cfg(not(debug_assertions))]
        self.errors.push(err);
    }

    pub(crate) fn peek(&mut self) -> Token {
        match self.tokens.peek() {
            Ok(val) => val,
            Err(err) => {
                self.add_error(err);
                self.token().clone()
            }
        
        }
    }

    pub(crate) fn current_is(&self, kind: &TokenKind) -> bool {
        &self.token().kind == kind
    }

    pub(crate) fn current_is_ident(&self, expected: &str) -> bool {
        match &self.token().kind {
            TokenKind::Ident(ident) => expected == ident,
            _ => false,
        }
    }

    pub(crate) fn current_is_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.token().kind)
    }

    pub(crate) fn bump(&mut self) {

        if let Err(err) = self.tokens.advance() {
            self.add_error(err);
        }

        #[cfg(debug_assertions)] {
            self.current = self.token().kind.clone();
            self.current_index += 1;
        }
    }

    pub(crate) fn bump_consume(&mut self) -> Token {

        let token = match self.tokens.consume_advance() {
            Ok(token) => token,
            Err((token, err)) => {
                self.add_error(err);
                token
            },
        };

        #[cfg(debug_assertions)] {
            self.current = self.token().kind.clone();
            self.current_index += 1;
        }

        token
    }

    pub(crate) fn token(&self) -> &Token {
        self.tokens.current()
    }
    
    pub(crate) fn expect(&mut self, kind: &TokenKind) -> SoulResult<()> {

        if self.current_is(&kind) {
            self.bump();
            Ok(())
        }
        else {
            Err(self.get_error_expected(kind))
        }
    }

    pub(crate) fn expect_any(&mut self, kinds: &[TokenKind]) -> SoulResult<()> {

        if self.current_is_any(kinds) {
            self.bump();
            Ok(())
        }
        else {
            Err(self.get_error_expected_any(kinds))
        }
    }

    pub(crate) fn expect_ident(&mut self, expected: &str) -> SoulResult<()> {

        if let TokenKind::Ident(ident) = &self.token().kind {
            if ident == expected {
                self.bump();
                Ok(())
            }
            else {
                Err(
                    SoulError::new(
                        format!("expected: '{}' but found: '{}'", expected, ident),
                        SoulErrorKind::InvalidAssignType,
                        Some(self.token().span),
                    )
                )
            }
        }
        else {
            Err(self.get_error_expected(&TokenKind::Ident(expected.to_string())))
        }
    }

    pub(crate) fn new_span(&self, start_span: Span) -> Span {
        start_span.combine(self.token().span)
    }

    pub(crate) fn skip_end_lines(&mut self) {
        self.skip(&[TokenKind::EndLine])
    }

    pub(crate) fn skip(&mut self, kinds: &[TokenKind]) {
        while self.current_is_any(kinds) && !self.current_is(&TokenKind::EndFile) {
            self.bump();
        }
    }

    pub(crate) fn get_error_expected(&self, expected: &TokenKind) -> SoulError {
        let message = format!("expected: '{:?}' but found: '{:?}'", expected, self.token().kind);
        SoulError::new(
            message, 
            SoulErrorKind::InvalidTokenKind,
            Some(self.token().span),
        )
    }

    pub(crate) fn get_error_expected_any(&self, expected: &[TokenKind]) -> SoulError {
        let message = format!("expected on of: [{:?}] but found: '{:?}'", expected, self.token().kind);
        SoulError::new(
            message, 
            SoulErrorKind::InvalidTokenKind,
            Some(self.token().span),
        )
    }
}