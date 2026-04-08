use std::vec;

use ast::ArrayKind;
use hir::{
    BlockId, ExpressionId, FieldId, GenericId, LazyTypeId, LocalId, PlaceId, StatementId, StructId,
    TypeId,
};
use soul_utils::{
    bimap::BiMap,
    ids::{FunctionId, IdAlloc},
    soul_names::{PrimitiveTypes, TypeModifier},
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
    pub generics: VecMap<GenericId, String>,
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
            structs: VecMap::const_default(),
            generics: VecMap::const_default(),
        }
    }

    pub fn id_to_type(&self, id: TypeId) -> Option<&ThirType> {
        self.types.get_value(id)
    }
    pub fn id_to_struct(&self, id: StructId) -> Option<&Struct> {
        self.structs.get(id)
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
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThirTypeKind {
    None,
    Type,
    Primitive(PrimitiveTypes),
    Array { element: TypeId, kind: ArrayKind },
    Ref { of_type: TypeId, mutable: bool },
    Pointer(TypeId),
    Optional(TypeId),
    Generic(GenericId),
    Struct(StructId),

    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub id: StructId,
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub id: FieldId,
    pub ty: TypeId,
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeTable {
    pub none_type: TypeId,
    pub bool_type: TypeId,
    pub u32_type: TypeId,

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
    pub base_type: TypeId,
    pub field_type: LazyTypeId,
    pub field_index: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldInfo {
    pub base_type: TypeId,
    pub field_type: TypeId,
    pub field_index: usize,
}
