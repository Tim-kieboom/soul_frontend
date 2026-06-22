use std::path::PathBuf;

use ast_model::{AstModuleStore, AstStore};
use soul_tokenizer::TokenStream;
use soul_utils::{CrateContext, collections::module_store::ModuleStore, span::ModuleId};

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
    pub crate_source_folder: PathBuf,
    pub store: &'f mut AstStore,
    pub context: &'f mut CrateContext,
    pub modules: &'f mut ModuleStore,
    pub ast_modules: &'f mut AstModuleStore,
}

pub fn parse_module<'a, 'f>(input: TokenStream<'a>, name: String, info: ParseInfo<'f>) {
    Parser::parse(input, name, info)
}
