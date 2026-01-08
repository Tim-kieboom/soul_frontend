use std::{fs::File, io::Read};

use anyhow::Result;
use soul_parser::{ParseResponse, parse};
use parser_models::syntax_display::{DisplayKind, SyntaxDisplay};
use soul_tokenizer::{TokenStream, tokenize};
use soul_utils::sementic_level::{SementicFault};

use crate::{convert_soul_error::{ToAnyhow, ToMessage}, paths::{Paths}};

mod paths;
mod convert_soul_error;

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

    let ParseResponse{tree, meta_data} = parse(token_stream);
    for fault in &meta_data.faults {
        eprintln!("{}", fault.to_message(&"main.soul", &source_file));
    }

    paths.write_to_output(
        tree.root.display(DisplayKind::Parser), 
        "ast.soulc",
    )?;

    if meta_data.faults.is_empty() {
        use soul_utils::char_colors::{GREEN, DEFAULT};
        println!("{GREEN}success!!{DEFAULT}")
    }

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
            SementicFault::error(err).to_anyhow(&paths.source_file, source_file)
        })?;

        sb.push_str(&format!("\tToken({})", token.kind.display()));
        sb.push_str(&format!(" >> Span({})", token.span.display()));
        sb.push_str(",\n");
    }

    sb.push_str("]");
    Ok(sb)
}

fn get_source_file(source_path: &String) -> Result<String> {
    let mut file = File::open(source_path)?;
    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;
    
    Ok(source_file)
}
