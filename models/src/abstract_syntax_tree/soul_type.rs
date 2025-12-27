//! SoulType module
//!
//! This module defines the type system for the Soul language, including primitive types,
//! complex/named types (structs, classes, traits), generics, references, pointers, arrays,
//! optionals, tuples, and function types. It also supports type modifiers like `const` and
//! `literal`.
//!
//! Helpers are provided for checking modifiers, type categories, and displaying types.
use crate::{
    abstract_syntax_tree::{
        expression::Expression,
        statment::Ident,
        syntax_display::{DisplayKind, SyntaxDisplay},
    },
    error::Span,
    sementic_models::scope::NodeId,
    soul_names::{
        InternalComplexTypes, InternalPrimitiveTypes, StackArrayKind, TypeModifier, TypeWrapper,
    },
};

/// Represents a type in the Soul language.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SoulType {
    /// The kind of type (primitive, complex, array, etc.).
    pub kind: TypeKind,
    /// Optional type modifier (const, mut, literal).
    pub modifier: Option<TypeModifier>,
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
    InternalComplex(InternalComplexTypes),
    /// Primitive types like int, bool, float
    Primitive(InternalPrimitiveTypes),
    /// A struct type
    Struct(NodeId),
    /// A class type
    Class(NodeId),
    /// A trait type
    Trait(NodeId),
    /// A enum type
    Enum(NodeId),
    /// A union type
    Union(NodeId),
    /// Array type: [T; N] or dynamic
    Array(ArrayType),
    /// Tuple type: (T1, T2, ...)
    Tuple(TupleType),
    /// Named tuple / record type
    NamedTuple(NamedTupleType),
    /// Function type: (params) -> return
    Function(FunctionType),
    /// Generic type parameter
    Generic { node_id: NodeId, kind: GenericKind },
    /// Reference type: &T or &mut T
    Reference(ReferenceType),
    /// Pointer type: *T
    Pointer(Box<SoulType>),
    /// Optional type: T?
    Optional(Box<SoulType>),
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
    pub size: Option<StackArrayKind>,
}

/// Function type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionType {
    /// Parameter types.
    pub parameters: TupleType,
    /// Return type.
    pub return_type: Box<SoulType>,
    /// The kind of function (mut, const, consume).
    pub function_kind: FunctionKind,
    /// Type modifier for the function.
    pub modifier: TypeModifier,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FunctionKind {
    /// normal function
    Mut,
    /// functional function
    Const,
    /// compile time function
    Consume,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceType {
    /// The inner type being referenced.
    pub inner: Box<SoulType>,
    /// The lifetime identifier.
    pub lifetime: Ident,
    /// Whether the reference is mutable.
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GenericDeclare {
    pub node_id: Option<NodeId>,
    pub kind: GenericDeclareKind,
    pub span: Span,
}
impl GenericDeclare {
    pub fn new_lifetime(ident: Ident, span: Span) -> Self {
        Self {
            span,
            node_id: None,
            kind: GenericDeclareKind::Lifetime(ident),
        }
    }

    pub fn new_type(
        name: Ident,
        traits: Vec<SoulType>,
        default: Option<SoulType>,
        span: Span,
    ) -> Self {
        Self {
            span,
            node_id: None,
            kind: GenericDeclareKind::Type {
                name,
                traits,
                default,
            },
        }
    }

    pub fn new_expression(
        name: Ident,
        for_type: Option<SoulType>,
        default: Option<Expression>,
        span: Span,
    ) -> Self {
        Self {
            span,
            node_id: None,
            kind: GenericDeclareKind::Expression {
                name,
                for_type,
                default,
            },
        }
    }
}

/// A generic parameter (lifetime or type).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GenericDeclareKind {
    /// A lifetime parameter.
    Lifetime(Ident),
    /// A type parameter.
    Type {
        name: Ident,
        traits: Vec<SoulType>,
        default: Option<SoulType>,
    },
    /// A type parameter.
    Expression {
        name: Ident,
        for_type: Option<SoulType>,
        default: Option<Expression>,
    },
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
pub struct TupleType {
    pub types: Vec<SoulType>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedTupleType {
    pub types: Vec<(Ident, SoulType, Option<NodeId>)>,
}

impl SoulType {
    /// Creates a new `SoulType` with the given modifier and kind.
    pub const fn new(modifier: Option<TypeModifier>, kind: TypeKind, span: Span) -> Self {
        Self {
            span,
            kind,
            modifier,
            generics: vec![],
        }
    }

    /// Creates a `Stub` type (parser unknown type).
    pub const fn new_stub(ident: Ident, span: Span) -> Self {
        Self::new(
            None,
            TypeKind::Stub {
                ident,
                resolved: None,
            },
            span,
        )
    }

    /// Creates a `none` type (empty type).
    pub const fn none(span: Span) -> Self {
        Self {
            span,
            kind: TypeKind::None,
            modifier: None,
            generics: vec![],
        }
    }

    /// Returns whether this type is optional (`T?`).
    pub fn is_optional(&self) -> bool {
        matches!(self.kind, TypeKind::Optional(_))
    }
    /// Returns whether this type is a reference (`&T` or `@T`).
    pub fn is_reference(&self) -> bool {
        matches!(self.kind, TypeKind::Reference(_))
    }
    /// Returns whether this type is a pointer (`*T`).
    pub fn is_pointer(&self) -> bool {
        matches!(self.kind, TypeKind::Pointer(_))
    }
    /// Returns whether this type is a complex/named type.
    pub fn is_complex(&self) -> bool {
        matches!(
            self.kind,
            TypeKind::Struct(_)
                | TypeKind::Class(_)
                | TypeKind::Trait(_)
                | TypeKind::Enum(_)
                | TypeKind::Union(_)
        )
    }
    /// Returns whether this type is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(self.kind, TypeKind::InternalComplex(_))
    }
}

impl SyntaxDisplay for SoulType {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, false);
        sb
    }

    fn inner_display(&self, sb: &mut String, _kind: DisplayKind, _tab: usize, _is_last: bool) {
        if let Some(modifier) = self.modifier {
            sb.push_str(modifier.as_str());
            sb.push(' ');
        }

        sb.push_str(&self.kind.display());
    }
}

