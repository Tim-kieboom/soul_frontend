use ast::AstResponse;
use ast_parser::parse;
use soul_name_resolver::name_resolve;
use soul_tokenizer::TokenStream;
use soul_utils::{compile_options::CompilerOptions, sementic_level::SementicFault};

pub fn to_ast<'a>(
    token_stream: TokenStream<'a>,
    _options: &CompilerOptions,
    faults: &mut Vec<SementicFault>,
) -> AstResponse {
    let mut response = parse(token_stream, faults);
    name_resolve(&mut response, faults);
    response
}
