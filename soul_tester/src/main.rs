use crate::display::{
    ast::display_ast, benchmark::display_benchmark, fault::display_fault,
    tokenizer::display_tokenizer,
};
use anyhow::Result;
use ast_model::AstTree;
use ast_run::{AstRequest, to_ast};
use soul_tokenizer::{TokenStream, to_token_stream};
use soul_utils::{
    char_colors::{DEFAULT, GREEN, RED},
    collections::{benchmark::Benchmark, module_store::ModuleStore},
    span::ModuleId,
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
        Err(err) => eprintln!("{err}"),
    }
}

fn frontend(benchmark: &mut Benchmark) -> Result<bool> {
    let source_folder = config::CONFIG.source_path().to_path_buf();
    let main_path = source_folder.join("main.soul");
    let mut module_store = ModuleStore::new();

    let file = source_file(&main_path)?;
    let tokens = tokenize(&file, module_store.get_root_id())?;
    module_store.insert_root(main_path);

    let ast = ast(tokens, source_folder, benchmark, &mut module_store)?;
    for fault in ast.faults().iter() {
        display_fault(fault, &module_store, &config::PRINT_CONFIGS, &mut stdout())?;
    }

    let fail = ast.faults().fails(config::COMPILER_OPTIONS.fail_level);
    display_benchmark(&benchmark, &config::PRINT_CONFIGS, &mut stdout())?;
    Ok(!fail)
}

fn tokenize<'a>(file: &'a str, root: ModuleId) -> Result<TokenStream<'a>> {
    let tokens = match to_token_stream(&file, root) {
        Ok(val) => val,
        Err(err) => return Err(anyhow::anyhow!("in tokenizer: {err:?}")),
    };

    display_tokenizer(&tokens)?;
    Ok(tokens)
}

fn ast<'a>(
    tokens: TokenStream<'a>,
    source_folder: PathBuf,
    benchmark: &'a mut Benchmark,
    module_store: &'a mut ModuleStore,
) -> Result<AstTree> {
    let ast = to_ast(
        tokens,
        AstRequest {
            benchmark,
            module_store,
            source_folder,
        },
        &config::COMPILER_OPTIONS,
    );
    display_ast(&ast)?;
    Ok(ast)
}

fn source_file(path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path)
}
