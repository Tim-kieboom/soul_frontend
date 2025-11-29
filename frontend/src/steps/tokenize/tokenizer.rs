use std::iter::Peekable;
use std::str::Chars;
use models::error::{SoulErrorKind, ExpansionId, SoulError, SoulResult, Span};
use models::symbool_kind::SymboolKind;
use crate::steps::tokenize::from_lexer::FromLexer;
use crate::steps::tokenize::{Request, Response};
use crate::steps::tokenize::token_stream::{Number, Token, TokenKind, TokenStream};

pub fn tokenize<'a>(request: Request<'a>) -> Response<'a> {
    Response{
        token_stream: TokenStream::new(Lexer::new(request.source))
    }
}

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    current_char: Option<char>,
    line: usize,
    offset: usize,
}

impl<'a> Lexer<'a> {

    pub(crate) fn new(source: &'a str) -> Self {
        let mut lexer = Lexer {
            input: source.chars().peekable(),
            current_char: None,
            line: 1,
            offset: 0,
        };
        lexer.next_char();
        lexer
    }

    pub(crate) fn current_char(&self) -> Option<char> {
        self.current_char
    }

    pub(crate) fn next_token(&mut self) -> SoulResult<Token> {
        if self.current_char.is_none() {
            return Ok(Token::new(
                TokenKind::EndFile, 
                self.new_span(self.line, self.offset)
            ))
        }

        self.skip_whitespace();
        
        let start_line = self.line;
        let start_offset = self.offset;

        let peek = self.peek_char();
        if self.current_char == Some('/') && peek == Some('/') {
            self.skip_line_comment();
            self.skip_whitespace();
            return Ok(Token::new(
                TokenKind::EndLine,
                self.new_span(start_line, start_offset)
            ))
        }
        else if self.current_char == Some('/') && peek == Some('*') {
            self.skip_multi_comment();
            self.skip_whitespace();
        }

        if let Some(symbool) = SymboolKind::from_lexer(self) {
            self.next_char();
            return Ok(Token::new(
                TokenKind::Symbool(symbool),
                self.new_span(start_line, start_offset)
            ))
        } 

        let char = match self.current_char {
            Some(val) => val,
            None => return Ok(Token::new(
                TokenKind::EndFile, 
                self.new_span(start_line, start_offset)
            )),
        };

        let kind = self.get_token_kind(char, start_line, start_offset)?;
        Ok(Token::new(kind, self.new_span(start_line, start_offset)))
    }

    pub(crate) fn next_char(&mut self) {
        self.current_char = self.input.next();
        if let Some(char) = self.current_char {
            if char == '\n' {
                self.line += 1;
                self.offset = 0;
            } 
            else {
                self.offset += 1;
            }
        }
    }

    pub(crate) fn peek_char(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn get_token_kind(&mut self, char: char, start_line: usize, start_offset: usize) -> SoulResult<TokenKind> {
        
        Ok(match char {
            '\n' | '\r' => {
                self.next_char();
                TokenKind::EndLine
            }
            '"' => TokenKind::StringLiteral(self.get_string(start_line, start_offset)?),
            '\'' => TokenKind::CharLiteral(self.get_char_literal(start_line, start_offset)?),
            ch if is_ident(ch) => TokenKind::Ident(self.get_ident()),
            ch if is_number(ch) => TokenKind::Number(self.get_number(start_line, start_offset)?),
            _ => {
                self.next_char();
                TokenKind::Unknown(char)
            }
        })
    }

    fn skip_line_comment(&mut self) {

        while let Some(char) = self.current_char {
            self.next_char();
            if char == '\n' || char == '\r' {
                break
            } 
        }
    }

    fn skip_multi_comment(&mut self) {

        let mut star = false;
        while let Some(char) = self.current_char {
            self.next_char();
            if char == '/' && star {
                break
            }
            star = char == '*';
        }
    }

    fn get_char_literal(&mut self, start_line: usize, start_offset: usize) -> SoulResult<char> {
        self.next_char();

        let char = if self.current_char == Some('\\') {
            self.next_char();
            match self.current_char {
                Some('n') => '\n',
                Some('r') => '\r',
                Some('t') => '\t',
                Some('\'') => '\'',
                Some('\\') => '\\',
                Some(other) => other,
                None => {
                    return Err(SoulError::new(
                        "Unclosed char literal escape sequence",
                        SoulErrorKind::InvalidEscapeSequence,
                        Some(self.new_span(start_line, start_offset)),
                    ));
                }
            }
        }
        else if let Some(char) = self.current_char {
            char
        }
        else {
            return Err(SoulError::new(
                "Unclosed char literal",
                SoulErrorKind::InvalidEscapeSequence,
                Some(self.new_span(start_line, start_offset)),
            ));
        };

        self.next_char();

        if self.current_char != Some('\'') {
            return Err(SoulError::new(
                "Char literal missing closing quote",
                SoulErrorKind::InvalidEscapeSequence,
                Some(self.new_span(start_line, start_offset))
            ));
        }

        self.next_char();
        Ok(char)
    }

    fn get_string(&mut self, start_line: usize, start_offset: usize) -> SoulResult<String> {
        let mut cstring = String::new();
        let mut backslash = false;

        self.next_char();
        while let Some(ch) = self.current_char {
            if backslash {
                let escaped_char = match ch {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    other => other,
                };
                cstring.push(escaped_char);
                backslash = false;
            } 
            else if ch == '\\' {
                backslash = true;
            } 
            else if ch == '"' {
                self.next_char();
                return Ok(cstring);
            } 
            else {
                cstring.push(ch);
            }
            self.next_char();
        }

        Err(SoulError::new("cString does not have an end qoute", SoulErrorKind::InvalidEscapeSequence, Some(self.new_span(start_line, start_offset))))
    }

    fn get_number(&mut self, start_line: usize, start_offset: usize) -> SoulResult<Number> {
        let mut num_str = String::new();
        let mut is_float = false;
        let mut has_minus = false;

        if self.current_char == Some('-') {
            has_minus = true;
            num_str.push('-');
            self.next_char();
        }

        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.next_char();
            } 
            else {
                break
            }
        }

