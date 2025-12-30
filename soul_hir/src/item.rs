use soul_ast::{
    abstract_syntax_tree::statment::Ident, sementic_models::scope::ScopeId,
    soul_names::TypeModifier, soul_page_path::SoulPagePath,
};

use crate::{ExpressionId, HirBlockId, HirBodyId, HirId, StatementId, hir_type::HirType};

/// Top-level items in a Soul module.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Item {
    /// Module import (`import path::to::module`).
    Import(Import),
    /// Function declaration with body.
    Function(Function),
    /// Scoped `use` block (Soul equivalent of Rust `impl`).
    UseBlock(UseBlock),
    /// Struct declaration (includes desugared Soul `class`).
    Struct(Struct),
    /// Trait declaration (a Rust-like interface).
    Trait(Trait),
    /// Union type declaration (Rust-like enum).
    Union(Union),
    /// C-style enum declaration.
    Enum(Enum),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Import {
    pub id: HirId,
    pub paths: Vec<SoulPagePath>,
}

/// Function item in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    pub id: HirId,
    pub name: Ident,
    pub body: HirBodyId,
    pub params: Vec<Parameter>,
    pub modifier: TypeModifier,
    pub return_type: HirType,
    pub generics: Vec<GenericDeclare>,
}

/// Function parameter.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Parameter {
    pub id: HirId,
    pub name: Ident,
    pub ty: HirType,
}

/// `use` block item (Soul's scoped extension mechanism).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UseBlock {
    pub id: HirId,
    pub ty: HirType,
    pub block: HirBlockId,
    /// Optional trait being implemented.
    pub impl_trait: Option<HirType>,
}

/// Block of statements with associated scope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: HirId,
    pub scope_id: ScopeId,
    pub modifier: TypeModifier,
    pub statements: Vec<StatementId>,
}

/// Struct declaration (includes desugared Soul `class`es).
///
/// Soul `class`es are desugared into plain structs with fields, with methods
/// converted to extension functions on `use` blocks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub id: HirId,
    pub name: Ident,
    pub fields: Vec<HirId>,
    pub generics: Vec<GenericDeclare>,
}

/// Trait declaration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Trait {
    pub id: HirId,
    pub name: Ident,
    pub methodes: Vec<HirId>,
    pub generics: Vec<GenericDeclare>,
}

/// Union type declaration (tagged enum, Rust-like).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Union {
    pub id: HirId,
    pub name: Ident,
    pub variants: Vec<UnionVariant>,
    pub generics: Vec<GenericDeclare>,
}

/// Union variant (tuple or named fields).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum UnionVariant {
    /// Tuple variant (unnamed fields `Variant(Type, Type)`).
    Tuple(Vec<HirType>),
    /// NamedTuple variant (unnamed fields `Variant{name1: Type, name2: Type}`).
    Named(Vec<(Ident, HirType)>),
}

/// C-style enum declaration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Enum {
    pub id: HirId,
    pub name: Ident,
    pub variants: EnumVariants,
    pub generics: Vec<GenericDeclare>,
}

/// Enum variants (units or associated expressions).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EnumVariants {
    /// Unit variant with discriminant.
    Ids(Vec<u64>),
    /// Variant with associated expression value.
    Expressions(Vec<ExpressionId>),
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

impl Item {
    pub fn get_id(&self) -> HirId {
        match self {
            Item::Enum(val) => val.id,
            Item::Union(val) => val.id,
            Item::Trait(val) => val.id,
            Item::Import(val) => val.id,
            Item::Struct(val) => val.id,
            Item::Function(val) => val.id,
            Item::UseBlock(val) => val.id,
        }
    }
}