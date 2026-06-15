use crate::display::{ast::display_ast_tree, fault::display_fault, tokenizer::display_tokens};
use anyhow::Result;
use ast_model::{AbstractSyntaxTree, AstStore};
use ast_parser::ParseInfo;
use ast_run::to_ast;
use soul_tokenizer::{TokenStream, to_token_stream};
use soul_utils::{
    CrateContext, collections::module_store::ModuleStore, ids::IdAlloc, span::ModuleId,
};
use std::{
    fs::OpenOptions,
    io::{Write, stdout},
    path::{Path, PathBuf},
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
    let source_folder = PathBuf::from(&config::CONFIG.src_path);
    let module_store = ModuleStore::new(source_folder.clone());
    let module_id = ModuleId::begin();

    let file = std::fs::read_to_string(&config::CONFIG.main_path)?;
    let tokens = match to_token_stream(&file, module_id) {
        Ok(val) => val,
        Err(err) => return Err(anyhow::Error::msg(format!("{err:?}"))),
    };

    display_tokenizer(&tokens)?;

    let mut store = AstStore::default();
    let mut context = CrateContext::default();
    let info = ParseInfo {
        parent: None,
        id: module_id,
        source_folder,
        store: &mut store,
        context: &mut context,
    };
    let ast = to_ast(tokens, module_store, info, &config::COMPILER_OPTIONS);
    for fault in context.faults.iter() {
        display_fault(fault, &file, &config::PRINT_CONFIGS, &mut stdout())?;
    }
    
    display_ast(&ast, &store)?;
    Ok(())
}

fn display_tokenizer<'a>(tokens: &TokenStream<'a>) -> Result<()> {
    let mut output_path = Path::new(&config::CONFIG.output_path).join("tokenizer");
    output_path.push("tokens.soulc");

    std::fs::create_dir_all(&output_path.parent().expect("just joined a parent"))?;
    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_path)
        .map_err(|err| anyhow::anyhow!("Failed to create output file({output_path:?}): {}", err))?;

    display_tokens(tokens.clone(), &mut writer)?;
    writer.flush()?;
    Ok(())
}

fn display_ast<'a>(tree: &AbstractSyntaxTree, store: &AstStore) -> Result<()> {
    let mut output_path = Path::new(&config::CONFIG.output_path).join("ast");
    output_path.push("ast.soulc");

    std::fs::create_dir_all(&output_path.parent().expect("just joined a parent"))?;
    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_path)
        .map_err(|err| anyhow::anyhow!("Failed to create output file({output_path:?}): {}", err))?;

    display_ast_tree(tree, Path::new(&config::CONFIG.src_path), store, &mut writer)?;
    writer.flush()?;
    Ok(())
}
