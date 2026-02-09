use std::{fs::File, io::Read};

use anyhow::Result;
use ast::{
    AbstractSyntaxTree,
    syntax_display::{DisplayKind, SyntaxDisplay},
};
use soul_name_resolver::name_resolve;
use ast_parser::parse;
use soul_tokenizer::{TokenStream, tokenize};
use soul_utils::{sementic_level::SementicFault};

use crate::{
    convert_soul_error::ToMessage, display_tokenizer::display_tokens,
    paths::Paths,
};

mod convert_soul_error;
mod display_tokenizer;
mod paths;

static PATHS: &[u8] = include_bytes!("../paths.json");

struct Ouput<'a> {
    source_file: &'a str,
    ast: AbstractSyntaxTree,
    faults: Vec<SementicFault>,
    token_stream: TokenStream<'a>,
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    let source_file = get_source_file(&paths.source_file)?;

    let token_stream = tokenize(&source_file);

    let mut parse_response = parse(token_stream.clone());
    name_resolve(&mut parse_response);

    let output = Ouput {
        token_stream,
        ast: parse_response.tree,
        source_file: &source_file,
        faults: parse_response.faults,
    };

    handle_output(&paths, output)
}

fn handle_output<'a>(paths: &Paths, output: Ouput<'a>) -> Result<()> {
    let root = &output.ast.root;
    let tokens_string = display_tokens(&paths, &output.source_file, output.token_stream)?;

    paths.write_multiple_outputs([
        (tokens_string, "tokens.soulc"),
        (root.display(&DisplayKind::Parser), "ast.soulc"),
        (
            root.display(&DisplayKind::NameResolver),
            "ast_NameResolved.soulc",
        ),
        // (display_hir(&output.hir), "hir.soulc"),
        // (
        //     root.display(&DisplayKind::TypeContext(
        //         typed_strings,
        //         output.typed_context.auto_copys,
        //     )),
        //     "ast_TypeContext.soulc",
        // ),
    ])?;

    for fault in &output.faults {
        eprintln!("{}", fault.to_message("main.soul", output.source_file));
    }

    if output.faults.is_empty() {
        use soul_utils::char_colors::{DEFAULT, GREEN};
        println!("{GREEN}success!!{DEFAULT}")
    }

    Ok(())
}

fn get_source_file(source_path: &String) -> Result<String> {
    let mut file = File::open(source_path)?;
    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;

    Ok(source_file)
}
