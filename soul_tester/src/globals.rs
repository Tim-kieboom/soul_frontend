use crate::{benchmark::Benchmarks, paths::Paths};
use anyhow::Result;
use soul_utils::{
    compile_options::{Arch, CompilerOptions, Os, TargetInfo},
    sementic_level::{MessageConfig, SementicLevel},
};
use std::sync::{LazyLock, Mutex, MutexGuard};

static RAW_PATHS: &[u8] = include_bytes!("../paths.json");
static BENCHMARKS: Mutex<Benchmarks> = Mutex::new(Benchmarks::const_default());

const OS: Os = Os::Windows;
const ARCH: Arch = Arch::X86_64;
const TARGET: TargetInfo = TargetInfo::new(ARCH, OS);

const DEFAULT_PACKED: bool = false;
const DEBUG_VIEW_LITERAL_RESOLVE: bool = false;

pub(crate) static PATHS: LazyLock<Paths> =
    LazyLock::new(|| serde_json::from_slice(RAW_PATHS).expect("no json error"));

pub(crate) const MESSAGE_CONFIG: MessageConfig = MessageConfig {
    backtrace: crate::BACKTRACE,
    colors: true,
};

pub(crate) const COMPILER_OPTIONS: CompilerOptions = CompilerOptions::new(
    DEBUG_VIEW_LITERAL_RESOLVE,
    SementicLevel::Error,
    TARGET,
    DEFAULT_PACKED,
);

pub(crate) fn get_benchmarks<'a>() -> Result<MutexGuard<'a, Benchmarks>> {
    BENCHMARKS
        .lock()
        .map_err(|err| anyhow::Error::msg(err.to_string()))
}
