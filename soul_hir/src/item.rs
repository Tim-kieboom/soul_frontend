use soul_ast::abstract_syntax_tree::{Spanned, ThisCallee, Visibility, statment::Ident};
use soul_utils::{SoulPagePath, VecMap, soul_names::TypeModifier};

use crate::{
    ExpressionId, HirBlockId, HirBodyId, HirId, ScopeId, Statement, StatementId, hir_type::HirType,
};

/// Top-level items in a Soul module.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Item {
    /// Module import (`import path::to::module`).
    Import(Import),
    /// Function declaration with body.
    Function(Function),
    Contructor(Contructor),
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
    pub body: HirBodyId,
    pub signature: FunctionSignature,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Contructor {
    pub id: HirId,
    pub ty: HirType,
    pub vis: Visibility,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionSignature {
    pub name: Ident,
    pub callee: Option<Spanned<FunctionCallee>>,
    pub return_type: HirType,
    pub modifier: TypeModifier,
    pub parameters: Vec<Parameter>,
    pub generics: Vec<GenericDeclare>,
    pub vis: Visibility,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCallee {
    pub ty: HirType,
    pub this: ThisCallee,
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
    pub statements: VecMap<StatementId, Statement>,
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
    pub vis: Visibility,
}

/// Trait declaration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Trait {
    pub id: HirId,
    pub name: Ident,
    pub methodes: Vec<TraitMethode>,
    pub generics: Vec<GenericDeclare>,
    pub vis: Visibility,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraitMethode {
    pub id: HirId,
    pub signature: FunctionSignature,
    pub vis: Visibility,
}

/// Union type declaration (tagged enum, Rust-like).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Union {
    pub id: HirId,
    pub name: Ident,
    pub variants: Vec<UnionVariant>,
    pub generics: Vec<GenericDeclare>,
    pub vis: Visibility,
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
    pub vis: Visibility,
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
            Item::Contructor(contructor) => contructor.id,
        }
    }
}
