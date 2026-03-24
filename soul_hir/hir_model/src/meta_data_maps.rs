use std::{collections::HashMap, hash::Hash};

use soul_utils::{
    Ident, ids::{FunctionId, IdAlloc, IdGenerator}, impl_soul_ids, soul_import_path::SoulImportPath, soul_names::TypeModifier, span::{ItemMetaData, Span}, vec_map::{VecMap, VecMapIndex}
};

use crate::{
    BlockId, ExpressionId, Field, GenericId, HirType, HirTypeKind, InferTypeId, LocalId, ModuleId, StatementId, StructId, TypeId
};

/// Maps HIR node IDs to their original source code spans.
///
/// Spans originate from the AST and are forwarded through lowering passes.
/// They are used exclusively for diagnostics and are not part of the IR
/// structure itself.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpanMap {
    /// Source spans for expressions.
    pub expressions: VecMap<ExpressionId, Span>,

    /// Source spans for statements.
    pub statements: VecMap<StatementId, Span>,

    /// Source spans for blocks.
    pub blocks: VecMap<BlockId, Span>,

    pub locals: VecMap<LocalId, Span>,

    pub functions: VecMap<FunctionId, Span>,
}
impl Default for SpanMap {
    fn default() -> Self {
        Self {
            blocks: Default::default(),
            locals: Default::default(),
            functions: Default::default(),
            statements: Default::default(),
            expressions: VecMap::from_slice(&[(ExpressionId::error(), Span::default_const())]),
        }
    }
}

/// Auxiliary semantic metadata attached to HIR statements.
///
/// This data is not required for code generation but is useful for
/// analysis passes such as borrow checking, drop elaboration,
/// or control-flow diagnostics.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MetaDataMap {
    /// Metadata associated with statements.
    pub statements: VecMap<StatementId, ItemMetaData>,
}

impl_soul_ids!(RefTypeId);
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypesMap {
    structs: VecMap<StructId, Struct>, 

    map: BiMap<TypeId, HirType>,
    /// for expressions like Cast (so you can have a diffrent TypeId in the typedcontext stage)
    ref_map: BiMap<RefTypeId, TypeId>,
    generics: VecMap<GenericId, String>,
    type_generator: IdGenerator<TypeId>,
    ref_generator: IdGenerator<RefTypeId>,
    struct_generator: IdGenerator<StructId>,
    infer_generator: IdGenerator<InferTypeId>,
}
impl TypesMap {
    pub fn new() -> Self {
        Self {
            structs: VecMap::new(),
            generics: VecMap::new(),
            ref_generator: IdGenerator::new(),
            type_generator: IdGenerator::new(),
            infer_generator: IdGenerator::new(),
            struct_generator: IdGenerator::new(),
            map: BiMap::from_array([(TypeId::error(), HirType::error_type())]),
            ref_map: BiMap::from_array([(RefTypeId::error(), TypeId::error())]),
        }
    }

    pub fn set_structs(&mut self, structs: VecMap<StructId, Struct>) {
        self.structs = structs;
    }

    pub fn structs(&self) -> &VecMap<StructId, Struct> {
        &self.structs
    }

    pub fn id_to_struct(&self, id: StructId) -> Option<&Struct> {
        self.structs.get(id)
    }

    pub fn ref_to_id(&self, id: RefTypeId) -> Option<TypeId> {
        self.ref_map.get_value(id).copied()
    }

    pub fn id_to_ref(&self, id: TypeId) -> Option<RefTypeId> {
        self.ref_map.get_key(&id)
    }

    pub fn ref_to_type(&self, id: RefTypeId) -> Option<&HirType> {
        let id = self.ref_to_id(id)?;
        self.map.get_value(id)
    }

    pub fn id_to_type(&self, id: TypeId) -> Option<&HirType> {
        self.map.get_value(id)
    }

    pub fn type_to_id(&self, ty: &HirType) -> Option<TypeId> {
        self.map.get_key(ty)
    }

    pub fn typekind_to_id(&self, ty: HirTypeKind) -> Option<TypeId> {
        self.map.get_key(&HirType {
            kind: ty,
            modifier: None,
        })
    }