        if self.current_char == Some('.') && self.peek_char() != Some('.') {
            is_float = self.lex_float(&mut num_str)
        }

        if let Some(ch) = self.current_char {
            if ch == 'e' || ch == 'E' {
                is_float = true;
                self.lex_exponextion_number(ch, &mut num_str, start_line, start_offset)?;
            }
        }

        if is_float {
            num_str.parse::<f64>()
                .map(|num| Number::Float(num))
                .map_err(|err| SoulError::new(err.to_string(), SoulErrorKind::InvalidNumber, Some(self.new_span(start_line, start_offset))))
        } 
        else if has_minus {
            num_str.parse::<i64>()
                .map(|num| Number::Int(num))
                .map_err(|err| SoulError::new(err.to_string(), SoulErrorKind::InvalidNumber, Some(self.new_span(start_line, start_offset))))
        } 
        else {
            num_str.parse::<u64>()
                .map(|num| Number::Uint(num))
                .map_err(|err| SoulError::new(err.to_string(), SoulErrorKind::InvalidNumber, Some(self.new_span(start_line, start_offset))))
        }
    }

    fn lex_exponextion_number(&mut self, ch: char, num_str: &mut String, start_line: usize, start_offset: usize) -> SoulResult<()> {
        num_str.push(ch);
        self.next_char();

        if let Some(ch2) = self.current_char {
            if ch2 == '+' || ch2 == '-' {
                num_str.push(ch2);
                self.next_char();
            }
        }

        let mut digit_found = false;
        while let Some(ch3) = self.current_char {
            if ch3.is_ascii_digit() {
                digit_found = true;
                num_str.push(ch3);
                self.next_char();
            } 
            else {
                break
            }
        }

        if !digit_found {
            Err(SoulError::new("exponent must have digits", SoulErrorKind::InvalidNumber, Some(self.new_span(start_line, start_offset))))
        }
        else {
            Ok(())
        }
    }

    fn lex_float(&mut self, num_str: &mut String) -> bool {

        num_str.push('.');
        self.next_char();

        while let Some(ch) = self.current_char {

            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.next_char();
            } 
            else {
                break
            }
        }

        true
    }

    fn get_ident(&mut self) -> String {
        let mut ident = String::new();

        while let Some(char) = self.current_char {

            if char.is_alphabetic() || char == '_' || char.is_ascii_digit() {
                ident.push(char);
                self.next_char();
            }
            else {
                break
            }
        }

        ident
    }

    fn new_span(&self, start_line: usize, start_offset: usize) -> Span {
        Span{
            start_line, 
            start_offset, 
            end_line: self.line, 
            end_offset: self.offset, 
            expansion_id: ExpansionId::default(),
        }
    }

    fn skip_whitespace(&mut self) {
        
        while self.current_char == Some(' ') || self.current_char == Some('\t') {
            self.next_char();
        }
    }
}

fn is_ident(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_number(ch: char) -> bool {
    ch.is_ascii_digit()
}