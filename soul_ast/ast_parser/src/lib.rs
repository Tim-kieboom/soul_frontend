use ast::Module;
use soul_tokenizer::TokenStream;
use soul_utils::{crate_store::CrateContext, span::ModuleId};
use std::path::PathBuf;

use crate::parser::Parser;
mod parser;

pub fn parse_module<'a, 'f>(
    tokens: TokenStream<'a>,
    id: ModuleId,
    name: String,
    parent: Option<ModuleId>,
    context: &'f mut CrateContext,
    source_folder: PathBuf,
) -> Module {
    Parser::parse(tokens, id, name, parent, context, source_folder)
}
