use parser_models::{AbstractSyntaxTree, meta_data::AstMetadata};
use soul_tokenizer::TokenStream;

use crate::parser::Parser;
mod parser;

pub struct ParseResponse {
    pub tree: AbstractSyntaxTree,
    pub meta_data: AstMetadata,
}

pub fn parse<'a>(tokens: TokenStream<'a>) -> ParseResponse {
    let (tree, faults) = Parser::parse(tokens);

    ParseResponse {
        tree,
        meta_data: AstMetadata::new(faults),
    }
}
