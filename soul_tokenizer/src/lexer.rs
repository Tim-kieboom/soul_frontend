use soul_utils::{fault::{Fault, SoulResult}, literal::{Literal, Number, StringLiteral, StringTag}, span::{ModuleId, Span, SpanLine}};
use crate::{model::{Token, TokenKind, keyword::KeyWord, symbol::Symbol, types::Types}, str_iter::StrIter};

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    module: ModuleId,
    line: SpanLine,
    current: Option<char>,
    input: StrIter<'a>,
}

impl<'a> Lexer<'a> {

    pub(crate) fn new(source: &'a str, module: ModuleId) -> Self {
        let mut lexer = Lexer {
            module,
            line: SpanLine {
                line: 1,
                offset: 0,
            },
            current: None,
            input: StrIter::new(source),
        };
        lexer.next_char();
        lexer
    }

    /// Advances to the next character, updating line/offset tracking.
    pub(crate) fn next_char(&mut self) {
        self.current = self.input.next();
        if let Some(char) = self.current {
            if char == '\n' {
                self.line.line += 1;
                self.line.offset = 0;
            } else {
                self.line.offset += 1;
            }
        }
    }

    /// Peeks at the next character without consuming it.
    pub(crate) fn peek_char(&mut self) -> Option<char> {
        self.input.peek()
    }

    pub(crate) fn next(&mut self) -> SoulResult<Token> {
        
        if self.current.is_none() {
            return Ok(Token::new(
                TokenKind::EndFile, 
                self.span(self.line)
            ))
        }

        self.skip_whitespace();

        let peek = self.peek_char();
        if self.current == Some('/') && peek == Some('/') {
            self.skip_line_comment();
            self.skip_whitespace();
            return Ok(Token::new(
                TokenKind::EndLine,
                Span::new_line(self.module, self.line),
            ));
        } else if self.current == Some('/') && peek == Some('*') {
            self.skip_multi_comment();
            self.skip_whitespace();
        }

        let line = self.line;
        if let Some(symbol) = self.try_get_symbol() {
            
            let kind = if self.is_negative_number(symbol) {
                TokenKind::Literal(Literal::Number(self.lex_number(line)?))
            }  else {
                self.next_char();
                TokenKind::Symbol(symbol)
            };

            return Ok(Token::new(kind, self.span(line)))   
        }

        let Some(current) = self.current else {
            return Ok(Token::new(
                TokenKind::EndFile, 
                self.span(self.line)
            ))
        };
        
        let kind = self.get_token_kind(current, line)?;
        if kind == TokenKind::EndLine {
            return Ok(Token::new(kind, Span::new_line(self.module, line)))
        }

        Ok(Token::new(kind, self.span(line)))
    }

    fn get_token_kind(&mut self, char: char, line: SpanLine) -> SoulResult<TokenKind> {
        
        if let Some(tag) = self.try_get_string_tag(char) {
            let str = self.lex_string(line)?;
            let string = match tag {
                StringTag::CStr => StringLiteral::Cstr(str),
            };

            return Ok(TokenKind::Literal(Literal::StringLiteral(string)))
        }


        Ok(match char {
            '\n' | '\r' => {
                self.next_char();
                if char == '\r' && self.current ==  Some('\n') {
                    self.next_char()
                }
                TokenKind::EndLine
            }
            '"' => {
                let string = StringLiteral::Str(self.lex_string(line)?);
                TokenKind::Literal(Literal::StringLiteral(string))
            }
            '\'' => TokenKind::Literal(Literal::Char(self.lex_char(line)?)),
            ch if is_ident(ch) => {
                let ident_str = self.lex_ident();
                if let Some(keyword) = KeyWord::from_str(ident_str) {
                    TokenKind::Keyword(keyword)
                } else if let Some(types) = Types::from_str(ident_str) {
                    TokenKind::Types(types)
                } else {
                    TokenKind::Ident(ident_str.to_string())
                }
            }
            ch if is_number(ch) => TokenKind::Literal(Literal::Number(self.lex_number(line)?)),
            _ => {
                self.next_char();
                return Err(Fault::error(
                    format!("{char:?} is unknown"),
                    Some(self.span(line))
                ));
            }
        })
    }

    fn lex_ident(&mut self) -> &str {

        let start = self.input.position();
        while let Some(char) = self.current {
            if char.is_alphabetic() || char == '_' || is_number(char) {
                self.next_char();
            } else {
                break;
            }
        }

        &self.input.slice(start..self.input.position())
    }

