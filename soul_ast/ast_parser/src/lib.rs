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
    pub id: ModuleId,
    pub parent: Option<ModuleId>,
    pub source_folder: PathBuf,
    pub store: &'f mut AstStore,
    pub context: &'f mut CrateContext,
}

pub fn parse_module<'a, 'f>(input: TokenStream<'a>, name: String, info: ParseInfo<'f>) -> Module {
    Parser::parse(input, name, info)
}
