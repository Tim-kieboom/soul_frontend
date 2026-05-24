use anyhow::Result;
use soul_utils::sementic_level::FaultCollector;

use utils::*;
mod displayers;
pub mod utils;
mod frontend;
mod llvm;
mod globals;

/// if true prints backtrace for each soulError in faults
const BACKTRACE: bool = false;

fn main() -> Result<()> {
    log::init(&globals::PATHS.log_file)?;
    let (manifest, mut crate_store) = globals::PATHS.load_crates()?;
    let root_path = globals::PATHS.project_path();

    let output = frontend::compile(&mut crate_store, &manifest)?;
    let success = llvm::compile(output, root_path, &manifest)?;
    if success {
        log::benchmark()?;
    }

    Ok(())
}

struct Output {
    mir_response: run_mir::MirResponse,
    hir_response: run_hir::HirResponse,
    faults: FaultCollector,
}
