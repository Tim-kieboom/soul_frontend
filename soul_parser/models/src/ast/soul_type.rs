use soul_utils::{
    Ident,
    soul_names::{InternalPrimitiveTypes, TypeModifier},
    span::Span,
};

use crate::ast::Expression;
use crate::scope::NodeId;

/// Represents a type in the Soul language.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SoulType {
    /// The kind of type (primitive, complex, array, etc.).
    pub kind: TypeKind,
    /// Optional type modifier (const, mut, literal).
    pub modifier: TypeModifier,
    pub generics: Vec<GenericDefine>,
    pub span: Span,
}

/// The specific kind of a type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TypeKind {
    /// empty type
    None,
    /// Represents the type of all types
    Type,
    /// stub type (used in parsing stage)
    Stub {
        ident: Ident,
        resolved: Option<NodeId>,
    },
    /// Primitive types like int, bool, float
    Primitive(InternalPrimitiveTypes),
    /// Array type: [N]T or dynamic []T
    Array(ArrayType),
    /// Tuple type: (T1, T2, ...)
    Tuple(TupleType),
    /// Named tuple / record type
    NamedTuple(NamedTupleType),
    /// Generic type parameter
    Generic { node_id: Option<NodeId>, kind: GenericKind },
    /// Reference type: &T or &mut T
    Reference(ReferenceType),
    /// Pointer type: *T
    Pointer(Box<SoulType>),
    /// Optional type: ?T
    Optional(Box<SoulType>),
}

/// A generic argument (type, lifetime, or expression).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GenericDefine {
    /// A type argument.
    Type(SoulType),
    /// A lifetime argument.
    Lifetime(Ident),
    /// An expression argument (for const generics).
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GenericKind {
    Type,
    LifeTime,
    Expression,
}

/// Array type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArrayType {
    /// The element type of the array.
    pub of_type: Box<SoulType>,
    /// Compile-time size, or `None` for dynamic arrays.
    pub size: Option<u64>,
}

pub type TupleType = Vec<SoulType>;
pub type NamedTupleType = Vec<(Ident, SoulType, Option<NodeId>)>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceType {
    /// The inner type being referenced.
    pub inner: Box<SoulType>,
    /// The lifetime identifier.
    pub lifetime: Option<Ident>,
    /// Whether the reference is mutable.
    pub mutable: bool,
}

impl ArrayType {
    pub fn new(ty: SoulType, size: Option<u64>) -> Self {
        Self { 
            of_type: Box::new(ty), 
            size,
        }
    }
}

impl ReferenceType {
    pub fn new(ty: SoulType, mutable: bool) -> Self {
        Self {
            inner: Box::new(ty),
            lifetime: None,
            mutable,
        }
    }

    pub fn new_lifetime(ty: SoulType, lifetime: Option<Ident>, mutable: bool) -> Self {
        Self {
            inner: Box::new(ty),
            lifetime,
            mutable,
        }
    }
}

impl SoulType {
    pub fn new(modifier: TypeModifier, kind: TypeKind, span: Span) -> Self {
        Self { kind, modifier, generics: vec![], span }
    }

    pub fn none(span: Span) -> Self {
        Self {
            span,
            generics: vec![],
            kind: TypeKind::None,
            modifier: TypeModifier::Const,
        }
    }

    pub fn none_mut(span: Span) -> Self {
        Self {
            span,
            generics: vec![],
            kind: TypeKind::None,
            modifier: TypeModifier::Mut,
        }
    }
}
