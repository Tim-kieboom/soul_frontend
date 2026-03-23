use ast::{ExternLanguage, FunctionKind};
use soul_utils::{Ident, ids::FunctionId};

use crate::{BlockId, ExpressionId, GenericId, LocalId, TypeId};

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

    pub generics: Vec<GenericId>,

    /// Return type of the function.
    pub return_type: TypeId,

    /// Body of the function.
    pub body: FunctionBody,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FunctionBody {
    Internal(BlockId),
    External(ExternLanguage),
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

    pub default: Option<ExpressionId>,
}
