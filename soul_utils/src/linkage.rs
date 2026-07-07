#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Linkage {
    #[serde(rename = "dynamic")]
    Dynamic,
    #[serde(rename = "static")]
    Static,
}

impl Default for Linkage {
    fn default() -> Self {
        Self::Dynamic
    }
}
