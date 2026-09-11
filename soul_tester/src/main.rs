use crate::display::{
    ast::display_ast, benchmark::display_benchmark, fault::display_fault, fault_to_anyhow_error,
    mir::display_mir, tokenizer::display_tokenizer,
};
use anyhow::Result;
use ast_model::AstTree;
use ast_run::{AstRequest, to_ast};
use inkwell::context::Context;
use mir_codegen::to_llvm;
use mir_model::MirProgram;
use soul_tokenizer::{TokenStream, to_token_stream};
use soul_utils::{
    CrateContext,
    char_colors::{DEFAULT, GREEN, RED},
    collections::{
        benchmark::Benchmark,
        crate_store::{CrateEntry, CrateStore, Manifest, resolve_source_root},
        module_store::ModuleStore,
    },
    fault::FaultCollector,
};

use std::{
    io::{self, stdout},
    path::{Path, PathBuf},
};

mod config;
mod display;

fn main() {
    match frontend(&mut Benchmark::new()) {
        Ok(true) => println!("{GREEN}success{DEFAULT}"),
        Ok(false) => eprintln!("{RED}failed{DEFAULT}"),
        Err(err) => eprintln!("{RED}!!error!!: {err}{DEFAULT}"),
    }
}

fn frontend(benchmark: &mut Benchmark) -> Result<bool> {
    let source_folder = config::CONFIG.source_path().to_path_buf();
    let main_path = config::CONFIG.to_main_path();
    let mut module_store = ModuleStore::new();

    let crate_store = build_crate_store(&source_folder);

    let file = source_file(&main_path)?;
    module_store.insert_root(main_path);
    let tokens = tokenize(&file, &module_store)?;

    let mut ast = ast(
        tokens,
        AstRequest {
            benchmark,
            source_folder,
            crate_store: &crate_store,
            module_store: &mut module_store,
        },
    )?;

    let mut all_faults = ast.drain_faults().into_unclassified();

    let ast_failed = failed(&all_faults);
    let mir = if ast_failed {
        MirProgram::empty()
    } else {
        mir(&ast, benchmark, &mut all_faults)
    };
    display_mir(&mir, &ast.crates.store)?;

    if !ast_failed {
        codegen(&mir, &ast)?;
    }

    for fault in all_faults.iter() {
        display_fault(fault, &module_store, &config::PRINT_CONFIGS, &mut stdout())?;
    }

    display_benchmark(benchmark, &config::PRINT_CONFIGS, &mut stdout())?;
    Ok(!ast_failed)
}

fn build_crate_store(source_folder: &Path) -> CrateStore {
    let mut store = CrateStore::new();

    let Some(manifest_dir) = find_manifest_dir(source_folder) else {
        return store;
    };

    let Some(manifest) = Manifest::load_from_dir(&manifest_dir) else {
        return store;
    };

    let Some(dependencies) = &manifest.dependencies else {
        return store;
    };

    for (name, spec) in dependencies {
        let Some(path_str) = &spec.path else {
            continue;
        };

        let dependencie_path = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            manifest_dir.join(path_str)
        };

        let canonical = dependencie_path.canonicalize().unwrap_or(dependencie_path);
        let source_root = resolve_source_root(&canonical);
        store.insert(
            name.clone(),
            CrateEntry::new(name.clone(), source_root).apply_linkage(spec.linkage),
        );
    }
    store
}

fn find_manifest_dir(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start.to_path_buf());
    while let Some(dir) = current {
        if dir.join("Soul.toml").is_file() {
            return Some(dir);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

fn failed(faults: &FaultCollector) -> bool {
    faults.fails(config::COMPILER_OPTIONS.fail_level)
}

fn source_file(path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path)
}

fn tokenize<'a>(file: &'a str, modules: &ModuleStore) -> Result<TokenStream<'a>> {
    let tokens = to_token_stream(file, modules.get_root_id())
        .map_err(|f| fault_to_anyhow_error(&f, modules))?;

    display_tokenizer(&tokens, modules)?;
    Ok(tokens)
}

fn ast<'a>(tokens: TokenStream<'a>, request: AstRequest<'a>) -> Result<AstTree> {
    let ast = to_ast(tokens, request, &config::COMPILER_OPTIONS);
    display_ast(&ast)?;
    Ok(ast)
}

fn mir(ast: &AstTree, benchmark: &mut Benchmark, all_faults: &mut FaultCollector) -> MirProgram {
    let mut mir_context = CrateContext::default();
    let mir_program = mir_run::to_mir(ast, benchmark, &mut mir_context, &config::COMPILER_OPTIONS);
    all_faults.extend_into(mir_context.faults);
    mir_program
}

/// Codegen failures are reported but non-fatal — this is the first (smallest-
/// slice) codegen pass, so plenty of otherwise-valid MIR (e.g. anything using
/// `f64`) isn't supported yet, the same way MIR faults don't gate the overall
/// AST-level pass/fail. See `mir_codegen`'s module docs for what's in scope.
fn codegen(mir: &MirProgram, ast: &AstTree) -> Result<()> {
    match to_llvm(&Context::create(), mir, ast, &config::COMPILER_OPTIONS) {
        Ok(module) => {
            let output_path = config::CONFIG.output_path().join("codegen");
            std::fs::create_dir_all(&output_path)?;
            module
                .print_to_file(output_path.join("module.ll"))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Err(err) => {
            eprintln!("{RED}codegen skipped, error: {}{DEFAULT}", err.message());
        }
    }
    Ok(())
}
