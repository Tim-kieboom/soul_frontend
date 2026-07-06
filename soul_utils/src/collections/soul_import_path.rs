use std::path::{Path, PathBuf};

/// A path to a Soul page/module.
///
/// Represents a hierarchical path to a module or page in the Soul language,
/// similar to a module path in other languages.
#[derive(
    Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct SoulImportPath{
    path: PathBuf,
    is_external: bool,
    is_absolute: bool,
}

impl SoulImportPath {

    pub fn new(path: PathBuf, is_external: bool) -> Self {
        Self { path, is_external, is_absolute: false }
    }

    pub fn set_extension(&mut self, extension: &str) -> bool {
        self.path.set_extension(extension)
    }

    pub fn set_external(&mut self) {
        self.is_external = true;
    }

    pub fn set_internal(&mut self) {
        self.is_external = false;
    }

    pub fn is_external(&self) -> bool {
        self.is_external
    }

    pub fn set_absolute(&mut self) {
        self.is_absolute = true;
    }

    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }

    pub fn push(&mut self, value: &str) {
        self.path.push(value);
    }

    pub fn pop(&mut self) -> bool {
        self.path.pop()
    }

    pub fn get_module_name(&self) -> Option<&str> {
        self.path.file_name()?.to_str()?.split('.').next()
    }

    pub fn iter(&mut self) -> std::path::Iter<'_> {
        self.path.iter()
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }

    pub fn as_pathbuf(&self) -> &PathBuf {
        &self.path
    }

    pub fn to_pathbuf(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn to_full_path(&self, dir_path: &PathBuf) -> PathBuf {
        let mut this = dir_path.clone();
        this.push(&self.path);
        this
    }

    pub fn display(&self, root_dir: &Path) -> String {
        let mut sb = String::new();
        self.write_display(root_dir, &mut sb);
        sb
    }

    pub fn write_display(&self, root_dir: &Path, sb: &mut String) {
        const SEPERATOR: &str = ".";

        let relative = self
            .as_path()
            .strip_prefix(root_dir)
            .unwrap_or(self.as_path());

        sb.push_str("crate");
        for pat in relative {
            sb.push_str(SEPERATOR);
            let text = match pat.to_str() {
                Some(str) => str,
                None => &pat.to_string_lossy(),
            };

            sb.push_str(text);
        }
    }

    pub fn to_string(self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

