use std::path::PathBuf;

use crate::{
    bimap::BiMap,
    define_str_enum,
    error::SoulError,
    ids::{IdAlloc, IdGenerator},
    span::ModuleId,
};

define_str_enum!(
    /// Severity level for diagnostics and faults.
    pub enum SementicLevel {
        /// Error level (compilation fails).
        Error => "error", 0,
        /// Warning level (may continue).
        Warning => "warning", 1,
        /// Note level (informational).
        Note => "note", 2,
        /// Debug level (development only).
        Debug => "debug", 3,
    }
);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompilerContext {
    pub source_folder: PathBuf,
    pub faults: Vec<SementicFault>,
    pub module_store: ModuleStore,
    path_stack: Vec<PathBuf>,
}
impl CompilerContext {
    pub fn new(source_folder: PathBuf, root_path: PathBuf) -> Self {
        Self {
            source_folder,
            path_stack: vec![],
            faults: vec![],
            module_store: ModuleStore::new(root_path),
        }
    }

    pub fn current_path(&self) -> &PathBuf {
        match self.path_stack.last() {
            Some(path) => path,
            None => &self.source_folder,
        }
    }

    pub fn pop_current_path(&mut self) {
        self.path_stack.pop();
    }

    pub fn push_current_path(&mut self, path: PathBuf) {
        self.path_stack.push(path);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleStore {
    root: ModuleId,
    map: BiMap<ModuleId, PathBuf>,
    alloc: IdGenerator<ModuleId>,
}
impl ModuleStore {
    pub fn new(root_path: PathBuf) -> Self {
        let mut this = Self {
            root: ModuleId::error(),
            map: BiMap::new(),
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

/// A fault (error/warning/note) that occurred during compilation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SementicFault {
    /// The underlying error.
    message: SoulError,
    /// The severity level of this fault.
    level: SementicLevel,
}
impl SementicFault {
    /// Creates a new error-level fault.
    pub const fn error(err: SoulError) -> Self {
        Self {
            message: err,
            level: SementicLevel::Error,
        }
    }

    /// Creates a new debug-level fault.
    pub const fn debug(err: SoulError) -> Self {
        Self {
            message: err,
            level: SementicLevel::Debug,
        }
    }

    /// Consumes the fault and returns the underlying error.
    pub fn consume_soul_error(self) -> SoulError {
        self.message
    }

    /// Returns a reference to the underlying error.
    pub const fn get_soul_error(&self) -> &SoulError {
        &self.message
    }

    /// Returns the severity level of this fault.
    pub const fn get_level(&self) -> SementicLevel {
        self.level
    }

    /// Checks whether this fault is fatal given the minimum fatal level.
    pub const fn is_fatal(&self, fatal_level: SementicLevel) -> bool {
        fatal_level.precedence().as_usize() >= self.get_level().precedence().as_usize()
    }
}
