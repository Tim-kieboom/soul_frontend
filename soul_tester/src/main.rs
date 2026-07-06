use crate::{benchmark::Benchmark, display::{ast::display_ast, benchmark::display_benchmark, fault::display_fault, tokenizer::display_tokenizer}};
use anyhow::Result;
use ast_model::AstTree;
use ast_run::to_ast;
use soul_tokenizer::{TokenStream, to_token_stream};
use soul_utils::{char_colors::{GREEN, RED, DEFAULT}, collections::module_store::ModuleStore, span::ModuleId};
use std::{
    io::stdout, path::PathBuf, time::Instant,
};

mod config;
mod display;
mod benchmark;

fn main() {
    let mut benchmark = Benchmark::new();
    match frontend(&mut benchmark) {
        Ok(true) => println!("{GREEN}success{DEFAULT}"),
        Ok(false) => eprintln!("{RED}failed{DEFAULT}"),
        Err(err) => eprintln!("{err}"),
    }
}

fn frontend(benchmark: &mut Benchmark) -> Result<bool> {
    let source_folder = config::CONFIG.source_path().to_path_buf();
    let main_path = source_folder.join("main.soul");
    let mut module_store = ModuleStore::new();
    
    let file = std::fs::read_to_string(&main_path)?;    
    let tokens = tokenize(&file, module_store.get_root_id())?;
    module_store.insert_root(main_path);

    let ast = ast(tokens, source_folder, benchmark, &mut module_store)?;
    if ast.faults().fails(config::COMPILER_OPTIONS.fail_level) {
        for fault in ast.faults().iter() {
            display_fault(fault, &file, &module_store, &config::PRINT_CONFIGS, &mut stdout())?;
        }

        display_benchmark(&benchmark, &config::PRINT_CONFIGS, &mut stdout())?;
        return Ok(false);
    }

    display_benchmark(&benchmark, &config::PRINT_CONFIGS, &mut stdout())?;
    Ok(true)
}

fn tokenize<'a>(file: &'a str, root: ModuleId) -> Result<TokenStream<'a>> {
    let tokens = to_token_stream(&file, root)
        .map_err(|err| anyhow::anyhow!("in tokenizer: {err:?}"))?;

    display_tokenizer(&tokens)?;
    Ok(tokens)
}

fn ast<'a>(tokens: TokenStream<'a>, source_folder: PathBuf, benchmark: &mut Benchmark, module_store: &mut ModuleStore) -> Result<AstTree> {
    let ast_timer = Instant::now();
    let ast = to_ast(
        tokens,
        module_store,
        source_folder,
        &config::COMPILER_OPTIONS,
    );
    benchmark.set_ast(ast_timer);
    display_ast(&ast)?;
    Ok(ast)
}


