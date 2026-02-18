use anyhow::Result;
use soul_utils::{soul_names::PrimitiveTypes, span::Span, symbool_kind::SymbolKind};
use std::{fmt::Write};

/// A single token containing its kind and source span information.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind/variant of this token.
    pub kind: TokenKind,
    /// The source code span for this token.
    pub span: Span,
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
    Symbol(SymbolKind),
    /// String literal, e.g. `"hello"`.
    StringLiteral(String),
}
impl TokenKind {
    pub fn is_end_line(&self) -> bool {
        matches!(self, TokenKind::EndLine)
    }

    pub fn is_end_file(&self) -> bool {
        matches!(self, TokenKind::EndFile)
    }
}

/// Represents different numeric literal kinds parsed from source code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    /// Signed integer literal, e.g. `-42`.
    Int(i64),
    /// Unsigned integer literal, e.g. `42u32`.
    Uint(u64),
    /// Floating point literal, e.g. `3.14`.
    Float(f64),
}

impl TokenKind {
    const END_FILE_STR: &str = "<end of file>";
    const END_LINE_STR: &str = "'\\n'";

    /// Returns a display string representation of the token kind.
    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb).expect("write should not fail");
        sb
    }

    pub fn write_display(&self, sb: &mut String) {
        self.inner_display(sb).expect("write should not fail");
    }

    pub fn inner_display(&self, sb: &mut String) -> Result<usize> {
        let old = sb.len();
        match self {
            TokenKind::Number(number) => _ = number.inner_display(sb)?,
            TokenKind::EndFile => sb.push_str(Self::END_FILE_STR),
            TokenKind::EndLine => sb.push_str(Self::END_LINE_STR),
            TokenKind::Unknown(char) => write!(sb, "Unknown('{char}')")?,
            TokenKind::Ident(ident) => write!(sb, "\"{ident}\"")?,
            TokenKind::CharLiteral(char) => write!(sb, "char('{char}')")?,
            TokenKind::StringLiteral(str) => write!(sb, "str(\"{str}\")")?,
            TokenKind::Symbol(symbool_kind) => write!(sb, "'{}'", symbool_kind.as_str())?,
        };
        Ok(sb.len().saturating_sub(old))
    }

    pub fn display_len(&self) -> usize {
        match self {
            TokenKind::EndFile => Self::END_FILE_STR.len(),
            TokenKind::EndLine => Self::END_LINE_STR.len(),
            TokenKind::Ident(ident) => "\"".len() + ident.len() + "\"".len(),
            TokenKind::Unknown(_) => "Unknown('".len() + 1 + "')".len(),
            TokenKind::Number(number) => {
                number.inner_display(&mut String::new())
                    .expect("no write error")
            }
            TokenKind::CharLiteral(_) => "char(".len() + 3 + "')".len(),
            TokenKind::Symbol(symbol_kind) => "\"".len() + symbol_kind.as_str().len() + "\"".len(),
            TokenKind::StringLiteral(str) => "str(\"".len() + str.len() + "\")".len(),
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
        let mut sb = String::new();
        self.inner_display(&mut sb).expect("write should not fail");
        sb
    }

    pub fn write_display(&self, sb: &mut String) {
        self.inner_display(sb).expect("write should not fail");
    }

    fn inner_display(&self, sb: &mut String) -> Result<usize> {
        
        const INT_STR: &str = PrimitiveTypes::UntypedInt.as_str();
        const UINT_STR: &str = PrimitiveTypes::UntypedUint.as_str();
        const FLOAT_STR: &str = PrimitiveTypes::UntypedFloat.as_str();
        
        let len = sb.len();
        match self {
            Number::Int(num) => write!(sb, "{num}: {INT_STR}"),
            Number::Uint(num) => write!(sb, "{num}: {UINT_STR}"),
            Number::Float(num) => write!(sb, "{num}: {FLOAT_STR}"),
        }?;
        Ok(sb.len().saturating_sub(len))
    }
}
