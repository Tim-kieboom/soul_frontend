use ast::Module;
use soul_tokenizer::TokenStream;
use soul_utils::{sementic_level::CompilerContext, span::ModuleId};

use crate::parser::Parser;
mod parser;

pub fn parse_module<'a, 'f>(
    tokens: TokenStream<'a>,
    id: ModuleId,
    name: String,
    parent: Option<ModuleId>,
    context: &'f mut CompilerContext,
) -> Module {
    Parser::parse(tokens, id, name, parent, context)
}
