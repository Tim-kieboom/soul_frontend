use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::globals;
use crate::log;
use crate::{
    Output,
    displayers::{display_ast, display_hir, display_mir, display_tokenizer},
    paths::{EntryFile, Paths},
};
use anyhow::{Error, Result};
use run_ast::to_ast;
use run_hir::{HirResponse, to_hir};
use run_mir::{extract_exports, to_mir};
use soul_tokenizer::to_token_stream;
use soul_utils::{
    CrateContext, CrateExports, CrateId, CrateStore, SoulToml,
    char_colors::{DEFAULT, GREEN},
    sementic_level::ModuleStore,
};

pub(crate) fn compile(crate_store: &mut CrateStore, manifest: &SoulToml) -> Result<Output> {
    compile_all_libs(crate_store, manifest)?;

    let entry_file = Paths::to_entry_file_path(globals::PATHS.project_path())?;
    let context = CrateContext::new(entry_file.is_lib, globals::MESSAGE_CONFIG);

    compiler_root_crate(entry_file, crate_store, context)
}

pub(crate) fn to_source_file(source_path: &Path) -> Result<String> {
    let mut file = match File::open(source_path) {
        Ok(val) => val,
        Err(err) => {
            return Err(Error::msg(format!(
                "tried to open path '{source_path:?}' but got error: {err}"
            )));
        }
    };

    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;
    Ok(source_file)
}

fn compiler_root_crate(
    entry_file: EntryFile,
    crate_store: &mut CrateStore,
    context: CrateContext,
) -> Result<Output> {
    let source_path = Paths::to_source_path(globals::PATHS.project_path())?;
    let mut module_store = ModuleStore::new(entry_file.path.clone());

    let all_exports = collect_all_crate_exports(&crate_store);

    let output = run_crate_frontend(
        crate_store.main_crate(),
        globals::PATHS.project_path(),
        source_path,
        &entry_file,
        &mut module_store,
        crate_store,
        context,
        &all_exports,
    )?;

    log::faults(&output.faults, &module_store);
    if log::is_fatal(&output.faults, globals::COMPILER_OPTIONS.fatal_level()) {
        return Err(anyhow::Error::msg("compiler got fatal compile error"));
    }
    logger::info!("{GREEN}frontend success{DEFAULT}",);
    Ok(output)
}

fn compile_all_libs(crate_store: &mut CrateStore, manifest: &SoulToml) -> Result<()> {
    let crate_info_list: Vec<_> = crate_store
        .entries()
        .map(|(id, data)| (id, data.name.clone(), data.project_path.clone()))
        .collect();

    for (crate_id, lib_name, project_path) in crate_info_list {
        if lib_name == manifest.package.name {
            continue;
        }

        let mut module_store = ModuleStore::new(project_path.clone());

        let source_path = Paths::to_source_path(&project_path)?;
        let entry_path = Paths::to_entry_file_path(&project_path)?;

        let context = CrateContext::new(entry_path.is_lib, globals::MESSAGE_CONFIG);
        let output = run_crate_frontend(
            crate_id,
            &project_path,
            source_path,
            &entry_path,
            &mut module_store,
            crate_store,
            context,
            &CrateExports::default(),
        )?;

        log::faults(&output.faults, &module_store);
        if log::is_fatal(&output.faults, globals::COMPILER_OPTIONS.fatal_level()) {
            continue;
        }

        let exports = extract_exports(&output.mir_response);

        if let Some(crate_mut) = crate_store.get_mut_by_name(&lib_name) {
            crate_mut.exports = exports;
        }
    }

    Ok(())
}

fn run_crate_frontend(
    crate_id: CrateId,
    manifest: &Path,
    source: PathBuf,
    entry: &EntryFile,
    module_store: &mut ModuleStore,
    crate_store: &CrateStore,
    mut context: CrateContext,
    crate_exports: &CrateExports,
) -> Result<Output> {
    let timer = Instant::now();
    let source_file = to_source_file(&entry.path)?;
    globals::benchmark()?.source_read(crate_id, timer.elapsed());

    let root = module_store.get_root_id();

    let timer = Instant::now();
    let tokens = to_token_stream(&source_file, root);
    display_tokenizer(&globals::PATHS, manifest, root, &source_file)?;
    globals::benchmark()?.tokenize(crate_id, timer.elapsed());

    let timer = Instant::now();
    let ast = to_ast(
        tokens,
        &globals::COMPILER_OPTIONS,
        module_store,
        &mut context,
        crate_store,
        source.clone(),
    );
    globals::benchmark()?.ast(crate_id, timer.elapsed());
    display_ast(manifest, module_store, &ast)?;

    let timer = Instant::now();
    let mut hir = to_hir(
        &ast,
        &globals::COMPILER_OPTIONS,
        &mut context,
        crate_exports,
        root,
        source.clone(),
    );
    display_hir(manifest, &hir, &ast)?;
    globals::benchmark()?.hir(crate_id, timer.elapsed());
    clear_hir_type_map(&mut hir);

    let timer = Instant::now();
    let mir = to_mir(
        &hir,
        &ast,
        &globals::COMPILER_OPTIONS,
        &mut context,
        crate_exports,
        root,
    );
    globals::benchmark()?.mir(crate_id, timer.elapsed());
    display_mir(manifest, &mir, &hir, &ast)?;

    Ok(Output {
        mir_response: mir,
        hir_response: hir,
        faults: context.faults,
    })
}

fn collect_all_crate_exports(crate_store: &CrateStore) -> CrateExports {
    let mut all_exports = CrateExports::default();
    for crate_ in crate_store.values() {
        for (name, id) in &crate_.exports.functions {
            all_exports.functions.insert(name.clone(), *id);
        }
        for (name, id) in &crate_.exports.types {
            all_exports.types.insert(name.clone(), *id);
        }
    }
    all_exports
}

fn clear_hir_type_map(hir: &mut HirResponse) {
    hir.hir.info.types.clear();
    hir.hir.info.infers.clear();
}
