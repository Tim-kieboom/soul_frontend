use std::{fmt, rc::Rc};

use soul_utils::{Ident, SharedStr, soul_names::PrimitiveTypes};

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// tuple `(int, str)` and named_tuple `(number: int, text: str)`
    TupleKind(TupleKind),
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
    /// Anonymous `impl Trait` type: `impl Display`.
    ImplTrait(Box<SoulType>),
    /// unknown type
    Stub(Stub),
    NamedVariant {
        base: Box<SoulType>,
        variant: Ident,
    },
}

impl fmt::Debug for SoulType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SoulType::None => write!(f, "none"),
            SoulType::Never => write!(f, "!"),
            SoulType::String => write!(f, "str"),
            SoulType::FormatString => write!(f, "fstr"),
            SoulType::Any => write!(f, "any"),
            SoulType::TupleKind(kind) => write!(f, "{:?}", kind),
            SoulType::Primitive(primitive) => write!(f, "{}", primitive),
            SoulType::Array(array) => write!(f, "{:?}", array),
            SoulType::Reference(reference) => {
                write!(f, "&")?;
                if let Some(lifetime) = &reference.lifetime {
                    write!(f, "'{} ", lifetime)?;
                }
                if reference.mutable {
                    write!(f, "mut ")?;
                }
                write!(f, "{:?}", reference.inner)
            }
            SoulType::Pointer(pointer) => {
                write!(f, "*")?;
                if let Some(lifetime) = &pointer.lifetime {
                    write!(f, "'{} ", lifetime)?;
                }
                if pointer.mutable {
                    write!(f, "mut ")?;
                }
                write!(f, "{:?}", pointer.inner)
            }
            SoulType::RawPtr(generic) => match generic {
                Some(ty) => write!(f, "RawPtr<{:?}>", ty),
                None => write!(f, "RawPtr"),
            },
            SoulType::Res { ok, err } => match (ok, err) {
                (None, None) => write!(f, "Res"),
                (Some(ok), None) => write!(f, "Res<{:?}>", ok),
                (None, Some(err)) => write!(f, "Res<none, {:?}>", err),
                (Some(ok), Some(err)) => write!(f, "Res<{:?}, {:?}>", ok, err),
            },
            SoulType::Error => write!(f, "Error"),
            SoulType::Optional(inner) => write!(f, "?{:?}", inner),
            SoulType::ImplTrait(inner) => write!(f, "impl {:?}", inner),
            SoulType::Stub(stub) => write!(f, "{:?}", stub),
            SoulType::NamedVariant { base, variant } => write!(f, "{:?}::{}", base, variant),
        }
    }
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TupleKind {
    Tuple(Tuple),
    NamedTuple(NamedTuple),
}

impl fmt::Debug for TupleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TupleKind::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", ty)?;
                }
                write!(f, ")")
            }
            TupleKind::NamedTuple(items) => {
                write!(f, "(")?;
                for (i, (name, ty)) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {:?}", name, ty)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl TupleKind {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        match self {
            TupleKind::Tuple(types) => types.len(),
            TupleKind::NamedTuple(items) => items.len(),
        }
    }
}

pub type Tuple = Vec<SoulType>;
pub type NamedTuple = Vec<(Ident, SoulType)>;

/// Array type
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArrayType {
    /// The element type of the array.
    pub of_type: Box<SoulType>,
    /// Compile-time size, or `None` for dynamic arrays.
    pub kind: ArrayKind,
}

impl fmt::Debug for ArrayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ArrayKind::StackArrayWildcard => write!(f, "[_]")?,
            ArrayKind::StackArray(size) => write!(f, "[{}]", size)?,
            ArrayKind::HeapArray => write!(f, "[]")?,
            ArrayKind::MutSlice => write!(f, "[&]")?,
            ArrayKind::ConstSlice => write!(f, "[&mut]")?,
        }
        write!(f, "{:?}", self.of_type)
    }
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

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stub {
    pub name: SharedStr,
    pub generics: Vec<SoulType>,
}

impl fmt::Debug for Stub {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.generics.is_empty() {
            write!(f, "<")?;
            for (i, generic) in self.generics.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{:?}", generic)?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Generic {
    pub name: Ident,
    /// Optional trait bound: `T: TraitName`
    pub bound: Option<SoulType>,
}

impl Stub {
    pub fn new(name: impl Into<Rc<str>>) -> Self {
        Self {
            generics: vec![],
            name: SharedStr::new(name),
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