    pub fn insert_struct(&mut self, obj: Struct) -> StructId {
        let id = self.struct_generator.alloc();
        self.structs.insert(id, obj);
        id
    }

    pub fn insert_generic(&mut self, name: String, id: GenericId) {
        self.generics.insert(id, name);
    }

    pub fn generic_name(&self, id: GenericId) -> Option<&str> {
        self.generics.get(id).map(|text| text.as_str())
    }

    pub fn insert(&mut self, ty: HirType) -> TypeId {
        self.map.insert(&mut self.type_generator, ty)
    }

    pub fn insert_ref(&mut self, ty: TypeId) -> RefTypeId {
        self.ref_map.insert(&mut self.ref_generator, ty)
    }

    pub fn force_insert_ref(&mut self, ref_id: RefTypeId, ty: TypeId) {
        self.ref_map.force_insert(ref_id, ty);
    }

    pub fn new_infertype(&mut self, modifier: Option<TypeModifier>, span: Span) -> TypeId {
        let infer = self.infer_generator.alloc();
        let ty = self.insert(HirType {
            kind: crate::HirTypeKind::InferType(infer, span),
            modifier,
        });
        ty
    }

    pub fn iter_types(&self) -> impl Iterator<Item = &HirType> {
        self.map.key_to_value.values()
    }

    pub fn iter_ids(&self) -> impl Iterator<Item = TypeId> {
        self.map.key_to_value.keys()
    }

    pub fn infer_generator(&self) -> &IdGenerator<InferTypeId> {
        &self.infer_generator
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub name: Ident,
    pub fields: Vec<Field>,
    pub generics: Vec<GenericId>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImportMap {
    map: BiMap<ModuleId, SoulImportPath>,
}
impl ImportMap {
    pub fn new() -> Self {
        Self { map: BiMap::new() }
    }

    pub fn insert(&mut self, alloc: &mut IdGenerator<ModuleId>, path: SoulImportPath) -> ModuleId {
        self.map.insert(alloc, path)
    }

    pub fn get_module(&self, id: ModuleId) -> Option<&SoulImportPath> {
        self.map.get_value(id)
    }

    pub fn get_id(&self, value: &SoulImportPath) -> Option<ModuleId> {
        self.map.get_key(value)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct BiMap<K: VecMapIndex, V: Hash + PartialEq + Eq> {
    key_to_value: VecMap<K, V>,
    value_to_key: HashMap<V, K>,
}
impl<K, V> BiMap<K, V>
where
    K: VecMapIndex + IdAlloc + Copy,
    V: Hash + Clone + PartialEq + Eq,
{
    pub fn new() -> Self {
        Self {
            key_to_value: VecMap::new(),
            value_to_key: HashMap::new(),
        }
    }

    pub fn from_array<const N: usize>(vec: [(K, V); N]) -> Self {
        let mut this = Self::new();

        for (key, value) in vec {
            this.force_insert(key, value)
        }
        this
    }

    pub fn insert(&mut self, alloc: &mut IdGenerator<K>, value: V) -> K {
        match self.value_to_key.get(&value) {
            Some(id) => return *id,
            None => {
                let id = alloc.alloc();
                self.value_to_key.insert(value.clone(), id);
                self.key_to_value.insert(id, value);
                return id;
            }
        }
    }

    pub fn force_insert(&mut self, key: K, value: V) {
        self.key_to_value.insert(key, value.clone());
        self.value_to_key.insert(value, key);
    }

    pub fn get_value(&self, key: K) -> Option<&V> {
        self.key_to_value.get(key)
    }

    pub fn get_key(&self, value: &V) -> Option<K> {
        self.value_to_key.get(value).copied()
    }
}
impl<K, V> Default for BiMap<K, V>
where
    K: VecMapIndex + IdAlloc + Copy,
    V: Hash + Clone + PartialEq + Eq,
{
    fn default() -> Self {
        Self {
            key_to_value: Default::default(),
            value_to_key: Default::default(),
        }
    }
}

impl<K, V> serde::Serialize for BiMap<K, V>
where
    K: serde::Serialize + VecMapIndex + IdAlloc + Copy,
    V: serde::Serialize + Hash + Clone + PartialEq + Eq,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.key_to_value.serialize(serializer)
    }
}
