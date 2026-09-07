use soul_utils::fault::{Fault, UnclassifiedKind};

/// Structured error kinds for the tokenizer/lexer. `Unclassified` is a
/// migration fallback carrying the raw message from call sites not yet
/// converted to a real variant.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum TokenErrorKind {
    #[error("{0}")]
    Unclassified(Box<str>),

    #[error("{found:?} is unknown")]
    UnknownChar { found: char },

    #[error("unexpected character {found:?} in format string")]
    UnexpectedCharInFormatString { found: char },

    #[error("unclosed format string literal")]
    UnclosedFormatString,

    #[error("Unclosed char literal escape sequence")]
    UnclosedCharEscape,

    #[error("Unclosed char literal")]
    UnclosedCharLiteral,

    #[error("char literal should end with '")]
    CharLiteralMissingEndQuote,

    #[error("StringLiteral does not have an end qoute")]
    UnterminatedString,

    #[error("invalid suffix after number literal")]
    InvalidNumberSuffix,

    #[error("{0}")]
    InvalidNumberLiteral(Box<str>),
}

impl From<UnclassifiedKind> for TokenErrorKind {
    fn from(value: UnclassifiedKind) -> Self {
        TokenErrorKind::Unclassified(value.0)
    }
}

impl From<TokenErrorKind> for UnclassifiedKind {
    fn from(value: TokenErrorKind) -> Self {
        UnclassifiedKind(value.to_string().into_boxed_str())
    }
}

pub type TokenFault = Fault<TokenErrorKind>;
