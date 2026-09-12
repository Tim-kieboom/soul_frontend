use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use soul_utils::{
    compiler_options::{CompilerOptions, MirOptions, PlatformInfo},
    fault::Severity,
};

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

/// `config.json`'s own location, resolved relative to the running exe
/// rather than baked in at compile time (`include_str!`) — needed so
/// `scripts/run_codegen_tests.py` can build `soul_tester` once and then
/// invoke `target/debug/soul_tester.exe` directly for every test file,
/// rewriting `config.json` between runs without forcing a recompile each
/// time. Assumes the exe lives three directories under the repo root
/// (`target/<profile>/soul_tester.exe`, `cargo`'s own default layout) —
/// doesn't hold under a custom `--target-dir`, but this binary is only ever
/// invoked via `cargo run`/`cargo build` or the test script, never shipped.
fn config_path() -> PathBuf {
    let exe = std::env::current_exe().expect("failed to resolve the running exe's own path");
    exe.parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .unwrap_or_else(|| {
            panic!(
                "expected {} to live three directories under the repo root (target/<profile>/soul_tester.exe)",
                exe.display()
            )
        })
        .join("soul_tester")
        .join("config.json")
}

fn parse_config() -> Configs {
    let path = config_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read config at {}: {e}", path.display()));
    let json: JsonConfigs = serde_json::from_str(&raw).expect("should have not parse error");
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
