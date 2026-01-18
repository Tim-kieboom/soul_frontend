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
use soul_utils::{sementic_level::SementicFault, vec_map::VecMap};

use crate::{
    convert_soul_error::{ToAnyhow, ToMessage},
    display_hir::display_hir,
    paths::Paths,
};

mod convert_soul_error;
mod display_hir;
mod paths;

static PATHS: &[u8] = include_bytes!("../paths.json");

struct Ouput<'a> {
    hir: HirTree,
    source_file: &'a str,
    ast: AbstractSyntaxTree,
    faults: Vec<SementicFault>,
    typed_context: VecMap<NodeId, HirType>,
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    let source_file = get_source_file(&paths.source_file)?;

    let token_stream = tokenize(&source_file);
    let token_string = pretty_format_tokenizer(&paths, &source_file, token_stream.clone())?;

    paths.write_to_output(token_string, "tokenize.soulc")?;

    let mut parse_response = parse(token_stream);
    name_resolve(&mut parse_response);

    let HirResponse { hir, faults } = lower_to_hir(&parse_response);
    parse_response.extend_faults(faults);

    let TypedHirResponse {
        typed_context,
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
        source_file: &source_file,
        typed_context,
    };

    handle_output(&paths, output)
}

fn handle_output<'a>(paths: &Paths, output: Ouput<'a>) -> Result<()> {
    let Ouput {
        hir,
        source_file,
        ast,
        faults,
        typed_context,
    } = output;

    let typed_strings = typed_context
        .entries()
        .map(|(id, ty)| (id, ty.display()))
        .collect::<VecMap<_, _>>();

    paths.write_multiple_outputs([
        (ast.root.display(&DisplayKind::Parser), "ast.soulc"),
        (ast.root.display(&DisplayKind::NameResolver), "ast_NameResolved.soulc"),
        (display_hir(&hir), "hir.soulc"),
        (ast.root.display(&DisplayKind::TypeContext(typed_strings)), "ast_TypeContext.soulc"),
    ])?;

    for fault in &faults {
        eprintln!("{}", fault.to_message("main.soul", source_file));
    }

    if faults.is_empty() {
        use soul_utils::char_colors::{DEFAULT, GREEN};
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
        let token = result
            .map_err(|err| SementicFault::error(err).to_anyhow(&paths.source_file, source_file))?;

        sb.push_str(&format!("\tToken({})", token.kind.display()));
        sb.push_str(&format!(" >> Span({})", token.span.display()));
        sb.push_str(",\n");
    }

    sb.push(']');
    Ok(sb)
}

fn get_source_file(source_path: &String) -> Result<String> {
    let mut file = File::open(source_path)?;
    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;

    Ok(source_file)
}
