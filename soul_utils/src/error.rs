use crate::span::Span;

// A result type alias for operations that can fail with a `SoulError`.
pub type SoulResult<T> = std::result::Result<T, SoulError>;

/// The kind of error that occurred during parsing or compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoulErrorKind {
    InvalidNumber,
    ScopeOverride(Span),
    UnexpectedCharacter,
    InvalidEscapeSequence,
}


/// An error that occurred during parsing or compilation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SoulError {
    pub kind: SoulErrorKind,
    pub message: String,
    pub span: Option<Span>,
}

impl SoulError {

    pub fn new<S: Into<String>>(message: S, kind: SoulErrorKind, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            kind,
            span,
        }
    }
}