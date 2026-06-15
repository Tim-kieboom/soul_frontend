use std::sync::LazyLock;

use soul_utils::compiler_options::CompilerOptions;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configs {
    pub src_path: String,
    pub main_path: String,
    pub output_path: String,
}

const RAW_CONFIG: &str = include_str!("../config.json");
pub const CONFIG: LazyLock<Configs> =
    LazyLock::new(|| serde_json::from_str(RAW_CONFIG).expect("should have not parse error"));
pub const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {};

const USE_BACKTRACE: bool = false;
const USE_COLOR: bool = true;
pub const PRINT_CONFIGS: PrintConfigs = PrintConfigs {
    backtrace: USE_BACKTRACE,
    color: USE_COLOR,
};
pub struct PrintConfigs {
    pub backtrace: bool,
    pub color: bool,
}
