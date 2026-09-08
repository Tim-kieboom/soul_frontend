use soul_utils::fault::{Fault, UnclassifiedKind};

/// Structured error kinds for AST-to-MIR lowering. `Unclassified` is a migration
/// fallback carrying the raw message from call sites not yet converted to a real
/// variant.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum MirErrorKind {
    #[error("{0}")]
    Unclassified(Box<str>),

    #[error("extern/signature-only declarations have no body to lower to MIR")]
    SignatureOnlyFunctionHasNoBody,

    #[error(
        "only simple (non-destructuring) variable bindings are supported in this lowering slice"
    )]
    NonSimpleVariablePatternUnsupported,

    #[error("a variable declaration with no initializer isn't supported in this lowering slice")]
    UninitializedVariableUnsupported,

    #[error("variable has no resolved type")]
    VariableHasNoResolvedType,

    #[error(
        "only a `return <expr>` statement is supported as a function's terminal statement in this lowering slice"
    )]
    NonReturnTerminalStatementUnsupported,

    #[error("this statement kind isn't supported in this lowering slice")]
    UnsupportedStatementKind,

    #[error("function has no `return <expr>` as its final reachable statement")]
    MissingReturnStatement,

    #[error("type `{ty}` isn't a primitive scalar, which is all this lowering slice supports")]
    NonPrimitiveType { ty: Box<str> },

    #[error("only arithmetic binary operators (+ - * / %) are supported in this lowering slice")]
    UnsupportedBinaryOperator,

    #[error("variable has no resolved binding")]
    VariableHasNoResolvedBinding,

    #[error("variable isn't bound to a local in this function's lowered scope")]
    VariableNotBoundToLocal,

    #[error("nested expression has no resolved type")]
    NestedExpressionHasNoResolvedType,

    #[error(
        "only literals, variables, and arithmetic binary expressions are supported as operands in this lowering slice"
    )]
    UnsupportedOperandExpression,

    #[error(
        "only a bare `bool` literal is supported as an `if`/`while` condition in this lowering slice"
    )]
    UnsupportedConditionExpression,

    #[error(
        "only `while <cond> {{ .. }}` loops are supported in this lowering slice, not bare `for` loops or `foreach`"
    )]
    UnsupportedLoopCondition,

    #[error("`break` outside of a loop")]
    BreakOutsideLoop,

    #[error("`continue` outside of a loop")]
    ContinueOutsideLoop,

    #[error("statement is unreachable: every preceding path already returned, broke, or continued")]
    UnreachableStatement,
}

impl From<UnclassifiedKind> for MirErrorKind {
    fn from(value: UnclassifiedKind) -> Self {
        MirErrorKind::Unclassified(value.0)
    }
}
impl From<MirErrorKind> for UnclassifiedKind {
    fn from(value: MirErrorKind) -> Self {
        UnclassifiedKind(value.to_string().into_boxed_str())
    }
}

pub type MirFault = Fault<MirErrorKind>;
pub type MirResult<T> = std::result::Result<T, MirFault>;
