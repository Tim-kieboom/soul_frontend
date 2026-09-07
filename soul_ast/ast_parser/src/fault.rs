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

    #[error(
        "'{modifier}' modifier cannot be applied to compound patterns; use per-binding '{modifier}' instead (e.g., ({modifier} a, b))"
    )]
    MutOnCompoundPattern { modifier: Box<str> },

    #[error("'{assign_op}' is not valid for variable declaration (can use ['=', ':='])")]
    InvalidAssignOperatorForDeclaration { assign_op: Box<str> },

    #[error("'mut' cannot be applied to constructor patterns; use per-field 'mut' instead")]
    MutOnConstructorPattern,

    #[error(
        "'mut' cannot be applied to tuple patterns; use per-element 'mut' instead (e.g., (mut a, b))"
    )]
    MutOnTuplePattern,

    #[error("'mut' cannot be applied to named-tuple patterns; use per-field 'mut' instead")]
    MutOnNamedTuplePattern,

    #[error("expected variable name, `_`, `(`, or `{{` but found `{found}`")]
    ExpectedPatternStart { found: Box<str> },

    #[error("StructConstructor already has '..'")]
    DuplicateStructSpread,

    #[error("StructConstructor's '..' should only be used at the end expected '}}'")]
    StructSpreadNotAtEnd,

    #[error("`{found}` is invalid as start of expression")]
    InvalidExpressionStart { found: Box<str> },

    #[error("expected format string part or end of format string")]
    UnterminatedFormatString,

    #[error("can not have {keyword} in expression")]
    KeywordNotAllowedInExpression { keyword: Box<str> },

    #[error("expected '(' or ':[' after 'new'")]
    ExpectedNewArguments,

    #[error("expected block after keyword")]
    ExpectedBlockAfterKeyword,

    #[error("expected '=>' in match arm")]
    ExpectedMatchArrow,

    #[error("`{symbol}` is invalid")]
    InvalidSymbolHere { symbol: Box<str> },

    #[error("should be ident")]
    ExpectedIdentBeforeCallArguments,

    #[error("expected identifier after '.'")]
    ExpectedIdentAfterDot,

    #[error("'{found}' should be a assign symbool")]
    ExpectedAssignSymbol { found: Box<str> },

    #[error("expected ',' or '}}' in import list")]
    ExpectedCommaOrCurlyCloseInImportList,

    #[error("could not pop path")]
    CouldNotPopImportPath,

    #[error("'{found}' should be '=' or ':='")]
    InvalidAssignSymbol { found: Box<str> },

    #[error("expected '=' or ':=' after constructor pattern")]
    ExpectedAssignAfterConstructorPattern,

    #[error("expected '=' or ':=' after destructuring pattern")]
    ExpectedAssignAfterDestructuringPattern,

    #[error("`{found}` is not a valid operator")]
    InvalidOperator { found: Box<str> },

    #[error("`{found}` is not a valid unary operator")]
    InvalidUnaryOperator { found: Box<str> },

    #[error("expected ident or `null` or `!null` but got {found}")]
    ExpectedIdentOrNullForTypeof { found: Box<str> },

    #[error("expected `{expected1}` or `{expected2}`, but got `{found}`")]
    ExpectedAssignOrDeclaration {
        expected1: Box<str>,
        expected2: Box<str>,
        found: Box<str>,
    },

    #[error("contructor function should have methode type")]
    ConstructorMissingMethodType,
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
