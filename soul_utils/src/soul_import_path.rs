use std::{
    path::{Path, PathBuf},
};

/// A path to a Soul page/module.
///
/// Represents a hierarchical path to a module or page in the Soul language,
/// similar to a module path in other languages.
#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct SoulImportPath(PathBuf);

impl SoulImportPath {
    pub fn new() -> Self {
        Self(PathBuf::new())
    }

    pub fn from_str(s: &str) -> Self {
        let cleaned = if s.starts_with("./") { &s[2..] } else { s };

        Self(PathBuf::from(cleaned.to_string()))
    }

    pub fn get_module_name(&self) -> Option<&str> {
        self.0.file_name()?
            .to_str()?
            .split('.')
            .next()
    }

    pub fn push(&mut self, value: &str) {
        self.0.push(value);
    }

    pub fn iter(&mut self) -> std::path::Iter<'_> {
        self.0.iter()
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn as_pathbuf(&self) -> &PathBuf {
        &self.0
    }

    pub fn to_full_path(&self, dir_path: &PathBuf) -> PathBuf {
        let mut this = dir_path.clone();
        this.push(&self.0);
        this.set_extension("soul");
        this
    }

    pub fn to_string(self) -> String {
        self.0.to_string_lossy().to_string()
    }
}
