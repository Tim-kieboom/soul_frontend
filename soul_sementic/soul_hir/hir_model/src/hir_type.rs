use parser_models::scope::NodeId;
use soul_utils::{Ident, soul_names::TypeModifier, span::Span};

use crate::{ExpressionId, LocalDefId, Visibility};

/// Resolved HIR type with generic arguments.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirType {
    /// Core type kind.
    pub kind: HirTypeKind,
    /// Generic arguments (for parameterized types).
    pub generics: Vec<GenericDefine>,
    pub modifier: TypeModifier,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GenericDefine {
    Type(HirType),
    Lifetime(Ident),
    Expression(ExpressionId),
}

pub type TupleType = Vec<HirType>;
pub type NamedTupleType = Vec<FieldType>;

/// Core type kinds in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HirTypeKind {
    Stub(NodeId),
    /// Stack-allocated array with known size.
    StackArray {
        ty: Box<HirType>,
        size: u64,
    },
    /// Reference type (const=`@T` or mut=`&T`).
    Ref {
        ty: Box<HirType>,
        mutable: bool,
    },
    Type,
    /// Raw pointer (`*T`).
    Pointer(Box<HirType>),
    /// Primitive type.
    Primitive(Primitive),
    /// Heap-allocated array.
    NamedTuple(NamedTupleType),
    Tuple(TupleType),
    /// Empty type `none`.
    None,
    /// Unresolved generic parameter.
    Generic(LocalDefId),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldType {
    pub name: Ident, 
    pub ty: HirType, 
    pub id: NodeId,
    pub vis: Visibility,
}
impl FieldType {
    pub fn new(name: Ident, ty: HirType, id: NodeId, vis: Visibility) -> Self {
        Self {
            name,
            ty,
            id,
            vis,
        }
    }
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
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveSize {
    /// 0-bit
    Nil = 0, 
    /// 8-bit.
    Bit8 = 8,
    /// 16-bit.
    Bit16 = 16,
    /// 32-bit.
    Bit32 = 32,
    /// 64-bit.
    Bit64 = 64,
    /// 64-bit.
    Bit124 = 124,
}
