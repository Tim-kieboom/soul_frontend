use parser_models::{ast::FunctionKind, scope::NodeId};
use soul_utils::{
    Ident,
    soul_import_path::SoulImportPath,
    soul_names::TypeModifier,
    span::{NodeMetaData, Span, Spanned},
    vec_map::VecMap,
};

use crate::{
    BodyId, NamedTupleType, StatementId, Variable, hir_type::HirType, scope::ScopeId,
    statement::Statement,
};

pub type Item = Spanned<ItemKind>;

/// Top-level items in a Soul module.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ItemKind {
    /// Module import (`import path::to::module`).
    Import(Import),
    /// Function declaration with body.
    Function(Box<Function>),
    /// global variable
    Variable(Variable),
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
    pub parameters: NamedTupleType,
    pub vis: Visibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Visibility {
    Public,
    Private,
}

impl Visibility {
    pub fn display_variant(&self) -> &'static str {
        match self {
            Visibility::Public => "pub",
            Visibility::Private => "priv",
        }
    }
    pub fn from_name(name: &Ident) -> Self {
        let first = match name.as_str().chars().next() {
            Some(val) => val,
            None => return Self::Private,
        };

        if first.is_uppercase() {
            Self::Public
        } else {
            Self::Private
        }
    }
}

pub trait ItemHelper {
    fn new_variable(variable: Variable, meta: NodeMetaData) -> Item;
    fn new_function(function: Function, meta: NodeMetaData) -> Item;
    fn new_import(import: Import, span: Span) -> Item;
}
impl ItemHelper for Item {
    fn new_variable(variable: Variable, meta: NodeMetaData) -> Item {
        Item::with_meta_data(ItemKind::Variable(variable), meta)
    }

    fn new_import(import: Import, span: Span) -> Item {
        Item::new(ItemKind::Import(import), span)
    }

    fn new_function(function: Function, meta: NodeMetaData) -> Item {
        Item::with_meta_data(ItemKind::Function(Box::new(function)), meta)
    }
}
