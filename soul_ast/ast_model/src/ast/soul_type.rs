use soul_utils::{
    Ident,
    soul_names::{InternalPrimitiveTypes, TypeModifier},
    span::Span,
};

use crate::scope::NodeId;

/// Represents a type in the Soul language.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SoulType {
    /// The kind of type (primitive, complex, array, etc.).
    pub kind: TypeKind,
    /// Optional type modifier (const, mut, literal).
    pub modifier: Option<TypeModifier>,
    pub span: Span,
}

/// The specific kind of a type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TypeKind {
    /// empty type
    None,
    /// Represents the type of all types
    Type,
    /// Primitive types like int, bool, float
    Primitive(InternalPrimitiveTypes),
    Array(ArrayType),
    /// Reference type: &T or &mut T
    Reference(ReferenceType),
    /// Pointer type: *T
    Pointer(Box<SoulType>),
    /// Optional type: ?T
    Optional(Box<SoulType>),
}

/// Array type
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArrayType {
    /// The element type of the array.
    pub of_type: Box<SoulType>,
    /// Compile-time size, or `None` for dynamic arrays.
    pub kind: ArrayKind,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArrayKind {
    /// stackArray `[2]int` set size same as C stackArray 
    StackArray(u64),
    /// heapArray `[*]int` runtime sized array that lifes on the heap
    HeapArray,
    /// MutRefSlice `[&]int` a Mutable Refrence to any Array kind (can also be part of an array like `slice: [&]int = &array[0..1]`)
    MutSlice,
    /// ConstRefSlice `[@]int` a Inmutable Refrence to any Array kind (can also be part of an array `slice: [@]int = @array[0..1]`)
    ConstSlice,
}
impl ArrayKind {
    pub fn to_string(&self) -> String {
        let mut sb = String::new();
        self.write_to_string(&mut sb).expect("expect no format errors");
        sb
    }

    pub fn write_to_string(&self, sb: &mut String) -> std::fmt::Result {
        use std::fmt::Write;

        match self {
            ArrayKind::StackArray(num) => write!(sb, "[{num}]"),
            ArrayKind::MutSlice => write!(sb, "[&]"),
            ArrayKind::HeapArray => write!(sb, "[*]"),
            ArrayKind::ConstSlice => write!(sb, "[@]"),
        }
    }
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
    pub fn new(ty: SoulType, kind: ArrayKind) -> Self {
        Self { 
            of_type: Box::new(ty), 
            kind,
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
    pub fn new(modifier: Option<TypeModifier>, kind: TypeKind, span: Span) -> Self {
        Self { kind, modifier, span }
    }

    pub const fn none(span: Span) -> Self {
        Self {
            span,
            kind: TypeKind::None,
            modifier: None,
        }
    }

    pub const fn with_modifier(mut self, modifier: Option<TypeModifier>) -> Self {
        self.modifier = modifier;
        self
    }
}
