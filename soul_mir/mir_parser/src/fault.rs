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

    #[error(
        "only arithmetic (+ - * / %), comparison (== != < > <= >=), and logical (&& ||) binary operators are supported in this lowering slice"
    )]
    UnsupportedBinaryOperator,

    #[error("only the `!` (logical not) unary operator is supported in this lowering slice")]
    UnsupportedUnaryOperator,

    #[error("variable has no resolved binding")]
    VariableHasNoResolvedBinding,

    #[error("variable isn't bound to a local in this function's lowered scope")]
    VariableNotBoundToLocal,

    #[error("nested expression has no resolved type")]
    NestedExpressionHasNoResolvedType,

    #[error(
        "only literals, variables, arithmetic/comparison/logical binary expressions, `!`, and calls are supported as operands in this lowering slice"
    )]
    UnsupportedOperandExpression,

    #[error(
        "only a bare `bool` literal/variable, `!<bool>`, or a comparison/logical (`&&`/`||`) expression is supported as an `if`/`while` condition in this lowering slice"
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

    #[error(
        "only a bare (already-declared) variable or a struct field (`variable.field`) is supported as an assignment target in this lowering slice"
    )]
    AssignmentTargetUnsupported,

    #[error("function call has no resolved target")]
    FunctionCallHasNoResolvedTarget,

    #[error(
        "only a plain `name(args...)` free-function call is supported in this lowering slice — no method-call receiver, generics, named arguments, or `defer`"
    )]
    UnsupportedCallShape,

    #[error("a `none`-returning call's result can't be used as a value")]
    CannotUseNoneValueAsOperand,

    #[error("`return <expr>` isn't valid in a `none`-returning function")]
    UnexpectedReturnValue,

    #[error("intrinsic `{name}` isn't supported in this lowering slice")]
    UnsupportedIntrinsic { name: Box<str> },

    /// The resolver logs a fault on an intrinsic arity mismatch but still
    /// stores the resolution and lets the call through — so a malformed
    /// `assert()`/`panic()` call can genuinely reach MIR lowering with the
    /// wrong argument count. This is the graceful fault for that, not a
    /// defensive/unreachable one: guard the argument index with it instead
    /// of indexing `call.arguments` directly.
    #[error("intrinsic `{name}` expects {expected} argument(s), got {got}")]
    IntrinsicArityMismatch {
        name: Box<str>,
        expected: usize,
        got: usize,
    },

    #[error(
        "only field access on a variable or another field access — `variable.field` or `object.field.field` — is supported in this lowering slice"
    )]
    UnsupportedFieldAccessObject,

    #[error("`..` default-filled struct constructors aren't supported in this lowering slice")]
    StructConstructorDefaultsUnsupported,

    /// The resolver already rejects a struct constructor missing a field
    /// value, or a field-access naming a field the struct doesn't have
    /// (`StructHasNoField`), before this ever runs — reaching this means the
    /// resolver let bad input through, same defensive category as
    /// `UnexpectedReturnValue`.
    #[error("struct `{struct_name}` has no field `{field}`")]
    StructFieldNotFound {
        struct_name: Box<str>,
        field: Box<str>,
    },
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
