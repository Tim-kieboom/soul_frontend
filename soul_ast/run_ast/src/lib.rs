use ast::AstContext;
use ast_parser::parse;
use soul_name_resolver::name_resolve;
use soul_tokenizer::TokenStream;
use soul_utils::{compile_options::CompilerOptions, sementic_level::CompilerContext};

const ENTRY_MOD_NAME: &str = "crate";

pub fn to_ast<'a>(
    token_stream: TokenStream<'a>,
    _options: &CompilerOptions,
    context: &mut CompilerContext,
    ast_context: &mut AstContext,
) {
    let id = context.module_store.get_root_id();
    let name = ENTRY_MOD_NAME.to_string();
    let module = parse(token_stream, id, name, None, context);
    ast_context.modules.insert(id, module);

    name_resolve(id, context, ast_context);
}
