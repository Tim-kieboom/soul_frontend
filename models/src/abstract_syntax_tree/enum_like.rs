use std::collections::{BTreeMap};
use crate::{abstract_syntax_tree::{expression::Expression, soul_type::{GenericDeclare, SoulType}, spanned::Spanned, statment::Ident}, scope::scope::ScopeId};

/// A C-like enum definition (enumeration with integer values).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Enum {
    /// The name of the enum.
    pub name: Ident,
    /// The scope identifier for this enum.
    pub scope_id: ScopeId,
    /// The enum variants.
    pub variants: Vec<EnumVariant>,
}

/// A Rust-like union/enum definition (sum type with data).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Union {
    /// The name of the union.
    pub name: Ident,
    /// Generic type parameters.
    pub generics: Vec<GenericDeclare>,
    /// The union variants.
    pub variants: Vec<Spanned<UnionVariant>>,
    /// The scope identifier for this union.
    pub scope_id: ScopeId,
}

/// A variant of a union type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnionVariant {
    /// The name of the variant.
    pub name: Ident,
    /// The kind of data this variant holds.
    pub field: UnionVariantKind,
}

/// The kind of data a union variant can hold.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum UnionVariantKind {
    /// A tuple variant with positional fields.
    Tuple(Vec<SoulType>),
    /// A named tuple variant with named fields.
    NamedTuple(BTreeMap<Ident, SoulType>)
}

/// A variant of a C-like enum.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnumVariant {
    /// The name of the variant.
    pub name: Ident,
    /// The integer value or expression for this variant.
    pub value: EnumVariantsKind,
}

/// The value kind for an enum variant.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EnumVariantsKind {
    /// An integer value.
    Int(i64),
    /// An expression that evaluates to the variant's value.
    Expression(Expression),
}

