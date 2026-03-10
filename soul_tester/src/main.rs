use std::{
    fs::{File, OpenOptions}, io::{Read, stdout}
};

use anyhow::Result;
use ast::{
    AbstractSyntaxTree,
    syntax_display::{DisplayKind, SyntaxDisplay},
};
use ast_parser::parse;
use displayer_hir::display_hir;
use fern::Dispatch;
use hir::HirTree;
use hir_literal_interpreter::try_literal_resolve_expression;
use hir_parser::hir_lower;
use hir_typed_context::{HirTypedTable, infer_hir_types};
use log::{error, info};
use mir_parser::{mir::MirTree, mir_lower};
use paths::Paths;
use soul_name_resolver::name_resolve;
use soul_tokenizer::tokenize;
use soul_utils::{char_colors::{DEFAULT, GREEN}, error::SoulError, sementic_level::SementicFault};

use crate::{
    convert_soul_error::{MessageConfig, ToMessage},
    displayer_hir::{display_typed_hir},
    displayer_mir::display_mir,
    displayer_tokenizer::display_tokens,
};

mod convert_soul_error;
mod displayer_hir;
mod displayer_mir;
mod displayer_tokenizer;
mod paths;

static PATHS: &[u8] = include_bytes!("../paths.json");

pub const MESSAGE_CONFIG: MessageConfig = MessageConfig {
    backtrace: false,
    colors: true,
};

struct Ouput {
    mir: MirTree,
    hir: HirTree,
    source_file: String,
    types: HirTypedTable,
    ast: AbstractSyntaxTree,
    faults: Vec<SementicFault>,
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    init_logger(&paths.log_file)?;

    let output = run_compiler(&paths)?; 
    display_output(&paths, &output)?;
    for fault in &output.faults {
        error!(
            "{}",
            fault.to_message("main.soul", &output.source_file, MESSAGE_CONFIG)
        );
    }

    if output.faults.is_empty() {
        let msg = "success!!";
        info!("{GREEN}{msg}{DEFAULT}");
    }

    Ok(())
}

fn run_compiler<'a>(paths: &'a Paths) -> Result<Ouput> {
    let source_file = read_source_file(&paths.source_file)?;

    let mut faults = vec![];
    let mut parse_response = parse(tokenize(&source_file), &mut faults);
    name_resolve(&mut parse_response, &mut faults);

    let hir = hir_lower(&parse_response, &mut faults);
    let types = infer_hir_types(&hir, &mut faults);

    for value_id in hir.expressions.keys() {
        let span = hir.spans.expressions[value_id];
        let literal = try_literal_resolve_expression(&hir, &types, value_id);
        if let Some(literal) = literal {
            let msg = SoulError::new(format!("literal resolved to >> {}", literal.value_to_string()), soul_utils::error::SoulErrorKind::InvalidContext, Some(span));
            error!(
                "{}",
                SementicFault::debug(msg).to_message("main.soul", &source_file, MESSAGE_CONFIG)
            );
        }
    }

    let mir = mir_lower(&hir, &types, &mut faults);

    Ok(Ouput {
        mir,
        hir,
        types,
        faults,
        source_file,
        ast: parse_response.tree,
    })
}

fn display_output<'a>(paths: &Paths, output: &Ouput) -> Result<()> {
    let root = &output.ast.root;
    let token_stream = tokenize(&output.source_file);
    let tokens_string = display_tokens(&paths, &output.source_file, token_stream)?;

    paths.write_multiple_outputs([
        (&tokens_string, "tokenizer/tokens.soulc"),
        (&root.display(&DisplayKind::Parser), "ast/tree.soulc"),
        (
            &root.display(&DisplayKind::NameResolver),
            "ast/NameResolved.soulc",
        ),
        (&display_hir(&output.hir), "hir/tree.soulc"),
        (
            &display_typed_hir(&output.hir, &output.types),
            "hir/typed.soulc",
        ),
        (
            &display_mir(&output.mir, &output.hir, &output.types),
            "mir/tree.soulc",
        ),
    ])
}

fn read_source_file(source_path: &str) -> Result<String> {
    let mut file = File::open(source_path)?;
    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;

    Ok(source_file)
}

fn init_logger(log_file: &str) -> Result<()> {
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    Dispatch::new()
        .format(|out, message, _record| out.finish(format_args!("{}", message)))
        .level_for("soulc", log::LevelFilter::Info)
        .chain(stdout())
        .chain(log_file)
        .apply()?;

    Ok(())
}
