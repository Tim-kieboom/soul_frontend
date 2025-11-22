/// A path to a Soul page/module.
///
/// Represents a hierarchical path to a module or page in the Soul language,
/// similar to a module path in other languages.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct SoulPagePath(String);