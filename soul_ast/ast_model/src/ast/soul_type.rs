use soul_utils::{Ident, soul_names::PrimitiveTypes};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SoulType {
    /// empty type
    None,
    /// type will never apear example for unreachable controlflows
    Never,
    /// `str`
    String,
    /// `fstr`
    FormatString,
    /// any type
    Any,
    /// Primitive types like int, bool, float
    Primitive(PrimitiveTypes),
    /// array type: `[1]int` or `[&]int` or `[&mut]int` or `[]int`
    Array(ArrayType),
    /// Reference type: `&int`` or `&mut int`
    Reference(ReferenceType),
    /// Pointer type: `*int`
    Pointer(ReferenceType),
    /// Raw pointer type: `RawPtr` or `RawPtr<int>`. Nullable by default.
    /// When no generic is specified, it's a void pointer (`RawPtr<none>`).
    RawPtr(Option<Box<SoulType>>),
    /// result type: `Res` or `Res<int>` or Res<int, str>.
    /// When no generic is specified, it's a void pointer with Error (`Res<none, Error>`).
    Res {
        ok: Option<Box<SoulType>>,
        err: Option<Box<SoulType>>,
    },
    /// Built-in error wrapper type (like Rust's `anyhow::Error`).
    /// Can wrap any error value — used as the default `E` in `Res<V>`.
    Error,
    /// Optional type: `?int`
    Optional(Box<SoulType>),
    /// unknown type
    Stub(Stub),
    NamedVariant {
        base: Box<SoulType>,
        variant: Ident,
    },
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
    /// StackArrayWildcard `[_]int` set infered size same as C stackArray
    StackArrayWildcard,
    /// stackArray `[2]int` set size same as C stackArray
    StackArray(u64),
    /// heapArray `[]int` runtime sized array that lifes on the heap
    HeapArray,
    /// MutRefSlice `[&]int` a Mutable Refrence to any Array kind (can also be part of an array like `slice: [&]int = &array[0..1]`)
    MutSlice,
    /// ConstRefSlice `[&mut]int` a Inmutable Refrence to any Array kind (can also be part of an array `slice: [&mut]int = &mut array[0..1]`)
    ConstSlice,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceType {
    /// The inner type being referenced.
    pub inner: Box<SoulType>,
    /// The lifetime identifier.
    pub lifetime: Option<Ident>,
    /// Whether the reference is mutable.
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stub {
    pub name: String,
    pub generics: Vec<SoulType>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Generic {
    pub name: Ident,
    /// Optional trait bound: `T: TraitName`
    pub bound: Option<SoulType>,
}

impl Stub {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            generics: vec![],
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
}
