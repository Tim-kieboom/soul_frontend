use soul_ast::{abstract_syntax_tree::statment::Ident, soul_names::TypeModifier};

use crate::HirId;

/// Resolved HIR type with generic arguments.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirType {
    /// Core type kind.
    pub kind: HirTypeKind,
    /// Generic arguments (for parameterized types).
    pub generics: Vec<HirType>,
    pub modifier: Option<TypeModifier>,
}

/// Core type kinds in HIR.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirTypeKind {
    /// Stack-allocated array with known size.
    StackArray { ty: Box<HirType>, size: HirId },
    /// Reference type (const=`@T` or mut=`&T`).
    Ref { ty: Box<HirType>, mutable: bool },
    /// Raw pointer (`*T`).
    Pointer(Box<HirType>),
    /// Primitive type.
    Primitive(Primitive),
    /// Heap-allocated array.
    Array(Box<HirType>),
    /// Struct type reference.
    Struct(HirId),
    /// Union type reference.
    Union(HirId),
    /// Enum type reference.
    Enum(HirId),
    /// Empty type `none`.
    None,
    /// Unresolved generic parameter.
    Generic(Ident),
}

/// Primitive types with size information.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Primitive {
    /// Boolean type.
    Boolean,
    /// Signed integer.
    Int(PrimitiveSize),
    /// Character (fixed-width).
    Char(PrimitiveSize),
    /// Unsigned integer.
    Uint(PrimitiveSize),
    /// Floating-point.
    Float(PrimitiveSize),
}

/// Primitive type bit widths.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveSize {
    /// 8-bit.
    Bit8 = 8,
    /// 16-bit.
    Bit16 = 16,
    /// 32-bit.
    Bit32 = 32,
    /// 64-bit.
    Bit64 = 64,
}