    fn lex_char(&mut self, line: SpanLine) -> SoulResult<char> {
        self.next_char();

        let char = if self.current == Some('\\') {
            self.next_char();
            match self.current {
                Some('n') => '\n',
                Some('r') => '\r',
                Some('t') => '\t',
                Some('0') => '\0',
                Some('\'') => '\'',
                Some('\\') => '\\',
                _ => {
                    return Err(Fault::error(
                        "Unclosed char literal escape sequence",
                        Some(self.span(line)),
                    ));
                }
            }
        } else if let Some(char) = self.current {
            char
        } else {
            return Err(Fault::error(
                "Unclosed char literal",
                Some(self.span(line)),
            ));
        };

        if self.peek_char() != Some('\'') {
            return Err(Fault::error(
                "char literal should end with \'",
                Some(self.span(line)),
            ));
        }

        self.next_char();
        self.next_char();
        Ok(char)
    }

    fn lex_string(&mut self, line: SpanLine) -> SoulResult<String> {
        let mut cstr = String::new();
        let mut backslash = false;

        self.next_char();
        while let Some(ch) = self.current {
            if backslash {
                let escaped_char = match ch {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    other => other,
                };
                cstr.push(escaped_char);
                backslash = false;
            } else if ch == '\\' {
                backslash = true;
            } else if ch == '"' {
                self.next_char();
                return Ok(cstr);
            } else {
                cstr.push(ch);
            }
            self.next_char();
        }

        Err(Fault::error(
            "StringLiteral does not have an end qoute",
            Some(self.span(line)),
        ))
    }

    fn try_get_string_tag(&mut self, char: char) -> Option<StringTag> {
        match StringTag::from_char(char) {
            Some(tag) if self.peek_char() == Some('"') => {
                self.next_char();
                Some(tag)
            }
            _ => None,
        }
    }

    fn lex_number(&mut self, line: SpanLine) -> SoulResult<Number> {
        let mut string = String::new();
        let mut is_float = false;
        let mut has_minus = false;

        if self.current == Some('-') {
            has_minus = true;
            string.push('-');
            self.next_char();
        }

        while let Some(ch) = self.current && is_number(ch) {
            string.push(ch);
            self.next_char();
        }

        if self.current == Some('.') && self.peek_char() != Some('.') {
            is_float = self.lex_float(&mut string);
        }

        if is_float {
            string.parse::<f64>().map(Number::Float).map_err(|err| {
                Fault::error(
                    err.to_string(),
                    Some(self.span(line)),
                )
            })
        } else if has_minus {
            string.parse::<i64>().map(Number::Int).map_err(|err| {
                Fault::error(
                    err.to_string(),
                    Some(self.span(line)),
                )
            })
        } else {
            string.parse::<u64>().map(Number::Uint).map_err(|err| {
                Fault::error(
                    err.to_string(),
                    Some(self.span(line)),
                )
            })
        }
    }

    fn lex_float(&mut self, string: &mut String) -> bool {
        string.push('.');
        self.next_char();

        while let Some(ch) = self.current && is_number(ch) {
            string.push(ch);
            self.next_char();
        }

        true
    }

    fn try_get_symbol(&mut self) -> Option<Symbol> {
        let current = self.current?;
        
        let mut pos = 0;
        let mut buf = [0u8; 8];
        let first = current.encode_utf8(&mut buf[pos..]).len();
        pos += first;
        if let Some(peek) = self.peek_char() {
            pos += peek.encode_utf8(&mut buf[pos..]).len();
        };
        
        let str_slice: &str = std::str::from_utf8(&buf[..pos]).ok()?;
        
        match Symbol::from_str(str_slice) {
            Some(symbol) => {
                if pos > first {
                    self.next_char();
                }
                Some(symbol)
            }
            None => Symbol::from_str(&str_slice[..first]),
        }
    }

    fn is_negative_number(&mut self, symbol: Symbol) -> bool {
        if symbol != Symbol::Minus {
            return false
        }

        match self.peek_char() {
            Some(peek) => is_number(peek),
            None => false,
        }
    }


    fn skip_line_comment(&mut self) {
        while let Some(char) = self.current {
            self.next_char();
            if char == '\n' || char == '\r' {
                break;
            }
        }
    }

    fn skip_multi_comment(&mut self) {
        let mut star = false;
        while let Some(char) = self.current {
            self.next_char();
            if char == '/' && star {
                break;
            }
            star = char == '*';
        }
    }

    fn skip_whitespace(&mut self) {
        while self.current == Some(' ') || self.current == Some('\t') {
            self.next_char();
        }
    }

    fn span(&self, line: SpanLine) -> Span {
        Span::new(
            self.module, 
            line,
            self.line
        )
    }
}

fn is_ident(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_number(ch: char) -> bool {
    ch.is_ascii_digit()
}