use crate::steps::{
    parse::{self, parser::parse},
<<<<<<< Updated upstream
    sementic_analyser::{name_resolution::name_resolver::NameResolver},
=======
    sementic_analyser::{SementicPass, name_resolution::name_resolver::NameResolver},
>>>>>>> Stashed changes
    tokenize::{self, tokenizer::tokenize},
};
use soul_ast::{ParseResonse, abstract_syntax_tree::AbstractSyntaxTree, error::SoulError, sementic_models::{ASTSemanticInfo, SementicPass, sementic_fault::{SementicFault, SementicLevel}}};
use std::io::{self};

mod steps;
pub mod utils;
pub use crate::steps::sementic_analyser::AstMetaData;

<<<<<<< Updated upstream
=======
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParseResonse {
    pub syntax_tree: AbstractSyntaxTree,
    pub sementic_info: AstMetaData,
}
impl ParseResonse {
    pub fn get_fatal_count(&self, fatal_level: SementicLevel) -> usize {
        self.sementic_info
            .faults
            .iter()
            .filter(|fault| fault.is_fatal(fatal_level))
            .count()
    }
}

>>>>>>> Stashed changes
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
<<<<<<< Updated upstream
    let mut info = ASTSemanticInfo::new();
=======
    let mut info = AstMetaData::new();
>>>>>>> Stashed changes
    NameResolver::new(&mut info).run(&mut syntax_tree);
    info.faults
        .extend(errors.into_iter().map(|err| SementicFault::error(err)));
    ParseResonse {
        syntax_tree,
        sementic_info: info,
    }
}
