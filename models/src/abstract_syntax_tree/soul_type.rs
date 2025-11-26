//! SoulType module
//!
//! This module defines the type system for the Soul language, including primitive types,
//! complex/named types (structs, classes, traits), generics, references, pointers, arrays,
//! optionals, tuples, and function types. It also supports type modifiers like `const` and
//! `literal`.
//!
//! Helpers are provided for checking modifiers, type categories, and displaying types.
use crate::{abstract_syntax_tree::{expression::Expression, statment::Ident, syntax_display::SyntaxDisplay}, soul_names::{InternalComplexTypes, InternalPrimitiveTypes, TypeModifier}};


/// Represents a type in the Soul language.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SoulType {
    /// The kind of type (primitive, complex, array, etc.).
    pub kind: TypeKind,
    /// Optional type modifier (const, mut, literal).
    pub modifier: Option<TypeModifier>,

    pub generics: Vec<TypeGeneric>,
}


/// The specific kind of a type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TypeKind {
    /// empty type
    None,
    Unknown(Ident),
    /// Primitive types like int, bool, float
    InternalComplex(InternalComplexTypes),
    /// Primitive types like int, bool, float
    Primitive(InternalPrimitiveTypes),
    /// Named complex types like structs, classes, traits, enums
    Complex(Ident),
    /// Array type: [T; N] or dynamic
    Array(ArrayType),
    /// Tuple type: (T1, T2, ...)
    Tuple(TupleType),
    /// Named tuple / record type
    NamedTuple(NamedTupleType),
    /// Function type: (params) -> return
    Function(FunctionType),
    /// Generic type parameter
    Generic(Ident),
    /// Reference type: &T or &mut T
    Reference(ReferenceType),
    /// Pointer type: *T
    Pointer(Box<SoulType>),
    /// Optional type: T?
    Optional(Box<SoulType>),
}

/// Array type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArrayType {
    /// The element type of the array.
    pub of_type: Box<SoulType>,
    /// Compile-time size, or `None` for dynamic arrays.
    pub size: Option<usize>,
}


/// Function type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionType {
    /// Parameter types.
    pub parameters: Vec<SoulType>,
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

/// A generic parameter (lifetime or type).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GenericParameter {
    /// A lifetime parameter.
    Lifetime(Ident),
    /// A type parameter.
    Type(SoulType),
}

/// A generic argument (type, lifetime, or expression).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TypeGeneric {
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
    pub types: Vec<(Ident, SoulType)>,
}

impl SoulType {
    /// Creates a new `SoulType` with the given modifier and kind.
    pub const fn new(modifier: Option<TypeModifier>, kind: TypeKind) -> Self {
        Self {
            kind,
            modifier,
            generics: vec![],
        }
    }

    /// Creates a `none` type (empty type).
    pub const fn none() -> Self {
        Self{kind: TypeKind::None, modifier: None, generics: vec![]}
    } 

    /// Returns whether this type is optional (`T?`).
    pub fn is_optional(&self) -> bool { matches!(self.kind, TypeKind::Optional(_)) }
    /// Returns whether this type is a reference (`&T` or `@T`).
    pub fn is_reference(&self) -> bool { matches!(self.kind, TypeKind::Reference(_)) }
    /// Returns whether this type is a pointer (`*T`).
    pub fn is_pointer(&self) -> bool { matches!(self.kind, TypeKind::Pointer(_)) }
    /// Returns whether this type is a complex/named type.
    pub fn is_complex(&self) -> bool { matches!(self.kind, TypeKind::Complex(_)) }
    /// Returns whether this type is a primitive type.
    pub fn is_primitive(&self) -> bool { matches!(self.kind, TypeKind::InternalComplex(_)) }
}

impl SyntaxDisplay for SoulType {
    fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, 0, false);
        sb
    }

    fn inner_display(&self, sb: &mut String, _tab: usize, _is_last: bool) {
        
        if let Some(modifier) = self.modifier {
            sb.push_str(modifier.as_str());
        }

        sb.push(' ');
        sb.push_str(&self.kind.display());
    }
}

impl TypeKind {
    /// Returns a string representation of the type kind
    pub fn display(&self) -> String {
        match self {
            TypeKind::InternalComplex(p) => p.as_str().to_string(),
            TypeKind::Complex(c) => c.clone(),
            TypeKind::Array(a) => {
                let inner = a.of_type.display();
                match a.size {
                    Some(size) => format!("[{}; {}]", inner, size),
                    None => format!("[{}]", inner),
                }
            }
            TypeKind::Tuple(tuple) => {
                let elems: Vec<String> = tuple.types.iter().map(|t| t.display()).collect();
                format!("({})", elems.join(", "))
            }
            TypeKind::NamedTuple(map) => {
                let elems: Vec<String> = map.types.iter().map(|(k,v)| format!("{}: {}", k, v.display())).collect();
                format!("{{{}}}", elems.join(", "))
            }
            TypeKind::Function(f) => {
                let params: Vec<String> = f.parameters.iter().map(|p| p.display()).collect();
                format!("fn({}) -> {}", params.join(", "), f.return_type.display())
            }
            TypeKind::Generic(ident) => ident.clone(),
            TypeKind::Reference(r) => {
                let mutability = if r.mutable { "mut " } else { "" };
                format!("&{}{}", mutability, r.inner.display())
            }
            TypeKind::Pointer(inner) => format!("*{}", inner.display()),
            TypeKind::Optional(inner) => format!("{}?", inner.display()),
            TypeKind::Unknown(ident) => ident.clone(),
            TypeKind::None => "none".to_string(),
            TypeKind::Primitive(internal_primitive_types) => internal_primitive_types.as_str().to_string(),
        }
    }
}

