use ast_model::AbstractSyntaxTree;
use ast_parser::{ParseInfo, parse_module};
use soul_tokenizer::TokenStream;
use soul_utils::{collections::module_store::ModuleStore, compiler_options::CompilerOptions};

const ENTRY_MOD_NAME: &str = "crate";

pub fn to_ast<'a, 'f>(
    tokens: TokenStream<'a>,
    module_store: ModuleStore,
    info: ParseInfo<'f>,
    _options: &CompilerOptions,
) -> AbstractSyntaxTree {
    let root = module_store.get_root_id();
    let mut ast = AbstractSyntaxTree::new(root);

    let name = ENTRY_MOD_NAME.to_string();
    let module = parse_module(tokens, name, info);
    ast.modules.insert(root, module);

    ast
}
