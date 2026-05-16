use anyhow::Result;
use soul_utils::sementic_level::FaultCollector;

use utils::*;
mod displayers;
pub mod utils;
mod frontend;
mod llvm;
mod globals;

fn main() -> Result<()> {
    log::init(&globals::PATHS.log_file)?;
    let (manifest, mut crate_store) = globals::PATHS.load_crates()?;
    let root_path = globals::PATHS.project_path();

    let output = frontend::compile(&mut crate_store, &manifest)?;

    llvm::compile(output, root_path, &manifest)?;
    log::benchmark()
}

struct Output {
    mir_response: run_mir::MirResponse,
    hir_response: run_hir::HirResponse,
    faults: FaultCollector,
}
