use std::{
    fs::{File, OpenOptions}, io::{Read, stdout}
};

use anyhow::Result;
use ast::{
    AstResponse, syntax_display::{DisplayKind, SyntaxDisplay}
};
use displayer_hir::display_hir;
use fern::Dispatch;
use log::{error, info};
use mir_parser::{mir::MirTree};
use paths::Paths;
use run_ast::to_ast;
use run_hir::{HirResponse, to_hir};
use run_mir::to_mir;
use soul_tokenizer::to_token_stream;
use soul_utils::{char_colors::{DEFAULT, GREEN}, compile_options::CompilerOptions, sementic_level::SementicFault};

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

pub const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    debug_view_literal_resolve: true,
};

struct Ouput {
    mir: MirTree,
    hir: HirResponse,
    ast: AstResponse,
    source_file: String,
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
    
    let mut faults = vec![];

    let source_file = to_source_file(&paths.source_file)?;
    let tokens = to_token_stream(&source_file);
    let ast = to_ast(tokens, &COMPILER_OPTIONS, &mut faults);
    let hir = to_hir(&ast, &COMPILER_OPTIONS, &mut faults);
    let mir = to_mir(&hir, &COMPILER_OPTIONS, &mut faults);

    Ok(Ouput {
        mir,
        ast,
        hir,
        faults,
        source_file,
    })
}

fn display_output<'a>(paths: &Paths, output: &Ouput) -> Result<()> {
    let root = &output.ast.tree.root;
    let token_stream = to_token_stream(&output.source_file);
    let tokens_string = display_tokens(&paths, &output.source_file, token_stream)?;

    paths.write_multiple_outputs([
        (&tokens_string, "tokenizer/tokens.soulc"),
        (&root.display(&DisplayKind::Parser), "ast/tree.soulc"),
        (
            &root.display(&DisplayKind::NameResolver),
            "ast/NameResolved.soulc",
        ),
        (&display_hir(&output.hir.tree), "hir/tree.soulc"),
        (
            &display_typed_hir(&output.hir.tree, &output.hir.types),
            "hir/typed.soulc",
        ),
        (
            &display_mir(&output.mir, &output.hir.tree, &output.hir.types),
            "mir/tree.soulc",
        ),
    ])
}

fn to_source_file(source_path: &str) -> Result<String> {
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
