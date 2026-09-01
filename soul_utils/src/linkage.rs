#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Linkage {
    #[serde(rename = "dynamic")]
    #[default]
    Dynamic,
    #[serde(rename = "static")]
    Static,
}
