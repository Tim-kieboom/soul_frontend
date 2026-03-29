use soul_utils::{bimap::BiMap, ids::IdGenerator, soul_names::TypeModifier, span::Span, vec_map::VecMap};

use crate::{GenericId, InferTypeId, StructId, TypeId, hir_type::{HirType, InferType, Struct}};

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

    pub fn clear(&mut self) {
        self.types.clear();
        self.structs.clear();
        self.generics.clear();
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

    pub fn structs_entries(&self) -> impl Iterator<Item = (StructId, &Struct)> {
        self.structs.entries()
    }

    pub fn types_keys(&self) -> impl Iterator<Item = TypeId> {
        self.types.keys()
    }

    pub fn insert_generic(&mut self, name: String) -> GenericId {
        let id = self.generic_alloc.alloc();
        self.generics.insert(id, name);
        id
    }

    pub fn id_to_generic(&self, id: GenericId) -> Option<&str> {
        self.generics.get(id).map(|text| text.as_str())
    }

    pub fn clone_type_alloc(&self) -> IdGenerator<TypeId> {
        self.type_alloc.clone()
    }

    pub fn clone_struct_alloc(&self) -> IdGenerator<StructId> {
        self.struct_alloc.clone()
    }

    pub fn clone_generic_alloc(&self) -> IdGenerator<GenericId> {
        self.generic_alloc.clone()
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InferTypesMap {
    infers: VecMap<InferTypeId, (InferType, Span)>,

    infer_alloc: IdGenerator<InferTypeId>,
}
impl InferTypesMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.infers.clear();
    }

    pub fn get_infer(&self, id: InferTypeId) -> Option<&InferType> {
        Some(&self.infers.get(id)?.0)
    }

    pub fn get_span(&self, id: InferTypeId) -> Option<Span> {
        Some(self.infers.get(id)?.1)
    }

    pub fn insert_infer(&mut self, generics: Vec<TypeId>, modifier: Option<TypeModifier>, span: Span) -> InferTypeId {
        let id = self.infer_alloc.alloc();
        self.infers.insert(id, (InferType{ kind: id, generics, modifier }, span));
        id
    }

    pub fn insert(&mut self, infer: InferType, span: Span) -> InferTypeId {
        let id = self.infer_alloc.alloc();
        self.infers.insert(id, (infer, span));
        id
    }

    pub fn entries(&self) -> impl Iterator<Item = (InferTypeId, &(InferType, Span))> {
        self.infers.entries()
    }

    pub fn clone_infer_allocator(&self) -> IdGenerator<InferTypeId> {
        self.infer_alloc.clone()
    }
}