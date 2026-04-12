use std::str::Split;

use crate::symbool_kind::SymbolKind;

/// A path to a Soul page/module.
///
/// Represents a hierarchical path to a module or page in the Soul language,
/// similar to a module path in other languages.
#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct SoulImportPath(String);
const PATH_SYMBOOL: &str = SymbolKind::Slash.as_str();

impl SoulImportPath {
    pub fn new() -> Self {
        Self(String::new())
    }

    pub fn from_string(s: String) -> Self {
        let cleaned = s.trim_matches('"').trim_start_matches("./").to_string();
        Self(cleaned)
    }

    pub fn push(&mut self, value: &str) {
        if !self.as_str().is_empty() {
            self.0.push_str(PATH_SYMBOOL);
        }
        self.0.push_str(value);
    }

    pub fn iter(&mut self) -> Split<'_, &str> {
        self.0.split(PATH_SYMBOOL)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_string(self) -> String {
        self.0
    }
}
