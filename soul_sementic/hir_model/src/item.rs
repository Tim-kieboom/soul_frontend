use parser_models::{ast::FunctionKind, scope::NodeId};
use soul_utils::{
    Ident, soul_import_path::SoulImportPath, soul_names::TypeModifier, vec_map::VecMap,
};

use crate::{
    BodyId, ExpressionId, StatementId, hir_type::HirType, scope::ScopeId, statement::Statement,
};

/// Top-level items in a Soul module.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Item {
    /// Module import (`import path::to::module`).
    Import(Import),
    /// Function declaration with body.
    Function(Box<Function>),
}

/// Block of statements with associated scope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: NodeId,
    pub scope_id: ScopeId,
    pub modifier: TypeModifier,
    pub statements: VecMap<StatementId, Statement>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Import {
    pub id: NodeId,
    pub paths: Vec<SoulImportPath>,
}

/// Function item in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    pub id: NodeId,
    pub body: BodyId,
    pub signature: FunctionSignature,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionSignature {
    pub name: Ident,
    pub methode_type: HirType,
    pub function_kind: FunctionKind,
    pub return_type: HirType,
    pub parameters: Vec<Parameter>,
    pub generics: Vec<GenericDeclare>,
    pub vis: Visibility,
}

/// Function parameter.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Parameter {
    pub id: NodeId,
    pub name: Ident,
    pub ty: HirType,
}

/// A generic parameter (lifetime or type).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GenericDeclare {
    /// A lifetime parameter.
    Lifetime(Ident),
    /// A type parameter.
    Type {
        name: Ident,
        traits: Vec<HirType>,
        default: Option<HirType>,
    },
    /// A type parameter.
    Expression {
        name: Ident,
        for_type: Option<HirType>,
        default: Option<ExpressionId>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Visibility {
    Public,
    Private,
}
