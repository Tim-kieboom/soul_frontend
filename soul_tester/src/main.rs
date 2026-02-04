use std::{fs::File, io::Read};

use anyhow::Result;
use hir_model::{HirResponse, HirTree, HirType};
use parser_models::{
    AbstractSyntaxTree, ParseResponse,
    scope::NodeId,
    syntax_display::{DisplayKind, SyntaxDisplay},
};
use soul_hir::lower_to_hir;
use soul_name_resolver::name_resolve;
use soul_parser::parse;
use soul_tokenizer::{TokenStream, tokenize};
use soul_typed_context::{TypedHirResponse, get_typed_context};
use soul_utils::{
    sementic_level::SementicFault,
    vec_map::{VecMap, VecSet},
};

use crate::{
    convert_soul_error::ToMessage, display_hir::display_hir, display_tokenizer::display_tokens,
    paths::Paths,
};

mod convert_soul_error;
mod display_hir;
mod display_tokenizer;
mod paths;

static PATHS: &[u8] = include_bytes!("../paths.json");

struct Ouput<'a> {
    hir: HirTree,
    source_file: &'a str,
    ast: AbstractSyntaxTree,
    faults: Vec<SementicFault>,
    token_stream: TokenStream<'a>,
    auto_copys: VecSet<NodeId>,
    typed_context: VecMap<NodeId, HirType>,
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    let source_file = get_source_file(&paths.source_file)?;

    let token_stream = tokenize(&source_file);

    let mut parse_response = parse(token_stream.clone());
    name_resolve(&mut parse_response);

    let HirResponse { hir, faults } = lower_to_hir(&parse_response);
    parse_response.extend_faults(faults);

    let TypedHirResponse {
        typed_context,
        auto_copys,
        faults,
    } = get_typed_context(&hir);
    parse_response.extend_faults(faults);

    let ParseResponse {
        faults,
        tree: ast,
        meta_data: _,
    } = parse_response;

    let output = Ouput {
        hir,
        ast,
        faults,
        token_stream,
        auto_copys,
        source_file: &source_file,
        typed_context,
    };

    handle_output(&paths, output)
}

fn handle_output<'a>(paths: &Paths, output: Ouput<'a>) -> Result<()> {
    let root = &output.ast.root;
    let tokens_string = display_tokens(&paths, &output.source_file, output.token_stream)?;
    let typed_strings = output
        .typed_context
        .entries()
        .map(|(id, ty)| (id, ty.display()))
        .collect::<VecMap<_, _>>();

    paths.write_multiple_outputs([
        (tokens_string, "tokens.soulc"),
        (root.display(&DisplayKind::Parser), "ast.soulc"),
        (
            root.display(&DisplayKind::NameResolver),
            "ast_NameResolved.soulc",
        ),
        (display_hir(&output.hir), "hir.soulc"),
        (
            root.display(&DisplayKind::TypeContext(typed_strings, output.auto_copys)),
            "ast_TypeContext.soulc",
        ),
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
