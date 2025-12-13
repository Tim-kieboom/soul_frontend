use crate::steps::{
    parse::{self, parser::parse},
    tokenize::{self, tokenizer::tokenize},
};
use models::{abstract_syntax_tree::AbstractSyntaxTree, error::SoulError};
use std::io::{self};

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
        use itertools::Itertools;

        use crate::utils::convert_error_message::{Level, ToMessage};

        match tokenize_response.token_stream.clone().to_vec().map(|vec| {
            vec.into_iter()
                .enumerate()
                .map(|(i, el)| format!("{}.{:?}", i + 1, el.kind))
                .join("\n\t")
        }) {
            Ok(tokens) => println!("[\n\t{tokens}\n]"),
            Err(err) => eprintln!(
                "{}",
                err.to_message(Level::Debug, "(filepath hardcoded) main.soul", source_file)
            ),
        }
    }

    let parse::Response { mut parser } = parse(tokenize_response);

    Ok(ParseResonse {
        syntax_tree: parser.parse_tokens(),
        errors: parser.comsume_errors(),
    })
}

pub fn sementic_analyse(_syntax_tree: &AbstractSyntaxTree) -> Vec<SoulError> {
    todo!("impl sementic analyser")
}
