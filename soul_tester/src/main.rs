use std::{fs::File, io::Read};

use anyhow::Result;
use ast::{
    AbstractSyntaxTree,
    syntax_display::{DisplayKind, SyntaxDisplay},
};
use ast_parser::parse;
use display_hir::display_hir;
use hir::HirTree;
use hir_parser::hir_lower;
use hir_typed_context::{HirTypedTable, infer_types};
use paths::Paths;
use soul_name_resolver::name_resolve;
use soul_tokenizer::{TokenStream, tokenize};
use soul_utils::sementic_level::SementicFault;

use crate::{convert_soul_error::ToMessage, display_hir::display_typed_hir, display_tokenizer::display_tokens};

mod convert_soul_error;
mod display_hir;
mod display_tokenizer;
mod paths;

static PATHS: &[u8] = include_bytes!("../paths.json");

pub const ERROR_BACKTRACE: bool = false;

struct Ouput<'a> {
    hir: HirTree,
    source_file: &'a str,
    ast: AbstractSyntaxTree,
    hir_types: HirTypedTable,
    faults: Vec<SementicFault>,
    token_stream: TokenStream<'a>,
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;

    let source_file = read_source_file(&paths.source_file)?;
    let token_stream = tokenize(&source_file);

    let mut faults = vec![];
    let mut parse_response = parse(token_stream.clone(), &mut faults);
    name_resolve(&mut parse_response, &mut faults);

    let hir = hir_lower(&parse_response, &mut faults);
    let hir_types = infer_types(&hir, &mut faults);

    let output = Ouput {
        hir,
        faults,
        hir_types,
        token_stream,
        ast: parse_response.tree,
        source_file: &source_file,
    };

    handle_output(&paths, output)
}

fn handle_output<'a>(paths: &Paths, output: Ouput<'a>) -> Result<()> {
    let root = &output.ast.root;
    let tokens_string = display_tokens(&paths, &output.source_file, output.token_stream)?;

    paths.write_multiple_outputs([
        (tokens_string, "tokenizer/tokens.soulc"),
        (root.display(&DisplayKind::Parser), "ast/tree.soulc"),
        (
            root.display(&DisplayKind::NameResolver),
            "ast/NameResolved.soulc",
        ),
        (display_hir(&output.hir), "hir/tree.soulc"),
        (display_typed_hir(&output.hir, &output.hir_types), "hir/typed.soulc"),
    ])?;

    for fault in &output.faults {
        eprintln!("{}", fault.to_message("main.soul", output.source_file, ERROR_BACKTRACE));
    }

    if output.faults.is_empty() {
        use soul_utils::char_colors::{DEFAULT, GREEN};
        println!("{GREEN}success!!{DEFAULT}")
    }

    Ok(())
}

fn read_source_file(source_path: &str) -> Result<String> {
    let mut file = File::open(source_path)?;
    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;

    Ok(source_file)
}
