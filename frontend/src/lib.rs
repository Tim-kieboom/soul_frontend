use std::io::{BufReader, Read};
use crate::{error::{ErrorKind, SoulError, SoulResult}, steps::tokenizer::{self, tokenize::tokenize}};

mod error;
mod steps;
mod soul_names;

pub fn compile_frontend<R: Read>(mut reader: BufReader<R>) -> SoulResult<()> {
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)
        .map_err(|err| SoulError::new(format!("while trying to read file, error: {}", err.to_string()), ErrorKind::InternalError, None))?;
    
    let request = tokenizer::Request{source: &buffer};
    let tokenizer::Response{token_stream: stream} = tokenize(request)?;

    println!("{:?}", stream.to_vec().map(|vec| vec.iter().map(|el| el.kind.clone()).collect::<Vec<_>>()));
    Ok(())
}
