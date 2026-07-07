use std::{path::PathBuf, time::Instant};

use ast_model::AstTree;
use ast_parser::{ParseInfo, parse_module};
use soul_name_resolver::name_resolve;
use soul_tokenizer::TokenStream;
use soul_utils::{
    collections::{benchmark::Benchmark, module_store::ModuleStore},
    compiler_options::CompilerOptions,
};

const ENTRY_MOD_NAME: &str = "crate";

pub struct AstRequest<'a> {
    pub source_folder: PathBuf,
    pub benchmark: &'a mut Benchmark,
    pub module_store: &'a mut ModuleStore,
}

pub fn to_ast<'a, 'f>(
    tokens: TokenStream<'a>,
    request: AstRequest<'a>,
    _options: &CompilerOptions,
) -> AstTree {
    let AstRequest {
        source_folder,
        benchmark,
        module_store,
    } = request;
    let root = module_store.get_root_id();
    let mut ast = AstTree::new(root);

    let name = ENTRY_MOD_NAME.to_string();
    let info = ParseInfo {
        parent: None,
        source_folder: source_folder.clone(),
        crate_source_folder: source_folder.clone(),
        store: &mut ast.store,
        context: &mut ast.context,
        id: module_store.get_root_id(),
        modules: module_store,
        ast_modules: &mut ast.modules,
    };

    let time = Instant::now();
    parse_module(tokens, name, info);
    benchmark.add_benchmark("ast", time.elapsed());

    let time = Instant::now();
    name_resolve(module_store, &mut ast);
    benchmark.add_benchmark("name_resolve", time.elapsed());
    ast
}
