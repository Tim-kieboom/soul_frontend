use soul_utils::{bimap::BiMap, ids::IdGenerator, soul_names::TypeModifier, vec_map::VecMap};

use crate::{GenericId, InferTypeId, StructId, TypeId, hir_type::{HirType, InferType, PossibleTypeId, Struct}};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TypesMap {
    types: BiMap<TypeId, HirType>,
    structs: VecMap<StructId, Struct>,
    generics: VecMap<GenericId, String>,

    type_alloc: IdGenerator<TypeId>,
    struct_alloc: IdGenerator<StructId>,
    generic_alloc: IdGenerator<GenericId>,
}
impl TypesMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_type(&mut self, ty: HirType) -> TypeId {
        self.types.insert(&mut self.type_alloc, ty)
    }

    pub fn id_to_type(&self, id: TypeId) -> Option<&HirType> {
        self.types.get_value(id)
    }

    pub fn type_to_id(&self, ty: &HirType) -> Option<TypeId> {
        self.types.get_key(ty)
    }

    pub fn insert_struct(&mut self, obj: Struct) -> StructId {
        let id = self.struct_alloc.alloc();
        self.structs.insert(id, obj);
        id
    }

    pub fn id_to_struct(&self, id: StructId) -> Option<&Struct> {
        self.structs.get(id)
    }

    pub fn insert_generic(&mut self, name: String) -> GenericId {
        let id = self.generic_alloc.alloc();
        self.generics.insert(id, name);
        id
    }

    pub fn id_to_generic(&self, id: GenericId) -> Option<&str> {
        self.generics.get(id).map(|text| text.as_str())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InferTypesMap {
    infers: VecMap<InferTypeId, InferType>,

    infer_alloc: IdGenerator<InferTypeId>,
}
impl InferTypesMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_infer(&self, id: InferTypeId) -> Option<&InferType> {
        self.infers.get(id)
    }

    pub fn insert_infer(&mut self, generics: Vec<PossibleTypeId>, modifier: Option<TypeModifier>) -> InferTypeId {
        let id = self.infer_alloc.alloc();
        self.infers.insert(id, InferType{ kind: id, generics, modifier });
        id
    }
}