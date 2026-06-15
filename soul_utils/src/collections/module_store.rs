use std::path::PathBuf;

use crate::{
    collections::bimap::BiMap,
    ids::{IdAlloc, IdGenerator},
    span::ModuleId,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleStore {
    root: ModuleId,
    map: BiMap<ModuleId, PathBuf>,
    alloc: IdGenerator<ModuleId>,
}
impl ModuleStore {
    pub fn new(root_path: PathBuf) -> Self {
        let mut this = Self {
            map: BiMap::new(),
            root: ModuleId::error(),
            alloc: IdGenerator::new(),
        };
        this.root = this.insert(root_path);
        this
    }

    pub fn get_or_insert(&mut self, path: &PathBuf) -> ModuleId {
        if let Some(id) = self.get_id(&path) {
            return id;
        }
        self.insert(path.clone())
    }

    pub fn insert(&mut self, path: PathBuf) -> ModuleId {
        self.map.insert(&mut self.alloc, path)
    }

    pub fn get_root_id(&self) -> ModuleId {
        self.root
    }

    pub fn entries(&self) -> impl Iterator<Item = (ModuleId, &PathBuf)> {
        self.map.entries()
    }

    pub fn get_id(&self, path: &PathBuf) -> Option<ModuleId> {
        self.map.get_key(path)
    }

    pub fn get_path(&self, id: ModuleId) -> Option<&PathBuf> {
        self.map.get_value(id)
    }
}
