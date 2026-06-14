use std::path::PathBuf;

use ast_model::{AstStore, Module};
use soul_tokenizer::TokenStream;
use soul_utils::{CrateContext, span::ModuleId};

use crate::parser::Parser;

mod parse;
mod parser;
mod utils;

#[cfg(test)]
mod tests;

pub struct ParseInfo<'f> {
    id: ModuleId,
    name: String,
    parent: Option<ModuleId>,
    source_folder: PathBuf,
    store: &'f mut AstStore,
    context: &'f mut CrateContext,
}

pub fn parse_module<'a, 'f>(input: TokenStream<'a>, info: ParseInfo<'f>) -> Module {
    Parser::parse(input, info)
}
