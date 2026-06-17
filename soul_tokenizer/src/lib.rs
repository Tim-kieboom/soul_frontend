use crate::lexer::{Lexer};
use crate::model::{Token, TokenKind};
use soul_utils::error::SoulResult;
use soul_utils::fault::Fault;
use soul_utils::span::{ModuleId, Span};

pub(crate) mod lexer;
pub mod model;
pub(crate) mod str_iter;

#[cfg(test)]
mod tests;

/// Position snapshot of a TokenStream for save/restore functionality.
pub struct TokenStreamPosition<'a>(TokenStream<'a>);

#[derive(Debug, Clone)]
pub struct TokenStream<'a> {
    lexer: Lexer<'a>,
    current: Token,
    index: usize,
}

/// Converts source code into a token stream for parsing.
pub fn to_token_stream<'a>(source: &'a str, module: ModuleId) -> SoulResult<TokenStream<'a>> {
    TokenStream::new(source, module)
}

impl<'a> TokenStream<'a> {
    /// Creates a new token stream from the given source code.
    pub fn new(source: &'a str, module: ModuleId) -> SoulResult<Self> {
        let mut this = Self {
            index: 0,
            lexer: Lexer::new(source, module),
            current: Token::new(TokenKind::EndLine, Span::default(module)),
        };
        this.initialize()?;
        Ok(this)
    }

    pub fn index(&self) -> usize {
        self.index
    }

    fn initialize(&mut self) -> SoulResult<()> {
        self.advance()
    }

    /// Captures the current stream position for later restoration.
    pub fn current_position(&self) -> TokenStreamPosition<'a> {
        TokenStreamPosition(self.clone())
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
    pub fn peek(&mut self) -> SoulResult<Token> {
        self.lexer.clone().next()
    }

    /// Advances the stream to the next token, updating the current token.
    pub fn advance(&mut self) -> SoulResult<()> {
        self.current = self.lexer.next()?;
        self.index += 1;
        Ok(())
    }

    /// Consumes and returns the current token, then advances the stream.
    ///
    /// # Returns
    /// - `(Token, None)` - no lexer error, returns the token
    /// - `(Token, Some(Fault))` - lexer error, returns the token
    pub fn consume_advance(&mut self) -> (Token, Option<Fault>) {
        use std::mem::swap;

        let mut consume_token = Token::new(TokenKind::EndLine, Span::error());
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
