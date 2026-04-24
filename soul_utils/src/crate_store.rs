use std::collections::HashMap;
use std::path::PathBuf;

use crate::{
    ids::{FunctionId, IdGenerator}, impl_soul_ids, sementic_level::{FaultCollector, MessageConfig}, span::{CrateId, ModuleId}, vec_map::VecMap
};

impl_soul_ids!(TypeId);

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CrateExports {
    pub functions: HashMap<String, FunctionId>,
    pub types: HashMap<String, TypeId>,
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrateContext {
    pub is_lib: bool,
    pub faults: FaultCollector,
}
impl CrateContext {
    pub fn new(is_lib: bool, config: MessageConfig) -> Self {
        Self {
            is_lib,
            faults: FaultCollector::new(config),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Crate {
    pub id: CrateId,
    pub name: String,
    pub root_module: ModuleId,
    pub project_path: PathBuf,
    pub exports: CrateExports,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrateStore {
    map: VecMap<CrateId, Crate>,
    alloc: IdGenerator<CrateId>,
    path_to_id: HashMap<String, CrateId>,
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
        if let Some(id) = self.path_to_id.get(&name) {
            return *id;
        }

        let id = self.alloc.alloc();
        let crate_data = Crate {
            id,
            root_module,
            name: name.clone(),
            project_path: source_path,
            exports: CrateExports::default(),
        };
        self.map.insert(id, crate_data);
        self.path_to_id.insert(name, id);
        id
    }

    pub fn resolve_function(&self, crate_name: &str, function_name: &str) -> Option<FunctionId> {
        self.name_to_crate(&crate_name.to_string())
            .and_then(|c| c.exports.functions.get(function_name).copied())
    }

    pub fn resolve_type(&self, crate_name: &str, type_name: &str) -> Option<TypeId> {
        self.name_to_crate(&crate_name.to_string())
            .and_then(|c| c.exports.types.get(type_name).copied())
    }

    pub fn get(&self, id: CrateId) -> Option<&Crate> {
        self.map.get(id)
    }

    pub fn get_mut(&mut self, id: CrateId) -> Option<&mut Crate> {
        self.map.get_mut(id)
    }

    pub fn name_to_id(&self, name: &String) -> Option<CrateId> {
        self.path_to_id.get(name).copied()
    }

    pub fn name_to_crate(&self, name: &String) -> Option<&Crate> {
        let id = self.path_to_id.get(name).copied()?;
        self.get(id)
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
