use ast::{DeclareStore, AstResponse, meta_data::AstMetadata};
use soul_tokenizer::TokenStream;
use soul_utils::{ids::{IdGenerator}, sementic_level::SementicFault};

use crate::parser::Parser;
mod parser;

pub fn parse<'a, 'f>(tokens: TokenStream<'a>, faults: &'f mut Vec<SementicFault>) -> AstResponse {
    let tree = Parser::parse(tokens, faults);

    AstResponse {
        tree,
        store: DeclareStore::new(),
        meta_data: AstMetadata::new(),
        function_generators: IdGenerator::new(),
    }
}
