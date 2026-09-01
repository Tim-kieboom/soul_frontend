use std::path::PathBuf;

use ast_model::CrateForest;
use soul_tokenizer::TokenStream;
use soul_utils::{
    CrateContext,
    collections::{crate_store::CrateStore, module_store::ModuleStore},
    span::ModuleId,
};

use crate::parser::Parser;

mod parse;
mod parser;
mod utils;

#[cfg(test)]
mod tests;

pub struct ParseInfo<'f> {
    pub id: ModuleId,
    pub source_folder: PathBuf,
    pub parent: Option<ModuleId>,
    pub crate_source_folder: PathBuf,

    pub modules: &'f mut ModuleStore,
    pub context: &'f mut CrateContext,
    pub forest: &'f mut CrateForest,
    pub crate_store: &'f CrateStore,
}

pub fn parse_module<'a, 'f>(input: TokenStream<'a>, name: String, info: ParseInfo<'f>) {
    Parser::parse(input, name, info)
}
