use std::{
    fs::{File, OpenOptions},
    io::{Read, stdout},
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Error, Result};
use ast::AbtractSyntaxTree;
use fern::Dispatch;
use inkwell::context::Context;
use log::{error, info};
use paths::Paths;
use run_ast::to_ast;
use run_hir::{HirResponse, to_hir};
use run_mir::{MirResponse, extract_exports, to_mir};
use soul_ir::{IrRequest, to_llvm_ir};
use soul_tokenizer::to_token_stream;
use soul_utils::{
    CrateExports, CrateStore, ModuleId, SoulToml,
    char_colors::{DEFAULT, GREEN},
    compile_options::{Arch, CompilerOptions, Os, TargetInfo},
    crate_store::CrateContext,
    ids::IdAlloc,
    sementic_level::{FaultCollector, MessageConfig, ModuleStore, SementicLevel},
};

use crate::{convert_soul_error::ToMessage, paths::EntryFile};

mod convert_soul_error;
mod displayer_ast;
mod displayer_hir;
mod displayer_mir;
mod displayer_tokenizer;
mod paths;

static PATHS: &[u8] = include_bytes!("../paths.json");

pub const MESSAGE_CONFIG: MessageConfig = MessageConfig {
    backtrace: true,
    colors: true,
};

const OS: Os = Os::Windows;
const ARCH: Arch = Arch::X86_64;
const TARGET: TargetInfo = TargetInfo::new(ARCH, OS);
pub const COMPILER_OPTIONS: CompilerOptions = CompilerOptions::new_default(TARGET);

struct Output {
    mir_response: MirResponse,
    hir_response: HirResponse,
}

fn collect_all_exports(crate_store: &CrateStore) -> CrateExports {
    let mut all_exports = CrateExports::default();
    for krate in crate_store.values() {
        for (name, id) in &krate.exports.functions {
            all_exports.functions.insert(name.clone(), *id);
        }
        for (name, id) in &krate.exports.types {
            all_exports.types.insert(name.clone(), *id);
        }
    }
    all_exports
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    init_logger(&paths.log_file)?;

    let (manifest, mut crate_store) = paths.load_crates()?;

    let mut timer = Instant::now();
    compile_all_libs(&paths, &mut crate_store, &manifest)?;

    // Collect all exports from dependencies after compiling them
    let all_exports = collect_all_exports(&crate_store);

    let root_lib = &manifest.package.name;
    let source_path = Paths::to_source_path(paths.project_path())?;
    let entry_file = Paths::to_entry_file_path(paths.project_path())?;

    let mut module_store = ModuleStore::new(entry_file.path.clone());
    let mut context = CrateContext::new(entry_file.is_lib, MESSAGE_CONFIG);

    let mut output = run_crate_frontend(
        &paths,
        paths.project_path(),
        source_path,
        &entry_file,
        &mut module_store,
        &crate_store,
        &mut context,
        &all_exports,
    )?;

    log_faults(&context.faults, &module_store);
    if is_fatal(&context.faults, COMPILER_OPTIONS.fatal_level()) {
        return Ok(());
    }

    info!(
        "{GREEN}frontend success: {}ms{DEFAULT}",
        timer.elapsed().as_millis()
    );

    timer = Instant::now();
    if run_llvm(
        &mut output,
        paths.project_path(),
        &mut context.faults,
        root_lib,
    ) {
        info!(
            "{GREEN}llvm success: {}ms{DEFAULT}",
            timer.elapsed().as_millis()
        )
    }
    Ok(())
}

