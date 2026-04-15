use std::path::{Path, PathBuf, StripPrefixError};

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

    pub fn push(&mut self, value: &str) {
        self.0.push(value);
    }

    pub fn pop(&mut self) -> bool {
        self.0.pop()
    }

    pub fn get_module_name(&self) -> Option<&str> {
        self.0.file_name()?
            .to_str()?
            .split('.')
            .next()
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

    pub fn to_pathbuf(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn to_full_path(&self, dir_path: &PathBuf) -> PathBuf {
        let mut this = dir_path.clone();
        this.push(&self.0);
        this
    }

    pub fn write_display(&self, root_dir: &Path, sb: &mut String) -> Result<(), StripPrefixError> {
        const SEPERATOR: &str = ".";

        let relative = self.as_path().strip_prefix(root_dir)?;
        sb.push_str("crate");
        for pat in relative {
            sb.push_str(SEPERATOR);
            let text = match pat.to_str() {
                Some(str) => str,
                None => &pat.to_string_lossy(), 
            };

            sb.push_str(text);
        }

        Ok(())
    }

    pub fn to_string(self) -> String {
        self.0.to_string_lossy().to_string()
    }
}
impl From<PathBuf> for SoulImportPath {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}
