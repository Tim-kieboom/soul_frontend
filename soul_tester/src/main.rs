use crate::{benchmark::Benchmark, display::{ast::display_ast_tree, benchmark::display_benchmark, fault::display_fault, tokenizer::display_tokens}};
use anyhow::Result;
use ast_model::AstTree;
use ast_run::to_ast;
use soul_tokenizer::{TokenStream, to_token_stream};
use soul_utils::{collections::module_store::ModuleStore, span::ModuleId};
use std::{
    fs::{File, OpenOptions}, io::stdout, path::{Path, PathBuf}, time::Instant,
};

mod config;
mod display;
mod benchmark;

fn main() {

    let mut benchmark = Benchmark::new();
    match frontend(&mut benchmark) {
        Ok(true) => println!("success"),
        Ok(false) => eprintln!("failed"),
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

    display_tokenizer(&tokens)
        .map_err(|err| anyhow::anyhow!("in display_tokenizer: {err}"))?;

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
    display_ast(&ast).map_err(|err| anyhow::anyhow!("in display_ast: {err}"))?;
    Ok(ast)
}

fn display_tokenizer<'a>(tokens: &TokenStream<'a>) -> Result<()> {
    let mut output_path = config::CONFIG.output_path().join("tokenizer");
    output_path.push("tokens.soulc");

    let mut writer = write_create_file(&output_path)?;
    display_tokens(tokens.clone(), &mut writer)?;
    Ok(())
}

fn display_ast<'a>(tree: &AstTree) -> Result<()> {
    let mut output_path = config::CONFIG.output_path().join("ast");
    output_path.push("tree.soulc");

    let mut writer = write_create_file(&output_path)?;
    display_ast_tree(tree, config::CONFIG.source_path(), &mut writer)?;
    Ok(())
}

fn write_create_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|err| anyhow::anyhow!("Failed to create output file({path:?}): {}", err))
}
