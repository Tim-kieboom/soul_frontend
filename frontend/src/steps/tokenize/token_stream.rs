use crate::steps::tokenize::tokenizer::Lexer;
use models::{
    error::{SoulError, SoulResult, Span},
    soul_names::InternalPrimitiveTypes,
    symbool_kind::SymboolKind,
};

/// This struct provides methods for token stream navigation, consumption, and
/// conversion to a complete token vector. It supports save/restore positions
/// and peeking at upcoming tokens.
#[derive(Debug, Clone)]
pub struct TokenStream<'a> {
    lexer: Lexer<'a>,
    current: Token,
}

/// A single token containing its kind and source span information.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind/variant of this token.
    pub kind: TokenKind,
    /// The source code span for this token.
    pub span: Span,
}

/// Position snapshot of a TokenStream for save/restore functionality.
#[derive(Debug, Clone)]
pub struct TokenStreamPosition<'a>(TokenStream<'a>);

/// Represents different numeric literal kinds parsed from source code.
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    /// Signed integer literal, e.g. `-42`.
    Int(i64),
    /// Unsigned integer literal, e.g. `42u32`.
    Uint(u64),
    /// Floating point literal, e.g. `3.14`.
    Float(f64),
}

/// Enumerates all possible token kinds recognized by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Marks the end of the input file.
    EndFile,
    /// Marks the end of a source line.
    EndLine,
    /// Identifier name, e.g. variable or function name.
    Ident(String),
    /// Single unrecognized character.
    Unknown(char),
    /// Numeric literal of any supported kind.
    Number(Number),
    /// Character literal, e.g. `'a'`.
    CharLiteral(char),
    /// Symbol/operator token with associated kind.
    Symbool(SymboolKind),
    /// String literal, e.g. `"hello"`.
    StringLiteral(String),
}

impl<'a> TokenStream<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer,
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

    /// Consumes all remaining tokens into a Vec, including the current token.
    pub fn to_vec(&self) -> SoulResult<Vec<Token>> {
        use std::mem::swap;

        let mut this = self.clone();
        let mut token = Token::new(TokenKind::EndFile, Span::default());
        swap(&mut this.current, &mut token);
        let mut tokens = vec![token];

        loop {
            this.advance()?;
            let mut token = Token::new(TokenKind::EndFile, Span::default());
            swap(&mut this.current, &mut token);
            let is_end = token.is_end_of_file();
            tokens.push(token);
            if is_end {
                break;
            }
        }

        Ok(tokens)
    }
}

impl TokenKind {
    /// Returns a display string representation of the token kind.
    pub fn display(&self) -> String {
        match self {
            TokenKind::Ident(ident) => ident.clone(),
            TokenKind::Unknown(char) => format!("{char}"),
            TokenKind::EndFile => "<end of file>".to_string(),
            TokenKind::EndLine => "<end of line>".to_string(),
            TokenKind::CharLiteral(char) => format!("'{char}'"),
            TokenKind::Number(number) => number.display(),
            TokenKind::StringLiteral(str) => format!("\"{str}\""),
            TokenKind::Symbool(symbool_kind) => symbool_kind.as_str().to_string(),
        }
    }

    /// Attempts to extract the string value if this is an Ident token.
    pub fn try_as_ident(&self) -> Option<&str> {
        match self {
            TokenKind::Ident(val) => Some(val),
            _ => None,
        }
    }
}

impl Token {
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Checks if this token marks the end of file.
    pub const fn is_end_of_file(&self) -> bool {
        matches!(self.kind, TokenKind::EndFile)
    }
}

impl Number {
    /// Number display formatting with type annotation.
    pub fn display(&self) -> String {
        match self {
            Number::Int(num) => format!("{num}: {}", InternalPrimitiveTypes::UntypedInt.as_str()),
            Number::Uint(num) => format!("{num}: {}", InternalPrimitiveTypes::UntypedUint.as_str()),
            Number::Float(num) => {
                format!("{num}: {}", InternalPrimitiveTypes::UntypedFloat.as_str())
            }
        }
    }
}
