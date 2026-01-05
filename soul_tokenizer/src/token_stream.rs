use soul_utils::{error::{SoulError, SoulResult}, span::Span};

use crate::{lexer::Lexer, token::{Token, TokenKind}};

/// This struct provides methods for token stream navigation, consumption, and
/// conversion to a complete token vector. It supports save/restore positions
/// and peeking at upcoming tokens.
#[derive(Debug, Clone)]
pub struct TokenStream<'a> {
    lexer: Lexer<'a>,
    current: Token,
}

/// Position snapshot of a TokenStream for save/restore functionality.
#[derive(Debug, Clone)]
pub struct TokenStreamPosition<'a>(TokenStream<'a>);


impl<'a> TokenStream<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            lexer: Lexer::new(source),
            current: Token::new(TokenKind::EndLine, Span::default()),
        }
    }

    /// initializes tokenstream plz call this before using tokenstream
    pub fn initialize(&mut self) -> SoulResult<()> {
        self.advance()
    }

    /// Captures the current stream position for later restoration
    pub fn current_position(&self) -> TokenStreamPosition<'a> {
        TokenStreamPosition(self.clone())
    }

    pub fn current_token_index(&self) -> usize {
        self.lexer.current_token_index()
    }

    /// Restores the stream to a previously saved position.
    pub fn set_position(&mut self, position: TokenStreamPosition<'a>) {
        *self = position.0;
    }

    /// Returns a reference to the current token.
    pub fn current(&self) -> &Token {
        &self.current
    }

    /// Peeks at the next token without advancing the stream position.
    pub fn peek(&self) -> SoulResult<Token> {
        let mut lexer = self.lexer.clone();
        lexer.next_token()
    }

    /// Advances the stream to the next token, updating current token.
    pub fn advance(&mut self) -> SoulResult<()> {
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    /// Consumes and returns the current token, then advances.
    ///
    /// # Returns
    /// - `(Token, None)` no lexer error returns token
    /// - `(Token, Some(SoulError))` returns lexer error and token
    pub fn consume_advance(&mut self) -> (Token, Option<SoulError>) {
        use std::mem::swap;

        let mut consume_token = Token::new(TokenKind::EndLine, Span::default());
        swap(&mut self.current, &mut consume_token);

        if let Err(err) = self.advance() {
            (consume_token, Some(err))
        } else {
            (consume_token, None)
        }
    }
}

impl<'a> Iterator for TokenStream<'a> {
    type Item = SoulResult<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.kind == TokenKind::EndFile {
            return None;
        }

        let (token, error) = self.consume_advance();
        match error {
            Some(err) => Some(Err(err)),
            None => Some(Ok(token)),
        }
    }
}
