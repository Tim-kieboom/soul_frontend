use ast::{meta_data::AstMetadata, AstResponse, DeclareStore};
use soul_tokenizer::TokenStream;
use soul_utils::{ids::IdGenerator, sementic_level::SementicFault};

use crate::parser::Parser;
mod parser;

pub fn parse<'a, 'f>(
    tokens: TokenStream<'a>,
    faults: &'f mut Vec<SementicFault>,
    source_file: Option<std::path::PathBuf>,
) -> AstResponse {
    let tree = Parser::parse(tokens, faults);

    AstResponse {
        tree,
        store: DeclareStore::new(),
        meta_data: AstMetadata::new(),
        function_generators: IdGenerator::new(),
        source_file,
    }
}
