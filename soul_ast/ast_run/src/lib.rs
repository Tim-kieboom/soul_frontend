use std::path::PathBuf;

use ast_model::{AstTree};
use ast_parser::{ParseInfo, parse_module};
use soul_tokenizer::TokenStream;
use soul_utils::{collections::module_store::ModuleStore, compiler_options::CompilerOptions};

const ENTRY_MOD_NAME: &str = "crate";

pub fn to_ast<'a, 'f>(
    tokens: TokenStream<'a>,
    module_store: &mut ModuleStore,
    source_folder: PathBuf,
    _options: &CompilerOptions,
) -> AstTree {
    let root = module_store.get_root_id();
    let mut ast = AstTree::new(root);

    let name = ENTRY_MOD_NAME.to_string();
    let info = ParseInfo {
        parent: None,
        source_folder,
        store: &mut ast.store,
        context: &mut ast.context,
        id: module_store.get_root_id(),
        modules: module_store,
        ast_modules: &mut ast.modules,
    };
    parse_module(tokens, name, info);

    ast
}
