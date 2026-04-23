use std::collections::HashMap;
use std::path::PathBuf;

use crate::{
    ids::IdGenerator,
    span::{CrateId, ModuleId},
    vec_map::VecMap,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Crate {
    pub id: CrateId,
    pub name: String,
    pub root_module: ModuleId,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrateStore {
    map: VecMap<CrateId, Crate>,
    path_to_id: HashMap<PathBuf, CrateId>,
    alloc: IdGenerator<CrateId>,
}
impl CrateStore {
    pub fn new() -> Self {
        Self {
            map: VecMap::const_default(),
            path_to_id: HashMap::new(),
            alloc: IdGenerator::new(),
        }
    }

    pub fn insert(&mut self, name: String, source_path: PathBuf, root_module: ModuleId) -> CrateId {
        if let Some(id) = self.path_to_id.get(&source_path) {
            return *id;
        }

        let id = self.alloc.alloc();
        let crate_data = Crate {
            id,
            name,
            root_module,
            source_path: source_path.clone(),
        };
        self.map.insert(id, crate_data);
        self.path_to_id.insert(source_path, id);
        id
    }

    pub fn get(&self, id: CrateId) -> Option<&Crate> {
        self.map.get(id)
    }

    pub fn get_mut(&mut self, id: CrateId) -> Option<&mut Crate> {
        self.map.get_mut(id)
    }

    pub fn get_by_path(&self, path: &PathBuf) -> Option<CrateId> {
        self.path_to_id.get(path).copied()
    }

    pub fn values(&self) -> impl Iterator<Item = &Crate> {
        self.map.values()
    }

    pub fn keys(&self) -> impl Iterator<Item = CrateId> {
        self.map.keys()
    }
}
impl Default for CrateStore {
    fn default() -> Self {
        Self::new()
    }
}
impl std::ops::Index<CrateId> for CrateStore {
    type Output = Crate;
    fn index(&self, index: CrateId) -> &Self::Output {
        &self.map[index]
    }
}