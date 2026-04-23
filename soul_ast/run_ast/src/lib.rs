use ast::AbtractSyntaxTree;
use ast_parser::parse_module;
use soul_name_resolver::name_resolve;
use soul_tokenizer::TokenStream;
use soul_utils::{compile_options::CompilerOptions, sementic_level::CompilerContext};

const ENTRY_MOD_NAME: &str = "crate";

pub fn to_ast<'a>(
    token_stream: TokenStream<'a>,
    _options: &CompilerOptions,
    context: &mut CompilerContext,
) -> AbtractSyntaxTree {
    let root = context.root_module_id();
    let mut ast = AbtractSyntaxTree::new(root);

    let name = ENTRY_MOD_NAME.to_string();
    let module = parse_module(token_stream, root, name, None, context);
    ast.modules.insert(root, module);

    name_resolve(root, context, &mut ast);
    ast
}
