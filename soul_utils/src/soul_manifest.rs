use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SoulToml {
    pub package: Package,
    #[serde(default)]
    pub dependencies: std::collections::HashMap<String, Dependency>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub edition: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub path: PathBuf,
    pub version: Option<String>,
}

impl SoulToml {
    pub fn from_path(path: &PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}