pub use crate::steps::sementic_analyser::sementic_fault::{SementicFault, SementicLevel};
use crate::steps::{
    parse::{self, parser::parse},
    sementic_analyser::{
        SemanticInfo, SementicPass, name_resolution::name_resolver::NameResolver,
    },
    tokenize::{self, tokenizer::tokenize},
};
use soul_ast::{abstract_syntax_tree::AbstractSyntaxTree, error::SoulError};
use std::io::{self};

mod steps;
pub mod utils;

pub struct ParseResonse {
    pub syntax_tree: AbstractSyntaxTree,
    pub sementic_info: SemanticInfo,
}
impl ParseResonse {
    pub fn get_fatal_count(&self, fatal_level: SementicLevel) -> usize {
        self.sementic_info.faults.iter()
            .filter(|fault| fault.is_fatal(fatal_level))
            .count()
    }
}

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
    let mut info = SemanticInfo::new();
    NameResolver::new(&mut info).run(&mut syntax_tree);
    info.faults.extend(
        errors.into_iter().map(|err| SementicFault::error(err))
    );
    ParseResonse {
        syntax_tree, 
        sementic_info: info, 
    }
}
