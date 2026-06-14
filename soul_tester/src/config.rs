use std::sync::LazyLock;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configs {
    pub main_path: String,
    pub output_path: String,
}

const RAW_CONFIG: &str = include_str!("../config.json");
pub const CONFIG: LazyLock<Configs> = LazyLock::new(|| serde_json::from_str(RAW_CONFIG).expect("should have not parse error") );