use crate::display::{ast::display_ast_tree, fault::display_fault, tokenizer::display_tokens};
use anyhow::{Result};
use ast_model::AbstractSyntaxTree;
use ast_run::to_ast;
use soul_tokenizer::{TokenStream, to_token_stream};
use soul_utils::collections::module_store::ModuleStore;
use std::{
    fs::{File, OpenOptions},
    io::stdout,
    path::Path,
};

mod config;
mod display;

fn main() {
    match frontend() {
        Ok(()) => println!("success"),
        Err(err) => eprintln!("{err}"),
    }
}

fn frontend() -> Result<()> {
    let source_folder = config::CONFIG.source_path().to_path_buf();
    let main_path = source_folder.join("main.soul");
    let mut module_store = ModuleStore::new();

    let file = std::fs::read_to_string(&main_path)?;
    let tokens = to_token_stream(&file, module_store.get_root_id())
        .map_err(|err| anyhow::anyhow!("in tokenizer: {err:?}"))?;

    module_store.insert_root(main_path);
    display_tokenizer(&tokens).map_err(|err| anyhow::anyhow!("in display_tokenizer: {err}"))?;

    let ast = to_ast(
        tokens,
        module_store,
        source_folder,
        &config::COMPILER_OPTIONS,
    );
    for fault in ast.faults() {
        display_fault(fault, &file, &config::PRINT_CONFIGS, &mut stdout())?;
    }

    display_ast(&ast).map_err(|err| anyhow::anyhow!("in display_ast: {err}"))?;

    Ok(())
}

fn display_tokenizer<'a>(tokens: &TokenStream<'a>) -> Result<()> {
    let mut output_path = config::CONFIG.output_path().join("tokenizer");
    output_path.push("tokens.soulc");

    let mut writer = write_create_file(&output_path)?;
    display_tokens(tokens.clone(), &mut writer)?;
    Ok(())
}

fn display_ast<'a>(tree: &AbstractSyntaxTree) -> Result<()> {
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
