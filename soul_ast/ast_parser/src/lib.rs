use ast::{AstResponse, DeclareStore, meta_data::AstMetadata};
use soul_tokenizer::TokenStream;
use soul_utils::{ids::IdGenerator, sementic_level::CompilerContext};

use crate::parser::Parser;
mod parser;

pub fn parse<'a, 'f>(
    tokens: TokenStream<'a>,
    faults: &'f mut CompilerContext,
    source_file: Option<std::path::PathBuf>,
) -> AstResponse {
    let tree = Parser::parse(tokens, faults);

    AstResponse {
        tree,
        source_file,
        store: DeclareStore::new(),
        meta_data: AstMetadata::new(),
        function_generators: IdGenerator::new(),
    }
}
