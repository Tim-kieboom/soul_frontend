use crate::steps::{
    parse::{self, parser::parse},
    sementic_analyser::{
        SementicInfo, SementicPass, name_resolution::name_resolver::NameResolver,
        type_resolution::type_resolver::TypeResolver,
    },
    tokenize::{self, tokenizer::tokenize},
};
use models::{abstract_syntax_tree::AbstractSyntaxTree, error::SoulError};
use std::io::{self};

pub use crate::steps::sementic_analyser::sementic_fault::{SementicFault, SementicLevel};
mod steps;
pub mod utils;

pub struct ParseResonse {
    pub syntax_tree: AbstractSyntaxTree,
    pub errors: Vec<SoulError>,
}

pub fn parse_file(source_file: &str) -> io::Result<ParseResonse> {
    let request = tokenize::Request {
        source: source_file,
    };
    let tokenize_response = tokenize(request);

    #[cfg(debug_assertions)]
    {
        use crate::utils::convert_error_message::ToMessage;
        use itertools::Itertools;

        match tokenize_response.token_stream.clone().to_vec().map(|vec| {
            vec.into_iter()
                .enumerate()
                .map(|(i, el)| format!("{}.{:?}", i + 1, el.kind))
                .join("\n\t")
        }) {
            Ok(tokens) => println!("[\n\t{tokens}\n]"),
            Err(err) => eprintln!(
                "{}",
                err.to_message(
                    SementicLevel::Debug,
                    "(filepath hardcoded) main.soul",
                    source_file
                )
            ),
        }
    }

    let parse::Response { mut parser } = parse(tokenize_response);

    Ok(ParseResonse {
        syntax_tree: parser.parse_tokens(),
        errors: parser.comsume_errors(),
    })
}

pub fn sementic_analyse(syntax_tree: &mut AbstractSyntaxTree) -> Vec<SementicFault> {
    let mut info = SementicInfo::new();
    NameResolver::new(&mut info).run(syntax_tree);
    TypeResolver::new(&mut info).run(syntax_tree);
    info.consume_faults()
}
