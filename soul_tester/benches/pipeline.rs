//! Benchmarks for the compiler frontend pipeline: tokenize -> parse -> name_resolve.
//!
//! Run with:
//!     cargo bench -p soul_tester
//!
//! Each stage is benchmarked in isolation (using `iter_batched` so setup work for
//! earlier stages isn't counted), plus one end-to-end benchmark via `ast_run::to_ast`.

use std::path::{Path, PathBuf};

use ast_model::AstTree;
use ast_parser::{ParseInfo, parse_module};
use ast_run::{AstRequest, to_ast};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use soul_name_resolver::name_resolve;
use soul_tokenizer::to_token_stream;
use soul_utils::{
    collections::{
        benchmark::Benchmark,
        crate_store::{CrateEntry, CrateStore, Manifest},
        module_store::ModuleStore,
    },
    compiler_options::CompilerOptions,
    fault::Severity,
};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    fail_level: Severity::Error,
};

struct Input {
    label: &'static str,
    source_folder: PathBuf,
    main_path: PathBuf,
    content: String,
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("soul")
}

/// Mirrors `soul_tester`'s own crate-store setup (see src/main.rs), pointed at the
/// `soul_tester/soul` fixture project so imports of `Std`/`Core` resolve.
fn build_crate_store(manifest_dir: &Path) -> CrateStore {
    let mut store = CrateStore::new();
    let Some(manifest) = Manifest::load_from_dir(manifest_dir) else {
        return store;
    };
    let Some(deps) = &manifest.dependencies else {
        return store;
    };
    for (name, spec) in deps {
        let Some(path_str) = &spec.path else {
            continue;
        };
        let dep_path = manifest_dir.join(path_str);
        let canonical = dep_path.canonicalize().unwrap_or(dep_path);
        let source_root = soul_utils::collections::crate_store::resolve_source_root(&canonical);
        store.insert(
            name.clone(),
            CrateEntry::new(name.clone(), source_root).with_linkage(spec.linkage),
        );
    }
    store
}

fn inputs() -> Vec<Input> {
    let root = fixture_root();
    let src = root.join("src");
    vec![
        Input {
            label: "tiny (main.soul, 5 lines)",
            source_folder: src.clone(),
            main_path: src.join("main.soul"),
            content: std::fs::read_to_string(src.join("main.soul")).unwrap(),
        },
        Input {
            label: "realistic (testCompiler.soul, 489 lines)",
            source_folder: src.clone(),
            main_path: src.join("testCompiler.soul"),
            content: std::fs::read_to_string(src.join("testCompiler.soul")).unwrap(),
        },
    ]
}

fn bench_tokenize(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenize");
    for input in inputs() {
        group.bench_function(input.label, |b| {
            b.iter_batched(
                || {
                    let mut module_store = ModuleStore::new();
                    module_store.insert_root(input.main_path.clone());
                    module_store
                },
                |module_store| {
                    let root = module_store.get_root_id();
                    to_token_stream(&input.content, root).expect("tokenize should succeed")
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_parse(c: &mut Criterion) {
    let crate_store = build_crate_store(&fixture_root());
    let mut group = c.benchmark_group("parse");
    for input in inputs() {
        group.bench_function(input.label, |b| {
            b.iter_batched(
                || {
                    let mut module_store = ModuleStore::new();
                    module_store.insert_root(input.main_path.clone());
                    let root = module_store.get_root_id();
                    let tokens =
                        to_token_stream(&input.content, root).expect("tokenize should succeed");
                    (module_store, tokens)
                },
                |(mut module_store, tokens)| {
                    let root = module_store.get_root_id();
                    let mut ast = AstTree::new(root);
                    let info = ParseInfo {
                        id: root,
                        crate_store: &crate_store,
                        parent: None,
                        modules: &mut module_store,
                        context: &mut ast.context,
                        forest: &mut ast.crates,
                        source_folder: input.source_folder.clone(),
                        crate_source_folder: input.source_folder.clone(),
                    };
                    parse_module(tokens, "crate".to_string(), info);
                    ast
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_resolve(c: &mut Criterion) {
    let crate_store = build_crate_store(&fixture_root());
    let mut group = c.benchmark_group("name_resolve");
    for input in inputs() {
        group.bench_function(input.label, |b| {
            b.iter_batched(
                || {
                    let mut module_store = ModuleStore::new();
                    module_store.insert_root(input.main_path.clone());
                    let root = module_store.get_root_id();
                    let tokens =
                        to_token_stream(&input.content, root).expect("tokenize should succeed");
                    let mut ast = AstTree::new(root);
                    let info = ParseInfo {
                        id: root,
                        crate_store: &crate_store,
                        parent: None,
                        modules: &mut module_store,
                        context: &mut ast.context,
                        forest: &mut ast.crates,
                        source_folder: input.source_folder.clone(),
                        crate_source_folder: input.source_folder.clone(),
                    };
                    parse_module(tokens, "crate".to_string(), info);
                    (module_store, ast)
                },
                |(mut module_store, mut ast)| {
                    name_resolve(&mut module_store, &mut ast, &crate_store);
                    ast
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let crate_store = build_crate_store(&fixture_root());
    let mut group = c.benchmark_group("full_pipeline (tokenize+parse+resolve)");
    for input in inputs() {
        group.bench_function(input.label, |b| {
            b.iter_batched(
                || {
                    let mut module_store = ModuleStore::new();
                    module_store.insert_root(input.main_path.clone());
                    module_store
                },
                // Tokenizing happens inside the timed routine (rather than in `setup`) so that
                // the `TokenStream`'s borrow of `input.content` and the fresh per-iteration
                // `ModuleStore`/`Benchmark` share one short-lived region — `to_ast`'s signature
                // ties all three to a single lifetime, which a setup/routine split can't satisfy.
                |mut module_store| {
                    let root = module_store.get_root_id();
                    let tokens =
                        to_token_stream(&input.content, root).expect("tokenize should succeed");
                    let mut benchmark = Benchmark::new();
                    let request = AstRequest {
                        source_folder: input.source_folder.clone(),
                        benchmark: &mut benchmark,
                        module_store: &mut module_store,
                        crate_store: &crate_store,
                    };
                    to_ast(tokens, request, &COMPILER_OPTIONS)
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_tokenize,
    bench_parse,
    bench_resolve,
    bench_full_pipeline
);
criterion_main!(benches);
