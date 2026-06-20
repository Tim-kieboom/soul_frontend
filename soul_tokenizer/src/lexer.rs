#[cfg(debug_assertions)]
use std::sync::Once;

use crate::{
    model::{StringFormatTag, Token, TokenKind, keyword::KeyWord, types::Types},
    str_iter::StrIter,
};
use soul_utils::{
    error::SoulResult,
    fault::Fault,
    literal::{Number, StringLiteral, StringTag, TokenLiteral},
    soul_names::Symbol,
    span::{ModuleId, Span, SpanLine},
};

#[cfg(debug_assertions)]
static TRY_GET_SYMBOL_INIT: Once = Once::new();

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    module: ModuleId,
    line: SpanLine,
    input: StrIter<'a>,
    current: Option<char>,
    pub(crate) in_fstr: bool,
    fstr_brace_depth: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(source: &'a str, module: ModuleId) -> Self {
        let mut lexer = Lexer {
            module,
            line: SpanLine { line: 1, offset: 0 },
            current: None,
            input: StrIter::new(source),
            in_fstr: false,
            fstr_brace_depth: 0,
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

    pub fn next(&mut self) -> SoulResult<Token> {
        if self.current.is_none() {
            return Ok(Token::new(TokenKind::EndFile, self.span(self.line)));
        }

        if self.in_fstr {
            return self.lex_fstring_part();
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
                TokenKind::Literal(TokenLiteral::Number(self.lex_number(line)?))
            } else {
                self.next_char();
                if symbol == Symbol::CurlyClose && self.fstr_brace_depth > 0 {
                    self.fstr_brace_depth -= 1;
                    if self.fstr_brace_depth == 0 {
                        self.in_fstr = true;
                    }
                }
                TokenKind::Symbol(symbol)
            };

            return Ok(Token::new(kind, self.span(line)));
        }

        let Some(current) = self.current else {
            return Ok(Token::new(TokenKind::EndFile, self.span(self.line)));
        };

        let kind = self.get_token_kind(current, line)?;
        if kind == TokenKind::EndLine {
            return Ok(Token::new(kind, Span::new_line(self.module, line)));
        }

        Ok(Token::new(kind, self.span(line)))
    }

    fn get_token_kind(&mut self, char: char, line: SpanLine) -> SoulResult<TokenKind> {
        
        let string_tag = match self.try_get_ident_or_tag(char) {
            Ok(val) => val,
            Err(ident_str) => {
                if let Some(keyword) = KeyWord::from_str(&ident_str) {
                    return Ok(TokenKind::Keyword(keyword))
                } else if let Some(types) = Types::from_str(&ident_str) {
                    return Ok(TokenKind::Types(types))
                } else {
                    return Ok(TokenKind::Ident(ident_str))
                }
            }
        };

        if let Some(tag) = string_tag {
            let string = match tag {
                StringTag::CStr => {
                    self.next_char();
                    StringLiteral::Cstr(self.lex_string(line)?)
                }
                StringTag::F => {
                    self.next_char();
                    self.next_char();
                    self.in_fstr = true;
                    return Ok(TokenKind::StringFormat(StringFormatTag::F))
                }
                StringTag::Fstr => {
                    self.next_char();
                    self.in_fstr = true;
                    return Ok(TokenKind::StringFormat(StringFormatTag::Fstr))
                }
            };

            return Ok(TokenKind::Literal(TokenLiteral::String(string)));
        }

        Ok(match char {
            '\n' | '\r' => {
                self.next_char();
                if char == '\r' && self.current == Some('\n') {
                    self.next_char()
                }
                TokenKind::EndLine
            }
            '"' => {
                let string = StringLiteral::Str(self.lex_string(line)?);
                TokenKind::Literal(TokenLiteral::String(string))
            }
            '\'' => TokenKind::Literal(TokenLiteral::Char(self.lex_char(line)?)),
            ch if is_number(ch) => TokenKind::Literal(TokenLiteral::Number(self.lex_number(line)?)),
            _ => {
                self.next_char();
                return Err(Fault::error(
                    format!("{char:?} is unknown"),
                    Some(self.span(line)),
                ));
            }
        })
    }

    fn lex_format_string_part(&mut self) -> String {
        let mut string = String::new();
        while let Some(char) = self.current {
            match char {
                '"' => break,
                '{' if self.peek_char() == Some('{') => {
                    string.push('{');
                    self.next_char();
                    self.next_char();
                }
                '}' if self.peek_char() == Some('}') => {
                    string.push('}');
                    self.next_char();
                    self.next_char();
                }
                '{' => break,
                _ => {
                    string.push(char);
                    self.next_char();
                }
            }
        }
        string
    }

    fn lex_fstring_part(&mut self) -> SoulResult<Token> {
        let line = self.line;
        let text = self.lex_format_string_part();

        match self.current {
            Some('"') if text.is_empty() => {
                self.next_char();
                self.in_fstr = false;
                Ok(Token::new(TokenKind::FStringEnd, self.span(line)))
            }
            Some('"') => {
                Ok(Token::new(TokenKind::FStringPart(text), self.span(line)))
            }
            Some('{') => {
                self.fstr_brace_depth += 1;
                self.in_fstr = false;
                Ok(Token::new(TokenKind::FStringPart(text), self.span(line)))
            }
            Some(ch) => {
                self.next_char();
                Err(Fault::error(
                    format!("unexpected character {ch:?} in format string"),
                    Some(self.span(line)),
                ))
            }
            None => Err(Fault::error(
                "unclosed format string literal".to_string(),
                Some(self.span(line)),
            )),
        }
    }

    fn lex_ident(&mut self) -> (&str, Option<char>) {
        let start = self.input.position();
        while let Some(char) = self.current {
            if char.is_alphabetic() || char == '_' || is_number(char) {
                self.next_char();
            } else {
                break;
            }
        }

        let peek = self.peek_char();
        let slice = &self.input.slice(start..self.input.position());
        (slice, peek)
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
            return Err(Fault::error("Unclosed char literal", Some(self.span(line))));
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

    fn try_get_ident_or_tag(&mut self, char: char) -> Result<Option<StringTag>, String> {

        if self.peek_char() == Some('"') {

            match char {
                'f' => return Ok(Some(StringTag::F)),
                'c' => return Ok(Some(StringTag::CStr)),
                _ => (),
            }
        }

        if !is_ident(char) {
            return Ok(None)
        }

        let (string, _peek) = self.lex_ident();
        let string_owned = string.to_string();
        if string == "fstr" && self.current == Some('"') {
            return Ok(Some(StringTag::Fstr))
        }

        Err(string_owned)
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

        while let Some(ch) = self.current
            && is_number(ch)
        {
            string.push(ch);
            self.next_char();
        }

        if self.current == Some('.') && self.peek_char() != Some('.') {
            is_float = self.lex_float(&mut string);
        }

        if is_float {
            string
                .parse::<f64>()
                .map(Number::Float)
                .map_err(|err| Fault::error(err.to_string(), Some(self.span(line))))
        } else if has_minus {
            string
                .parse::<i64>()
                .map(Number::Int)
                .map_err(|err| Fault::error(err.to_string(), Some(self.span(line))))
        } else {
            string
                .parse::<u64>()
                .map(Number::Uint)
                .map_err(|err| Fault::error(err.to_string(), Some(self.span(line))))
        }
    }

    fn lex_float(&mut self, string: &mut String) -> bool {
        string.push('.');
        self.next_char();

        while let Some(ch) = self.current
            && is_number(ch)
        {
            string.push(ch);
            self.next_char();
        }

        true
    }

    fn try_get_symbol(&mut self) -> Option<Symbol> {
        let current = self.current?;

        #[cfg(debug_assertions)]
        TRY_GET_SYMBOL_INIT
            .call_once(|| debug_assert!(Symbol::STRING_VALUES.iter().all(|name| name.len() <= 2)));

        if let Some(peek) = self.peek_char() {
            let two_str = [current, peek].iter().collect::<String>();

            if let Some(symbol) = Symbol::from_str(&two_str) {
                self.next_char();
                return Some(symbol);
            }
        }

        Symbol::from_str(&current.to_string())
    }

    fn is_negative_number(&mut self, symbol: Symbol) -> bool {
        if symbol != Symbol::Minus {
            return false;
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
        Span::new(self.module, line, self.line)
    }
}

fn is_ident(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_number(ch: char) -> bool {
    ch.is_ascii_digit()
}
