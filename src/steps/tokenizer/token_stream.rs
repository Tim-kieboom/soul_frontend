use crate::{error::{SoulResult, Span}, steps::{tokenizer::{symbool::SymboolKind, tokenize::Lexer}, utils::Number}};

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
pub enum TokenKind {
    Unknown(char),
    EndFile,
    EndLine,
    Ident(String),
    StringLiteral(String),
    CharLiteral(char),
    Number(Number),
    Symbool(SymboolKind),
}
impl<'a> TokenStream<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> SoulResult<Self> {
        let current = lexer.next()?;
        Ok(Self{lexer, current})
    }

    pub fn next(&mut self) -> SoulResult<Option<Token>> {
        let token = self.lexer.next()?;
        if token.is_end_of_file() {
            Ok(None)
        }
        else {
            Ok(Some(token))
        }
    }

    pub fn to_vec(mut self) -> SoulResult<Vec<Token>> {
        let mut tokens = vec![];

        while let Some(token) = self.next()? {
            tokens.push(token);
        }

        Ok(tokens)
    }
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self {
            kind,
            span,
        }
    }

    pub fn is_end_of_file(&self) -> bool {
        matches!(self.kind, TokenKind::EndFile)
    }
}