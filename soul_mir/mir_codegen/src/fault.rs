use soul_utils::{
    FunctionId,
    fault::{Fault, UnclassifiedKind},
};

/// Structured error kinds for MIR-to-LLVM codegen. `Unclassified` is a
/// migration fallback carrying the raw message from call sites not yet
/// converted to a real variant.
///
/// Most variants carry `span: None` — unlike the AST/MIR-lowering stages,
/// `mir_model`'s `Statement`/`Rvalue`/`Terminator` don't carry spans at all
/// (only `LocalDecl` does), so a real span is only available for the local/
/// parameter/return-type checks; everything else is a structural limitation
/// of the MIR shape, not something codegen can paper over.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum CodegenErrorKind {
    #[error("{0}")]
    Unclassified(Box<str>),

    #[error("{id:?} has no AST entry")]
    MissingAstEntry { id: FunctionId },

    #[error("type `{ty}` isn't a primitive scalar, which is all this codegen slice supports")]
    NonPrimitiveType { ty: Box<str> },

    #[error("type `{ty}` isn't supported in this codegen slice")]
    UnsupportedPrimitiveType { ty: Box<str> },

    #[error("place projections (field/index/deref) aren't supported in this codegen slice")]
    PlaceProjectionUnsupported,

    #[error(
        "switchInt with more than one target isn't supported in this codegen slice (only bool if/while conditions are constructed today)"
    )]
    SwitchIntTargetCountUnsupported,

    #[error("switchInt target value must be a bool constant in this codegen slice")]
    SwitchIntTargetValueUnsupported,

    #[error("call to {id:?} has no MIR body")]
    CallHasNoMirBody { id: FunctionId },

    #[error("call to {id:?} was never declared")]
    CallNeverDeclared { id: FunctionId },

    #[error("call to {id:?} passes more arguments than it has parameters")]
    CallArgumentCountMismatch { id: FunctionId },

    #[error("call result used but callee returns `none`")]
    CallResultIsNone,

    /// Arithmetic/comparison/logical operators are int-only in this slice —
    /// checked explicitly here rather than trusting the resolver to have
    /// already ruled a pointer out, since `into_int_value()`-style unwraps
    /// would otherwise panic on real (if malformed) input instead of
    /// faulting cleanly.
    #[error("expected an integer operand, got a pointer")]
    ExpectedIntOperand,

    #[error("expected a float operand, got an int or a pointer")]
    ExpectedFloatOperand,

    #[error("expected a pointer operand, got an integer")]
    ExpectedPointerOperand,

    #[error("unary operator `{op}` isn't supported in this codegen slice")]
    UnsupportedUnaryOperator { op: Box<str> },

    #[error("binary operator `{op}` isn't supported in this codegen slice")]
    UnsupportedBinaryOperator { op: Box<str> },

    #[error("references/aggregates/casts aren't supported in this codegen slice")]
    UnsupportedRvalue,

    #[error("constant `{value}` isn't supported in this codegen slice")]
    UnsupportedConstant { value: Box<str> },

    #[error("`Drop` isn't constructed by lowering yet and isn't supported in codegen either")]
    DropUnsupported,

    #[error("LLVM intrinsic `{name}` isn't available in this build of LLVM")]
    OverflowIntrinsicUnavailable { name: Box<str> },

    #[error("function has no blocks")]
    FunctionHasNoBlocks,

    #[error("function is missing parameter {index}")]
    MissingParameterValue { index: usize },

    #[error("`main`'s return type is wider than the 32-bit process exit code convention supports")]
    EntryPointReturnTypeTooWide,

    /// An LLVM builder call failed — this is an internal-consistency error
    /// (this codegen pass building invalid IR), not a "not supported yet"
    /// rejection of the input.
    #[error("LLVM builder operation failed: {message}")]
    LlvmBuilderError { message: Box<str> },
}

impl From<UnclassifiedKind> for CodegenErrorKind {
    fn from(value: UnclassifiedKind) -> Self {
        CodegenErrorKind::Unclassified(value.0)
    }
}
impl From<CodegenErrorKind> for UnclassifiedKind {
    fn from(value: CodegenErrorKind) -> Self {
        UnclassifiedKind(value.to_string().into_boxed_str())
    }
}

pub type CodegenFault = Fault<CodegenErrorKind>;
pub type CodegenResult<T> = std::result::Result<T, CodegenFault>;
