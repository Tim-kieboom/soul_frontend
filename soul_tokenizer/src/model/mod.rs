pub mod keyword;
pub mod types;

use crate::model::{keyword::KeyWord, types::Types};
use soul_utils::{literal::TokenLiteral, soul_names::Symbol, span::Span};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TokenKind {
    StringFormat(StringFormatTag),
    FStringPart(String),
    FStringEnd,
    Literal(TokenLiteral),
    Keyword(KeyWord),
    Symbol(Symbol),
    Ident(String),
    Types(Types),
    EndLine,
    EndFile,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StringFormatTag {
    /// format to string
    F,
    /// get format string
    Fstr,
}
impl StringFormatTag {
    pub const fn as_str(&self) -> &str {
        match self {
            StringFormatTag::F => "f",
            StringFormatTag::Fstr => "fstr",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}
impl TokenKind {
    const END_FILE_STR: &str = "<end of file>";
    const END_LINE_STR: &str = "'\\n'";

    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.write_display(&mut sb).expect("no fmt error");
        sb
    }

    pub fn write_display(&self, sb: &mut String) -> std::fmt::Result {
        use std::fmt::Write;

        match self {
            TokenKind::Literal(TokenLiteral::Number(number)) => number.write_display(sb)?,
            TokenKind::EndFile => sb.push_str(Self::END_FILE_STR),
            TokenKind::EndLine => sb.push_str(Self::END_LINE_STR),
            TokenKind::Literal(TokenLiteral::String(str)) => write!(sb, "{str:?}")?,
            TokenKind::Literal(TokenLiteral::Char(char)) => write!(sb, "{char:?}")?,
            TokenKind::Ident(ident) => sb.push_str(ident),
            TokenKind::Types(types) => sb.push_str(types.as_str()),
            TokenKind::Symbol(symbol) => sb.push_str(symbol.as_str()),
            TokenKind::Keyword(key_word) => sb.push_str(key_word.as_str()),
            TokenKind::StringFormat(fmt) => {
                sb.push_str(fmt.as_str());
            }
            TokenKind::FStringPart(text) => {
                write!(sb, "fstring_part({text:?})")?;
            }
            TokenKind::FStringEnd => {
                sb.push_str("fstring_end");
            }
        };
        Ok(())
    }
}
