use ast::{DeclareStore, ParseResponse, meta_data::AstMetadata};
use soul_tokenizer::TokenStream;
use soul_utils::sementic_level::SementicFault;

use crate::parser::Parser;
mod parser;

pub fn parse<'a, 'f>(tokens: TokenStream<'a>, faults: &'f mut Vec<SementicFault>) -> ParseResponse {
    let tree = Parser::parse(tokens, faults);

    ParseResponse {
        tree,
        store: DeclareStore::new(),
        meta_data: AstMetadata::new(),
    }
}
