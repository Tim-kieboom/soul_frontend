use soul_utils::{soul_names::InternalPrimitiveTypes, span::Span, symbool_kind::SymboolKind};

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
    Symbol(SymboolKind),
    /// String literal, e.g. `"hello"`.
    StringLiteral(String),
}

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

impl TokenKind {
    /// Returns a display string representation of the token kind.
    pub fn display(&self) -> String {
        match self {
            TokenKind::Ident(ident) => format!("\"{ident}\""),
            TokenKind::Unknown(char) => format!("'{char}'"),
            TokenKind::EndFile => "<end of file>".to_string(),
            TokenKind::EndLine => "<end of line>".to_string(),
            TokenKind::CharLiteral(char) => format!("r#'{char}'"),
            TokenKind::Number(number) => number.display(),
            TokenKind::StringLiteral(str) => format!("r#\"{str}\""),
            TokenKind::Symbol(symbool_kind) => format!("'{}'", symbool_kind.as_str()),
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
        const INT_STR: &str = InternalPrimitiveTypes::UntypedInt.as_str();
        const UINT_STR: &str = InternalPrimitiveTypes::UntypedUint.as_str();
        const FLOAT_STR: &str = InternalPrimitiveTypes::UntypedFloat.as_str();

        match self {
            Number::Int(num) => format!("{num}: {INT_STR}"),
            Number::Uint(num) => format!("{num}: {UINT_STR}"),
            Number::Float(num) => format!("{num}: {FLOAT_STR}"),
        }
    }
}
