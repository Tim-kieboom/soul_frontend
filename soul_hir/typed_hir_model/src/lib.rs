use std::vec;

use ast::ArrayKind;
use hir::{
    BlockId, CustomTypeId, EnumId, ExpressionId, FieldId, GenericId, LazyTypeId, LocalId, PlaceId,
    StatementId, StructId, TypeId, UnionId,
};
use soul_utils::{
    bimap::BiMap,
    ids::{FunctionId, IdAlloc},
    soul_names::{PrimitiveTypes, TypeModifier},
    span::Span,
    vec_map::VecMap,
    vec_set::VecSet,
};

pub mod display_thir;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TypedHir {
    pub types_map: ThirTypesMap,
    pub types_table: TypeTable,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThirTypesMap {
    pub array_struct: StructId,
    pub types: BiMap<TypeId, ThirType>,
    pub structs: VecMap<StructId, Struct>,
    pub enums: VecMap<EnumId, Enum>,
    pub unions: VecMap<UnionId, UnionInfo>,
    pub generics: VecMap<GenericId, String>,
    pub match_methods: VecMap<ExpressionId, MatchMethodInfo>,
}
impl ThirTypesMap {
    pub fn new(array_struct: StructId) -> Self {
        Self {
            array_struct,
            types: BiMap::from_array([(
                TypeId::error(),
                ThirType {
                    kind: ThirTypeKind::Error,
                    generics: vec![],
                    modifier: None,
                },
            )]),
            enums: VecMap::const_default(),
            structs: VecMap::const_default(),
            unions: VecMap::const_default(),
            generics: VecMap::const_default(),
            match_methods: VecMap::const_default(),
        }
    }

    pub fn id_to_type(&self, id: TypeId) -> Option<&ThirType> {
        self.types.get_value(id)
    }
    pub fn id_to_struct(&self, id: StructId) -> Option<&Struct> {
        self.structs.get(id)
    }
    pub fn id_to_enum(&self, id: EnumId) -> Option<&Enum> {
        self.enums.get(id)
    }
    pub fn id_to_union(&self, id: UnionId) -> Option<&UnionInfo> {
        self.unions.get(id)
    }
    pub fn id_to_generic(&self, id: GenericId) -> Option<&str> {
        self.generics.get(id).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ThirType {
    pub kind: ThirTypeKind,
    pub generics: Vec<TypeId>,
    pub modifier: Option<TypeModifier>,
}
impl ThirType {
    pub fn is_mutable(&self) -> bool {
        self.modifier == Some(TypeModifier::Mut)
    }

    pub const fn is_any_int_type(&self) -> bool {
        if let ThirTypeKind::Primitive(prim) = self.kind {
            prim.is_signed_interger()
        } else {
            false
        }
    }

    pub const fn is_ptr(&self) -> bool {
        matches!(self.kind, ThirTypeKind::Pointer(_))
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThirTypeKind {
    None,
    Type,
    Never,
    Primitive(PrimitiveTypes),
    Array { element: TypeId, kind: ArrayKind },
    Ref { of_type: TypeId, mutable: bool },
    Pointer(TypeId),
    Optional(TypeId),
    Generic(GenericId),
    CustomTypes(CustomTypeId),
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub id: StructId,
    pub name: String,
    pub fields: Vec<Field>,
    pub packed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Enum {
    pub id: EnumId,
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnionInfo {
    pub name: String,
    pub internal_struct: StructId,
    pub variant_types: Vec<TypeId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub id: FieldId,
    pub ty: TypeId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeTable {
    pub none_type: TypeId,
    pub never_type: TypeId,
    pub bool_type: TypeId,
    pub index_type: TypeId,
    pub u32_type: TypeId,
    pub c_int_type: TypeId,

    pub expressions: VecMap<ExpressionId, TypeId>,
    pub statements: VecMap<StatementId, TypeId>,
    pub sizeofs: VecMap<ExpressionId, TypeId>,
    pub functions: VecMap<FunctionId, TypeId>,
    pub places: VecMap<PlaceId, TypeId>,
    pub locals: VecMap<LocalId, TypeId>,
    pub blocks: VecMap<BlockId, TypeId>,

    pub fields: VecMap<FieldId, FieldInfo>,
    pub place_fields: VecMap<PlaceId, FieldId>,

    pub auto_copy: VecSet<ExpressionId>,
    pub generic_instantiations: VecMap<GenericId, VecSet<TypeId>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LazyFieldInfo {
    pub span: Span,
    pub base_type: TypeId,
    pub field_index: usize,
    pub field_type: LazyTypeId,
}

/// Resolved match-method info for `expr.Variant{body}` / chained expressions.
/// Populated by THIR inference, consumed by MIR lowering.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MatchMethodInfo {
    pub union_id: UnionId,
    pub arms: Vec<ResolvedMatchMethodArm>,
}

/// A single arm in a resolved match method.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedMatchMethodArm {
    /// The variant index in the union.
    pub variant_index: usize,
    /// Whether this arm was auto-generated (extracts inner value of an uncovered variant).
    pub is_implicit: bool,
    /// Optional binding local id (for explicit arms with binding).
    pub binding: Option<LocalId>,
    /// The body block id. `None` for implicit arms (MIR generates UnionExtract directly).
    pub body: Option<BlockId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldInfo {
    pub base_type: TypeId,
    pub field_type: TypeId,
    pub field_index: usize,
}

impl ThirTypeKind {
    pub const fn is_array(&self) -> bool {
        match self {
            ThirTypeKind::Array { .. } => true,
            _ => false,
        }
    }
}
