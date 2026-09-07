use soul_utils::fault::{Fault, FaultCollector, UnclassifiedKind};

/// Structured error kinds for the AST parser. `Unclassified` is a migration
/// fallback carrying the raw message from call sites not yet converted to a
/// real variant.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum AstErrorKind {
    #[error("{0}")]
    Unclassified(Box<str>),

    #[error("can not have more then one 'this' in methode")]
    DuplicateThisParameter,

    #[error("RawPtr expects exactly one generic type parameter, e.g. `RawPtr<int>`")]
    RawPtrExpectsOneGeneric,

    #[error("Res expects at most two generic type parameters, e.g. `Res<int, str>`")]
    ResExpectsAtMostTwoGenerics,

    #[error("expected element type after array size, e.g. `[64]char`")]
    ArrayMissingElementType,

    #[error("expected string_literal of language name but got {found}")]
    ExpectedLanguageStringLiteral { found: Box<str> },

    #[error("expected normal string_literal of language name but got {tag} string_literal")]
    ExpectedNormalLanguageStringLiteral { tag: Box<str> },

    #[error("language {language} is not supported")]
    UnsupportedExternLanguage { language: Box<str> },
}

impl From<UnclassifiedKind> for AstErrorKind {
    fn from(value: UnclassifiedKind) -> Self {
        AstErrorKind::Unclassified(value.0)
    }
}

/// Lets an `AstErrorKind` fault flow back into a not-yet-migrated,
/// `UnclassifiedKind`-typed `TryResult` (e.g. via `.try_err()`), downgrading
/// the structured kind to its rendered message.
impl From<AstErrorKind> for UnclassifiedKind {
    fn from(value: AstErrorKind) -> Self {
        UnclassifiedKind(value.to_string().into_boxed_str())
    }
}

pub type AstFault = Fault<AstErrorKind>;
pub type AstFaultCollector = FaultCollector<AstErrorKind>;