impl TypeKind {
    /// Returns a string representation of the type kind
    pub fn display(&self) -> String {
        let kind = DisplayKind::Parser;
        match self {
            TypeKind::Type => "Type".to_string(),
            TypeKind::InternalComplex(p) => p.as_str().to_string(),
            TypeKind::Struct(id) => id.display(),
            TypeKind::Class(id) => id.display(),
            TypeKind::Enum(id) => id.display(),
            TypeKind::Union(id) => id.display(),
            TypeKind::Trait(id) => id.display(),
            TypeKind::Array(a) => {
                let inner = a.of_type.display(kind);
                match &a.size {
                    Some(StackArrayKind::Number(num)) => format!("[{}]{}", num, inner),
                    Some(StackArrayKind::Ident { ident, resolved: _ }) => {
                        format!("[{}]{}", ident.as_str(), inner)
                    }
                    None => format!("[{}]", inner),
                }
            }
            TypeKind::Tuple(tuple) => {
                let elems: Vec<String> = tuple.types.iter().map(|t| t.display(kind)).collect();
                format!("({})", elems.join(", "))
            }
            TypeKind::NamedTuple(map) => {
                let elems: Vec<String> = map
                    .types
                    .iter()
                    .map(|(k, v, _)| format!("{}: {}", k.as_str(), v.display(kind)))
                    .collect();
                format!("{{{}}}", elems.join(", "))
            }
            TypeKind::Function(f) => {
                let params: Vec<String> =
                    f.parameters.types.iter().map(|p| p.display(kind)).collect();
                format!(
                    "fn({}) -> {}",
                    params.join(", "),
                    f.return_type.display(kind)
                )
            }
            TypeKind::Generic { node_id, .. } => node_id.display(),
            TypeKind::Reference(r) => {
                let ref_str = if r.mutable {
                    TypeWrapper::MutRef.as_str()
                } else {
                    TypeWrapper::ConstRef.as_str()
                };
                format!("{}{}", ref_str, r.inner.display(kind))
            }
            TypeKind::Pointer(inner) => format!("*{}", inner.display(kind)),
            TypeKind::Optional(inner) => format!("{}?", inner.display(kind)),
            TypeKind::Stub { ident, .. } => ident.as_str().to_string(),
            TypeKind::None => "none".to_string(),
            TypeKind::Primitive(internal_primitive_types) => {
                internal_primitive_types.as_str().to_string()
            }
        }
    }
}
