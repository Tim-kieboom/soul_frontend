use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use soul_utils::{
    compiler_options::{CompilerOptions, MirOptions, PlatformInfo},
    fault::Severity,
};

const RAW_CONFIG: &str = include_str!("../config.json");
pub static CONFIG: LazyLock<Configs> = LazyLock::new(parse_config);

const MIR_OPTIONS: MirOptions = MirOptions::empty()
    .add(MirOptions::CHECK_ALGORITHMIC_OVERFLOW)
    .add(MirOptions::CHECK_INDEX_OUT_OF_BOUNDS);

pub const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    mir: MIR_OPTIONS,
    fail_level: Severity::Error,
    platform: PlatformInfo::new_windows_x86_64(),
};

pub const PRINT_CONFIGS: PrintConfigs = PrintConfigs {
    #[cfg(feature = "error_backtrace")]
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
    main_path: String,
    source_path: String,
    output_path: String,
    project_path: String,
}

#[derive(Debug, Clone)]
pub struct Configs {
    source_path: PathBuf,
    output_path: PathBuf,
    main_file_name: String,
}

impl Configs {
    pub fn new(json: JsonConfigs) -> Self {
        Self {
            main_file_name: json.main_path,
            source_path: Path::new(&json.project_path).join(json.source_path),
            output_path: Path::new(&json.project_path).join(json.output_path),
        }
    }

    pub fn to_main_path(&self) -> PathBuf {
        self.source_path.join(&self.main_file_name)
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

pub struct PrintConfigs {
    #[cfg(feature = "error_backtrace")]
    pub backtrace: bool,
    pub color: bool,
}
