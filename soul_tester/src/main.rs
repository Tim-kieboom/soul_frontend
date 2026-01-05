use std::{fs::File, io::Read};

use anyhow::Result;
use soul_tokenizer::{TokenStream, tokenize};
use soul_utils::sementic_level::SementicLevel;

use crate::{convert_soul_error::ToAnyhow, paths::Paths};

pub mod paths;
pub mod convert_soul_error;

static PATHS: &[u8] = include_bytes!("../paths.json");

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    let source_file = get_source_file(&paths.source_file)?;

    let token_stream = tokenize(&source_file);
    let token_string = pretty_format_tokenizer(
        &paths, 
        &source_file, 
        token_stream.clone(),
    )?;

    paths.write_to_output(token_string, "tokenize.soulc")?;

    Ok(())
}

fn pretty_format_tokenizer<'a>(
    paths: &Paths, 
    source_file: &str, 
    token_stream: TokenStream<'a>,
) -> Result<String> {
    let mut sb = "[\n".to_string();

    for result in token_stream {
        let token = result.map_err(|err| {
            err.to_anyhow(SementicLevel::Error, &paths.source_file, source_file)
        })?;

        sb.push_str(&format!("Token({})", token.kind.display()));
        sb.push_str(&format!(" >> Span({})", token.span.display()));
        sb.push_str(",\n");
    }

    sb.push_str("\n]");
    Ok(sb)
}

fn get_source_file(source_path: &String) -> Result<String> {
    let mut file = File::open(source_path)?;
    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;
    
    Ok(source_file)
}
