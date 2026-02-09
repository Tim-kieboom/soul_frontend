use soul_utils::Ident;

use crate::{Block, FunctionId, LocalId, TypeId};

/// A function definition in HIR.
///
/// Functions are fully resolved and typed. Parameter names and
/// types have already been checked, and the function body is lowered
/// into HIR blocks and statements.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    /// Unique identifier of the function.
    pub id: FunctionId,

    /// Source-level name of the function.
    pub name: Ident,

    /// Function calling convention / `this` semantics.
    pub kind: FunctionKind,

    /// Function parameters.
    pub parameters: Vec<Parameter>,

    /// Return type of the function.
    pub return_type: TypeId,

    /// Body of the function.
    pub body: Block,
}

/// A function parameter.
///
/// Parameters are represented as locals with an associated type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Parameter {
    /// Local variable ID bound to this parameter.
    pub local: LocalId,

    /// Type of the parameter.
    pub ty: TypeId,
}

/// Describes how a function receives its implicit `this` parameter.
///
/// This controls ownership and mutability semantics for method calls.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FunctionKind {
    /// `&this`
    MutRef,
    /// ``
    Static,
    /// `this`
    Consume,
    /// `@this`
    ConstRef,
}
impl FunctionKind {
    pub fn display(&self) -> Option<&'static str> {
        match self {
            FunctionKind::Static => None,
            FunctionKind::MutRef => Some("&this"),
            FunctionKind::Consume => Some("this"),
            FunctionKind::ConstRef => Some("@this"),
        }
    }
}
