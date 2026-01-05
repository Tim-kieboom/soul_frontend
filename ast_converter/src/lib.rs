use crate::steps::{
    parse::{self, parser::parse},
    sementic_analyser::name_resolution::name_resolver::NameResolver,
    tokenize::{self, tokenizer::tokenize},
};
use soul_ast::{
    ParseResonse,
    abstract_syntax_tree::AbstractSyntaxTree,
    sementic_models::{AstMetadata, SementicPass},
};
use soul_utils::{SementicFault, SementicLevel, SoulError};
use std::io::{self};

mod steps;
pub mod utils;

pub fn parse_file(source_file: &str) -> io::Result<ParseResonse> {
    let request = tokenize::Request {
        source: source_file,
    };
    
    let tokenize_response = tokenize(request);
    let parse::Response { mut parser } = parse(tokenize_response);

    Ok(sementic_analyse(
        parser.parse_tokens(),
        parser.comsume_errors(),
    ))
}

fn sementic_analyse(mut syntax_tree: AbstractSyntaxTree, errors: Vec<SoulError>) -> ParseResonse {
    let mut info = AstMetadata::new();
    NameResolver::new(&mut info).run(&mut syntax_tree);
    info.faults
        .extend(errors.into_iter().map(SementicFault::error));
    ParseResonse {
        syntax_tree,
        sementic_info: info,
    }
}
