use std::path::PathBuf;

use crate::{collections::bimap::BiMap, ids::IdGenerator, span::ModuleId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleStore {
    root: ModuleId,
    map: BiMap<ModuleId, PathBuf>,
    alloc: IdGenerator<ModuleId>,
}
impl ModuleStore {
    pub fn new() -> Self {
        let mut alloc = IdGenerator::new();
        let root = alloc.alloc();
        Self {
            root,
            alloc,
            map: BiMap::new(),
        }
    }

    pub fn alloc(&mut self) -> ModuleId {
        self.alloc.alloc()
    }

    pub fn insert_root(&mut self, root_path: PathBuf) {
        self.map.force_insert(self.get_root_id(), root_path)
    }

    pub fn get_or_insert(&mut self, path: &PathBuf) -> ModuleId {
        if let Some(id) = self.get_id(path) {
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

impl Default for ModuleStore {
    fn default() -> Self {
        Self::new()
    }
}
