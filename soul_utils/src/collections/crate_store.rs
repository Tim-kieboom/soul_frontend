use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::linkage::Linkage;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrateStore {
    crates: HashMap<String, CrateEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrateEntry {
    pub name: String,
    pub source_root: PathBuf,
    pub linkage: Linkage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub name: Option<String>,
    pub dependencies: Option<HashMap<String, DependencySpec>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencySpec {
    pub path: Option<String>,
    #[serde(default)]
    pub linkage: Linkage,
}

impl CrateStore {
    pub fn new() -> Self {
        Self {
            crates: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, entry: CrateEntry) {
        self.crates.insert(name, entry);
    }

    pub fn get(&self, name: &str) -> Option<&CrateEntry> {
        self.crates.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.crates.contains_key(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &CrateEntry)> {
        self.crates.iter()
    }
}

impl CrateEntry {
    pub fn new(name: String, source_root: PathBuf) -> Self {
        Self {
            name,
            source_root,
            linkage: Linkage::default(),
        }
    }

    pub fn with_linkage(mut self, linkage: Linkage) -> Self {
        self.linkage = linkage;
        self
    }
}

impl Manifest {
    pub fn load(path: &Path) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
        toml::from_str(&contents).ok()
    }

    pub fn load_from_dir(dir: &Path) -> Option<Self> {
        let manifest_path = dir.join("Soul.toml");
        Self::load(&manifest_path)
    }
}

/// Given a crate directory (as specified in Soul.toml's `path`),
/// determine the source root where .soul files live.
/// Follows Rust convention: if `src/` exists, use it; otherwise use the directory itself.
pub fn resolve_source_root(crate_path: &Path) -> PathBuf {
    let src = crate_path.join("src");
    if src.is_dir() {
        src
    } else {
        crate_path.to_path_buf()
    }
}
