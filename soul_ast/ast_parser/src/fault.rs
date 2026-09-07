use soul_utils::{
    collections::try_result::TryResult,
    fault::{Fault, UnclassifiedKind},
};

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

    #[error("expected ident")]
    ExpectedIdentForTypeAssert,

    #[error("can not have '{else_kw}' or '{else_kw} {if_kw}' after '{else_kw}'")]
    DuplicateElseBranch { else_kw: Box<str>, if_kw: Box<str> },

    #[error("expected a literal or '_' for match pattern")]
    ExpectedLiteralOrWildcardPattern,

    #[error("expected variant name after '.' in constructor pattern")]
    ExpectedVariantNameInPattern,

    #[error("unexpected end of file while parsing expression")]
    UnexpectedEndOfFileInExpression,

    #[error("expected array literal or '(' after type constructor")]
    ExpectedArrayLiteralOrParenAfterTypeConstructor,

    #[error("external import missing crate name")]
    ExternalImportMissingCrateName,

    #[error("external crate '{lib_name}' not found in Soul.toml dependencies")]
    ExternalCrateNotFound { lib_name: Box<str> },

    #[error("no 'mod.soul' found in folder '{path}'")]
    MissingModFile { path: Box<str> },

    #[error("file '{path}' not found")]
    ModuleFileNotFound { path: Box<str> },

    #[error(
        "crate '{crate_name}' has no root file (lib.soul, main.soul, or mod.soul) in '{source_root}'"
    )]
    CrateMissingRootFile {
        crate_name: Box<str>,
        source_root: Box<str>,
    },

    #[error("token '{found}' not allowed in array typeWrapper")]
    InvalidArrayTypeWrapperToken { found: Box<str> },

    #[error("expected ident got `{found}`")]
    ExpectedIdent { found: Box<str> },

    #[error("expected: `{expected}` but found: `{found}`")]
    ExpectedExactToken { expected: Box<str>, found: Box<str> },

    #[error("expected: `{expected}` but found: `{found}`")]
    ExpectedExactIdent { expected: Box<str>, found: Box<str> },

    #[error("expected on of: [`{expected}`] but found: `{found}`")]
    ExpectedOneOfTokens { expected: Box<str>, found: Box<str> },

    #[error("`This.(..)` has to be static function")]
    NonStaticThisConstructor,

    #[error("you can not have a non default parameter after default parameter")]
    NonDefaultParameterAfterDefault,

    #[error("'{found}' not allowed in import")]
    TokenNotAllowedInImport { found: Box<str> },

    #[error(
        "`{token}` at the end of a line can only be used for expressions at the end of a block"
    )]
    ExpressionOnlyAtEndOfBlock { token: Box<str> },

    #[error("{kind} can not be used in struct body")]
    StatementNotAllowedInBody { kind: Box<str> },

    #[error("Variable is not allowed in use block")]
    VariableNotAllowedInUseBlock,

    #[error("keyword '{keyword}' can not be type")]
    KeywordUsedAsType { keyword: Box<str> },

    #[error(transparent)]
    LexError(#[from] soul_tokenizer::fault::TokenErrorKind),
}

impl From<UnclassifiedKind> for AstErrorKind {
    fn from(value: UnclassifiedKind) -> Self {
        AstErrorKind::Unclassified(value.0)
    }
}
impl From<AstErrorKind> for UnclassifiedKind {
    fn from(value: AstErrorKind) -> Self {
        UnclassifiedKind(value.to_string().into_boxed_str())
    }
}

pub type AstFault = Fault<AstErrorKind>;
pub type AstTryResult<O, N> = TryResult<O, N, AstFault>;
pub type AstResult<T> = std::result::Result<T, AstFault>;
