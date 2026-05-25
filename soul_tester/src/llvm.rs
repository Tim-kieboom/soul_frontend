use crate::paths::Paths;
use crate::{Output, globals, log};
use anyhow::Result;
use inkwell::context::Context;
use soul_utils::{
    SoulToml,
    char_colors::{DEFAULT, GREEN},
    sementic_level::ModuleStore,
};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

pub(crate) fn compile(output: Output, manifest: &Path, toml: &SoulToml) -> Result<bool> {
    let mut faults = output.faults;
    let request = soul_ir::IrRequest {
        mir: &output.mir_response,
        types: &output.hir_response.typed,
        context: &Context::create(),
        crate_name: toml.package.name.to_string(),
    };

    faults.faults.clear();

    let timer = Instant::now();
    let ir = soul_ir::to_llvm_ir(&request, &globals::COMPILER_OPTIONS, &mut faults.faults);
    globals::benchmark()?.ir = timer.elapsed();
    log::faults(&faults, &ModuleStore::new(PathBuf::new()));

    #[cfg(not(debug_assertions))]
    if ir.is_fatal {
        return Ok(false);
    }

    #[cfg(debug_assertions)]
    if ir.is_fatal {
        let llvm_code = ir.module.to_string();

        Paths::write_to_output(&llvm_code, manifest, Path::new("fatal_out.ll"))?;
        return Ok(false);
    }

    let llvm_code = ir.module.to_string();

    Paths::write_to_output(&llvm_code, manifest, Path::new("out.ll"))?;
    logger::info!("{GREEN}llvm success{DEFAULT}");
    Ok(true)
}
