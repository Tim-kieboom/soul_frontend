use itertools::Itertools;
use std::io::{self, BufReader, Read};
use models::{abstract_syntax_tree::AbstractSyntaxTree, error::{SoulError}};
use crate::steps::{parse::{self, parser::parse}, tokenize::{self, tokenizer::tokenize}};

mod steps;
mod utils;

pub struct ParseResonse {
    pub syntax_tree: AbstractSyntaxTree, 
    pub errors: Vec<SoulError>,
}

pub fn parse_file<R: Read>(mut reader: BufReader<R>) -> io::Result<ParseResonse> {

    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    
    let request = tokenize::Request{source: &buffer};
    let tokenize_response = tokenize(request);

    #[cfg(debug_assertions)] {
        let tokens = tokenize_response.token_stream.clone()
            .to_vec()
            .map(|vec| vec.into_iter().enumerate().map(|(i, el)| format!("{}.{:?}", i+1, el.kind)).join("\n\t"));
        
        match tokens {
            Ok(tokens) => println!("[\n\t{tokens}\n]"),
            Err(err) => eprintln!("{}", err.to_message()),
        }
    }

    let parse::Response{mut parser} = parse(tokenize_response); 
    
    Ok(
        ParseResonse {
            syntax_tree: parser.parse_tokens(),
            errors: parser.comsume_errors(),
        }
    )
}

pub fn sementic_analyse(_syntax_tree: &AbstractSyntaxTree) -> Vec<SoulError> {
    todo!("impl sementic analyser")
}
