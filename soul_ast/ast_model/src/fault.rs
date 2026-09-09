use std::{fmt::Display, path::PathBuf};

use crate::{AssignType, SoulType};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    SharedStr,
    collections::try_result::TryResult,
    fault::{Fault, UnclassifiedKind},
    literal::StringTag,
    soul_names::{Operator, Symbol},
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

    #[error("expected string_literal of language name but got {}", found.display())]
    ExpectedLanguageStringLiteral { found: TokenKind },

    #[error("expected normal string_literal of language name but got {tag:?} string_literal")]
    ExpectedNormalLanguageStringLiteral { tag: StringTag },

    #[error("language {language} is not supported")]
    UnsupportedExternLanguage { language: Box<str> },

    #[error(
        "'mut' modifier cannot be applied to compound patterns; use per-binding 'mut' instead (e.g., (mut a, b))"
    )]
    MutOnCompoundPattern,

    #[error("'{}' is not valid for variable declaration (can use ['=', ':='])", assign.as_str())]
    InvalidAssignOperatorForDeclaration { assign: AssignType },

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

    #[error("can not have `{}` in expression", keyword.as_str())]
    KeywordNotAllowedInExpression { keyword: KeyWord },

    #[error("expected '(' or ':[' after 'new'")]
    ExpectedNewArguments,

    #[error("expected block after keyword")]
    ExpectedBlockAfterKeyword,

    #[error("expected '=>' in match arm")]
    ExpectedMatchArrow,

    #[error("`{}` is invalid", symbol.as_str())]
    InvalidSymbolHere { symbol: Symbol },

    #[error("should be ident")]
    ExpectedIdentBeforeCallArguments,

    #[error("expected identifier after '.'")]
    ExpectedIdentAfterDot,

    #[error("'{}' should be a assign symbool", found.display())]
    ExpectedAssignSymbol { found: TokenKind },

    #[error("expected ',' or '}}' in import list")]
    ExpectedCommaOrCurlyCloseInImportList,

    #[error("could not pop path")]
    CouldNotPopImportPath,

    #[error("'{}' should be '=' or ':='", found.display())]
    InvalidAssignSymbol { found: TokenKind },

    #[error("expected '=' or ':=' after constructor pattern")]
    ExpectedAssignAfterConstructorPattern,

    #[error("expected '=' or ':=' after destructuring pattern")]
    ExpectedAssignAfterDestructuringPattern,

    #[error("`{found}` is not a valid operator")]
    InvalidOperator { found: Box<str> },

    #[error("`{}` is not a valid unary operator", found.as_str())]
    InvalidUnaryOperator { found: Operator },

    #[error("expected ident or `null` or `!null` but got {found}")]
    ExpectedIdentOrNullForTypeof { found: Box<str> },

    #[error("expected `{}` or `{}`, but got `{found}`", expected1.as_str(), expected2.as_str())]
    ExpectedAssignOrDeclaration {
        expected1: AssignType,
        expected2: AssignType,
        found: Box<str>,
    },

    #[error("contructor function should have methode type")]
    ConstructorMissingMethodType,

    #[error("expected ident")]
    ExpectedIdentForTypeAssert,

    #[error("can not have 'else or 'else if' after 'else'")]
    DuplicateElseBranch,

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

    #[error("no 'mod.soul' found in folder '{path:?}'")]
    MissingModFile { path: PathBuf },

    #[error("file '{path}' not found")]
    ModuleFileNotFound { path: PathBuf },

    #[error(
        "crate '{crate_name}' has no root file (lib.soul, main.soul, or mod.soul) in '{source_root}'"
    )]
    CrateMissingRootFile {
        crate_name: Box<str>,
        source_root: Box<str>,
    },

    #[error("token '{}' not allowed in array typeWrapper", found.display())]
    InvalidArrayTypeWrapperToken { found: TokenKind },

    #[error("expected ident got `{}`", found.display())]
    ExpectedIdent { found: TokenKind },

    #[error("expected: `{}` but found: `{}`", expected, found)]
    ExpectedExactToken { expected: Box<str>, found: Box<str> },

    #[error("expected on of: [`{expected}`] but found: `{found}`")]
    ExpectedOneOfTokens { expected: Box<str>, found: Box<str> },

    #[error("`This.(..)` has to be static function")]
    NonStaticThisConstructor,

    #[error("you can not have a non default parameter after default parameter")]
    NonDefaultParameterAfterDefault,

    #[error("'{}' not allowed in import", found.display())]
    TokenNotAllowedInImport { found: TokenKind },

    #[error(
        "`{token}` at the end of a line can only be used for expressions at the end of a block"
    )]
    ExpressionOnlyAtEndOfBlock { token: Symbol },

    #[error("{kind} can not be used in struct body")]
    StatementNotAllowedInBody { kind: Box<str> },

    #[error("Variable is not allowed in use block")]
    VariableNotAllowedInUseBlock,

    #[error("keyword '{keyword}' can not be type")]
    KeywordUsedAsType { keyword: KeyWord },

    // ----------------------------------------------------------------
    //  Name resolution (soul_name_resolver shares this CrateContext)
    // ----------------------------------------------------------------
    #[error("module `{path:?}` not found in ModuleStore")]
    ImportedModuleNotFound { path: PathBuf },

    #[error("module `{module_name}` does not export `{item}`")]
    ModuleDoesNotExportItem {
        module_name: Box<str>,
        item: Box<str>,
    },

    #[error("{kind} '{name}' is private")]
    ItemIsPrivate { kind: Box<str>, name: Box<str> },

    #[error("{kind} '{name}' already exists")]
    ItemAliasAlreadyExists { kind: Box<str>, name: Box<str> },

    #[error("type of name {name} already exists in scope")]
    TypeAlreadyExistsInScope { name: Box<str> },

    #[error("`{name}` already exists in scope")]
    ValueAlreadyExistsInScope { name: Box<str> },

    #[error("parent and child function can not have the same name")]
    ParentChildFunctionSameName,

    #[error("function name can not be empty")]
    FunctionNameEmpty,

    #[error("function name should not start with '{found}' (start with letter or '_')")]
    FunctionNameInvalidStart { found: char },

    #[error("function name should not have '___' in the name")]
    FunctionNameTripleUnderscore,

    #[error("variable name can not be empty")]
    VariableNameEmpty,

    #[error("variable name should not start with '{found}' (start with letter or '_')")]
    VariableNameInvalidStart { found: char },

    #[error("variable '{name}' is used before its declaration")]
    VariableUsedBeforeDeclaration { name: Box<str> },

    #[error("variable '{name}' is undefined in scope")]
    UndefinedVariable { name: Box<str> },

    #[error("unknown intrinsic 'intrinsic.{path}'")]
    UnknownIntrinsic { path: Box<str> },

    #[error("'intrinsic.{path}' expects {expected} argument(s), got {got}")]
    IntrinsicArityMismatch {
        path: Box<str>,
        expected: usize,
        got: usize,
    },

    #[error("'{function_name}' not found in {location}")]
    FunctionNotFoundIn {
        function_name: Box<str>,
        location: Box<str>,
    },

    #[error("struct `{struct_name}` has no field `{field_name}`")]
    StructHasNoField {
        struct_name: Box<str>,
        field_name: Box<str>,
    },

    #[error("generic parameter `{generic_name}` inferred as both `{first}` and `{second}`")]
    GenericParameterConflict {
        generic_name: Box<str>,
        first: Box<str>,
        second: Box<str>,
    },

    #[error("field `{field_name}` type mismatch: expected `{expected}`, got `{got}`")]
    FieldTypeMismatch {
        field_name: Box<str>,
        expected: Box<str>,
        got: Box<str>,
    },

    #[error("type mismatch in binary expression: left is `{left}`, right is `{right}`")]
    BinaryExpressionTypeMismatch { left: Box<str>, right: Box<str> },

    #[error("argument type mismatch: expected `{expected}`, got `{got}`")]
    ArgumentTypeMismatch { expected: Box<str>, got: Box<str> },

    #[error("variant `{enum_name}.{variant_name}` expects {expected} argument(s), got {got}")]
    EnumVariantArityMismatch {
        enum_name: Box<str>,
        variant_name: Box<str>,
        expected: usize,
        got: usize,
    },

    #[error("{0}")]
    EnumVariantArgumentTypeMismatch(Box<EnumVariantArgumentTypeMismatch>),

    #[error("return type mismatch: expected `{expected}`, got nothing")]
    ReturnTypeMismatchMissing { expected: Box<str> },

    #[error("return type mismatch: expected `{expected}`, got `{got}`")]
    ReturnTypeMismatch { expected: Box<str>, got: Box<str> },

    #[error("cannot assign to an immutable variable")]
    AssignToImmutableVariable,

    #[error("assignment type mismatch: expected `{expected}`, got `{got}`")]
    AssignmentTypeMismatch { expected: Box<str>, got: Box<str> },

    #[error(transparent)]
    LexError(#[from] soul_tokenizer::fault::TokenErrorKind),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnumVariantArgumentTypeMismatch {
    pub enum_name: SharedStr,
    pub variant_name: SharedStr,
    pub expected: SoulType,
    pub got: SoulType,
}
impl Display for EnumVariantArgumentTypeMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format_args!(
            "variant `{}.{}` argument type mismatch: expected `{:?}`, got `{:?}`",
            self.enum_name, self.variant_name, self.expected, self.got,
        )
        .fmt(f)
    }
}
impl From<EnumVariantArgumentTypeMismatch> for AstErrorKind {
    fn from(value: EnumVariantArgumentTypeMismatch) -> Self {
        Self::EnumVariantArgumentTypeMismatch(Box::new(value))
    }
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
