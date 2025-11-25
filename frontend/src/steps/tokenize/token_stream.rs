use crate::steps::{tokenize::tokenizer::Lexer};
use models::{error::{SoulError, SoulResult, Span}, symbool_kind::SymboolKind};

#[derive(Debug, Clone)]
pub struct TokenStream<'a> {
    lexer: Lexer<'a>,
    current: Token,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    Int(i64),
    Uint(u64),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    EndFile,
    EndLine,
    Ident(String),
    Unknown(char),
    Number(Number),
    CharLiteral(char),
    Symbool(SymboolKind),
    StringLiteral(String),
}

#[derive(Debug, Clone)]
pub struct TokenStreamPosition<'a>(TokenStream<'a>);

impl<'a> TokenStream<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self {
        Self{
            lexer, 
            current: Token::new(TokenKind::EndLine, Span::default()),
        }
    }

    pub fn initialize(&mut self) -> SoulResult<()> {
        self.advance()
    }

    pub fn current_position(&self) -> TokenStreamPosition<'a> {
        TokenStreamPosition(self.clone())
    }

    pub fn set_position(&mut self, position: TokenStreamPosition<'a>) {
        *self = position.0
    }

    pub fn current(&self) -> &Token {
        &self.current
    }

    pub fn advance(&mut self) -> SoulResult<()> {
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    pub fn consume_advance(&mut self) -> Result<Token, (Token, SoulError)> {
        use std::mem::swap;
        
        let mut consume_token = Token::new(TokenKind::EndLine, Span::default());
        swap(&mut self.current, &mut consume_token);
        
        if let Err(err) = self.advance() {
            Err((consume_token, err))
        }
        else {
            Ok(consume_token)
        }
    }

    pub fn to_vec(mut self) -> SoulResult<Vec<Token>> {
        use std::mem::swap;

        let mut token = Token::new(TokenKind::EndFile, Span::default()); 
        swap(&mut self.current, &mut token);
        let mut tokens = vec![token];

        loop {

            self.advance()?;
            let mut token = Token::new(TokenKind::EndFile, Span::default()); 
            swap(&mut self.current, &mut token);
            let is_end = token.is_end_of_file();
            tokens.push(token);
            if is_end {
                break
            }
        }

        Ok(tokens)
    }
}

impl TokenKind {

    pub fn display(&self) -> String {

        match self {
            TokenKind::Ident(ident) => ident.clone(),
            TokenKind::Unknown(char) => format!("{char}"),
            TokenKind::EndFile => format!("<end of file>"),
            TokenKind::EndLine => format!("<end of line>"),
            TokenKind::CharLiteral(char) => format!("'{char}'"),
            TokenKind::Number(number) => format!("{:?}", number),
            TokenKind::StringLiteral(str) => format!("\"{str}\""),
            TokenKind::Symbool(symbool_kind) => symbool_kind.as_str().to_string(),
        }
    }
}

impl Token {
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self {
            kind,
            span,
        }
    }

    pub const fn is_end_of_file(&self) -> bool {
        matches!(self.kind, TokenKind::EndFile)
    }
}