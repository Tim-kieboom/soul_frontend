use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use soul_utils::{compiler_options::CompilerOptions, fault::Severity};

const RAW_CONFIG: &str = include_str!("../config.json");
pub const CONFIG: LazyLock<Configs> = LazyLock::new(parse_config);
pub const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    fail_level: Severity::Error
};

pub const PRINT_CONFIGS: PrintConfigs = PrintConfigs {
    backtrace: true,
    color: true,
};

fn parse_config() -> Configs {
    let json = serde_json::from_str(RAW_CONFIG).expect("should have not parse error");
    Configs::new(json)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonConfigs {
    source_path: String,
    output_path: String,
    project_path: String,
}

#[derive(Debug, Clone)]
pub struct Configs {
    source_path: PathBuf,
    output_path: PathBuf,
}

impl Configs {
    pub fn new(json: JsonConfigs) -> Self {
        Self {
            source_path: Path::new(&json.project_path).join(json.source_path),
            output_path: Path::new(&json.project_path).join(json.output_path),
        }
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

pub struct PrintConfigs {
    pub backtrace: bool,
    pub color: bool,
}