fn compile_all_libs(
    paths: &Paths,
    crate_store: &mut CrateStore,
    manifest: &SoulToml,
) -> Result<()> {
    let crate_info_list: Vec<_> = crate_store
        .values()
        .map(|data| (data.name.clone(), data.project_path.clone()))
        .collect();

    for (lib_name, project_path) in crate_info_list {
        if lib_name == manifest.package.name {
            continue;
        }

        let mut module_store = ModuleStore::new(project_path.clone());

        let source_path = Paths::to_source_path(&project_path)?;
        let entry_path = Paths::to_entry_file_path(&project_path)?;

        let mut context = CrateContext::new(entry_path.is_lib, MESSAGE_CONFIG);
        let output = run_crate_frontend(
            paths,
            &project_path,
            source_path,
            &entry_path,
            &mut module_store,
            crate_store,
            &mut context,
            &CrateExports::default(),
        )?;

        log_faults(&context.faults, &module_store);
        if is_fatal(&context.faults, COMPILER_OPTIONS.fatal_level()) {
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
    paths: &Paths,
    manifest: &Path,
    source: PathBuf,
    entry: &EntryFile,
    module_store: &mut ModuleStore,
    crate_store: &CrateStore,
    context: &mut CrateContext,
    crate_exports: &CrateExports,
) -> Result<Output> {
    let source_file = to_source_file(&entry.path)?;
    let root = module_store.get_root_id();
    let tokens = to_token_stream(&source_file, root);
    display_tokenizer(paths, manifest, root, &source_file)?;

    let ast = to_ast(
        tokens,
        &COMPILER_OPTIONS,
        module_store,
        context,
        crate_store,
        source,
    );
    display_ast(manifest, module_store, &ast)?;

    let mut hir = to_hir(&ast, &COMPILER_OPTIONS, context, crate_exports, root);
    display_hir(manifest, &hir, &ast)?;
    clear_hir_type_map(&mut hir);

    let mir = to_mir(&hir, &ast, &COMPILER_OPTIONS, context, crate_exports, root);
    display_mir(manifest, &mir, &hir, &ast)?;

    Ok(Output {
        mir_response: mir,
        hir_response: hir,
    })
}

fn run_llvm(
    output: &mut Output,
    manifest: &Path,
    faults: &mut FaultCollector,
    lib_name: &str,
) -> bool {
    let request = IrRequest {
        mir: &output.mir_response,
        types: &output.hir_response.typed,
        context: &Context::create(),
        crate_name: lib_name.to_string(),
    };

    faults.faults.clear();
    let ir = to_llvm_ir(&request, &COMPILER_OPTIONS, &mut faults.faults);
    log_faults(faults, &ModuleStore::new(PathBuf::new()));

    #[cfg(not(debug_assertions))]
    if ir.is_fatal {
        return false;
    }

    #[cfg(debug_assertions)]
    if ir.is_fatal {
        let llvm_code = ir.module.to_string();

        if let Err(err) = Paths::write_to_output(&llvm_code, manifest, Path::new("fatal_out.ll")) {
            error!("{err}");
        }
        return false;
    }

    let llvm_code = ir.module.to_string();

    if let Err(err) = Paths::write_to_output(&llvm_code, manifest, Path::new("out.ll")) {
        error!("{err}");
        return false;
    }

    true
}

fn display_tokenizer(
    paths: &Paths,
    manifest: &Path,
    module: ModuleId,
    source_file: &str,
) -> Result<()> {
    let token_stream = to_token_stream(source_file, module);
    let tokens = displayer_tokenizer::display_tokens(paths, source_file, token_stream)?;
    Paths::write_to_output(&tokens, manifest, Path::new("tokenizer\\tokens.soulc"))
}

fn display_ast(
    manifest: &Path,
    module_store: &ModuleStore,
    ast_context: &AbtractSyntaxTree,
) -> Result<()> {
    let root = module_store.get_root_id();
    Paths::write_to_output(
        &displayer_ast::display_ast(root, module_store, ast_context),
        manifest,
        Path::new("ast\\tree.soulc"),
    )?;
    Paths::write_to_output(
        &displayer_ast::display_ast_name_resolved(root, module_store, ast_context),
        manifest,
        Path::new("ast\\NameResolved.soulc"),
    )
}

fn display_hir(manifest: &Path, hir: &HirResponse, ast_context: &AbtractSyntaxTree) -> Result<()> {
    Paths::write_to_output(
        &displayer_hir::display_hir(ast_context, &hir.hir),
        manifest,
        Path::new("hir\\tree.soulc"),
    )?;
    Paths::write_to_output(
        &displayer_hir::display_thir(ast_context, &hir.hir, &hir.typed),
        manifest,
        Path::new("thir\\tree.soulc"),
    )?;
    Paths::write_to_output(
        &displayer_hir::display_created_types(&hir.hir, &hir.typed),
        manifest,
        Path::new("thir\\types.soulc"),
    )?;
    Ok(())
}

fn clear_hir_type_map(hir: &mut HirResponse) {
    hir.hir.info.types.clear();
    hir.hir.info.infers.clear();
}

fn display_mir(
    manifest: &Path,
    mir: &MirResponse,
    hir: &HirResponse,
    ast_context: &AbtractSyntaxTree,
) -> Result<()> {
    Paths::write_to_output(
        &displayer_mir::display_mir(&mir.tree, hir, &ast_context.modules),
        manifest,
        Path::new("mir\\tree.soulc"),
    )
}

fn to_source_file(source_path: &Path) -> Result<String> {
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

fn log_faults(faults: &FaultCollector, module_store: &ModuleStore) {
    let mut source_file = String::new();
    let mut current_module = ModuleId::error();

    for fault in &faults.faults {
        let module_id = match fault.get_soul_error().span {
            Some(val) => val.module,
            None => module_store.get_root_id(),
        };

        let path = match module_store.get_path(module_id) {
            Some(val) => val,
            None => &PathBuf::new(),
        };

        if module_id != current_module {
            source_file = to_source_file(path).unwrap_or_default();
            current_module = module_id;
        }

        error!("{}", fault.to_message(path, &source_file, MESSAGE_CONFIG));
    }
}

fn init_logger(log_file: &str) -> Result<()> {
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    Dispatch::new()
        .format(|out, message, _record| out.finish(format_args!("{}", message)))
        .level_for("soulc", log::LevelFilter::Info)
        .chain(stdout())
        .chain(log_file)
        .apply()?;

    Ok(())
}

fn is_fatal(faults: &FaultCollector, fatal_level: SementicLevel) -> bool {
    faults.faults.iter().any(|f| f.is_fatal(fatal_level))
}
