use std::{collections::HashMap, hash::Hash};

use soul_utils::{
    soul_import_path::SoulImportPath, span::{ItemMetaData, Span}, vec_map::{VecMap, VecMapIndex}
};

use crate::{ExpressionId, HirType, IdAlloc, IdGenerator, ModuleId, StatementId, TypeId};

/// Maps HIR node IDs to their original source code spans.
///
/// Spans originate from the AST and are forwarded through lowering passes.
/// They are used exclusively for diagnostics and are not part of the IR
/// structure itself.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SpanMap {
    /// Source spans for expressions.
    pub expressions: VecMap<ExpressionId, Span>,

    /// Source spans for statements.
    pub statements: VecMap<StatementId, Span>,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypedContext {
    map: BiMap<TypeId, HirType>,
}
impl TypedContext {
    pub fn new() -> Self {
        let mut this = Self { map: BiMap::new() };
        this.map.force_insert(this.error_type(), HirType::error_ty());
        this
    }

    pub fn error_type(&self) -> TypeId {
        TypeId::error()
    }

    pub fn get_type(&self, id: TypeId) -> Option<&HirType> {
        self.map.get_value(id)
    }

    pub fn get_id(&self, ty: &HirType) -> Option<TypeId> {
        self.map.get_key(ty)
    }

    pub fn insert(&mut self, alloc: &mut IdGenerator<TypeId>, ty: HirType) -> TypeId {
        self.map.insert(alloc, ty)
    }
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
        Self { key_to_value: Default::default(), value_to_key: Default::default() }
    }
}

impl<K, V> serde::Serialize for BiMap<K, V>
where
    K: serde::Serialize + VecMapIndex + IdAlloc + Copy,
    V: serde::Serialize + Hash + Clone + PartialEq + Eq,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer 
    {
        self.key_to_value.serialize(serializer)
    }
}
