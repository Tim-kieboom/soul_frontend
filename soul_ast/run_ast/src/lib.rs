use ast::AstResponse;
use ast_parser::parse;
use soul_name_resolver::name_resolve;
use soul_tokenizer::TokenStream;
use soul_utils::{compile_options::CompilerOptions, sementic_level::CompilerContext};

pub fn to_ast<'a>(
    token_stream: TokenStream<'a>,
    _options: &CompilerOptions,
    context: &mut CompilerContext,
    source_file: Option<std::path::PathBuf>,
) -> AstResponse {
    let mut response = parse(token_stream, context, source_file);
    name_resolve(&mut response, context.module_store.get_root_id(), context);
    response
}
