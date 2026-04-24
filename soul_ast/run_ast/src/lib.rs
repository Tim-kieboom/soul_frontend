use ast::AbtractSyntaxTree;
use ast_parser::parse_module;
use soul_name_resolver::name_resolve;
use soul_tokenizer::TokenStream;
use soul_utils::{
    compile_options::CompilerOptions, crate_store::CrateContext, sementic_level::ModuleStore,
};
use std::path::PathBuf;

const ENTRY_MOD_NAME: &str = "crate";

pub fn to_ast<'a>(
    token_stream: TokenStream<'a>,
    _options: &CompilerOptions,
    module_store: &mut ModuleStore,
    context: &mut CrateContext,
    source_folder: PathBuf,
) -> AbtractSyntaxTree {
    let root = module_store.get_root_id();
    let mut ast = AbtractSyntaxTree::new(root);

    let name = ENTRY_MOD_NAME.to_string();
    let module = parse_module(
        token_stream,
        root,
        name,
        None,
        context,
        source_folder.clone(),
    );
    ast.modules.insert(root, module);

    name_resolve(root, module_store, context, &mut ast, source_folder);
    ast
}
