use std::{
    fs::{File, OpenOptions},
    io::{Read, stdout}, time::Instant,
};

use anyhow::Result;
use ast::{
    AstResponse,
    syntax_display::{DisplayKind, SyntaxDisplay},
};
use fern::Dispatch;
use inkwell::context::Context;
use log::{error, info};
use paths::Paths;
use run_ast::to_ast;
use run_hir::{HirResponse, to_hir};
use run_mir::{MirResponse, to_mir};
use soul_ir::{IrRequest, to_llvm_ir};
use soul_tokenizer::to_token_stream;
use soul_utils::{
    char_colors::{DEFAULT, GREEN},
    compile_options::CompilerOptions,
    sementic_level::{SementicFault, SementicLevel},
};

use crate::{
    convert_soul_error::{MessageConfig, ToMessage}, displayer_tokenizer::display_tokens
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
    debug_view_literal_resolve: false,
    fault_level: SementicLevel::Error,
};

struct Ouput {
    mir_response: MirResponse,
    hir_response: HirResponse,
    source_file: String,
    faults: Vec<SementicFault>,
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    init_logger(&paths.log_file)?;
    
    let mut timer = Instant::now();
    let output = run_fontend(&paths)?;
    log_faults(&output.faults, &output.source_file);
    if is_fatal(&output.faults, SementicLevel::Error) {
        return Ok(());
    }
    
    info!("{GREEN}frontend success: {}ms{DEFAULT}", timer.elapsed().as_millis());

    timer = Instant::now();
    if run_llvm(&output) {
        info!("{GREEN}llvm success: {}ms{DEFAULT}", timer.elapsed().as_millis())
    }
    Ok(())
}

fn run_fontend<'a>(paths: &'a Paths) -> Result<Ouput> {
    let mut faults = vec![];

    let source_file = to_source_file(&paths.source_file)?;
    let tokens = to_token_stream(&source_file);
    display_tokenizer(paths, &source_file)?;

    let ast = to_ast(tokens, &COMPILER_OPTIONS, &mut faults);
    display_ast(paths, &ast)?;
    log_faults(&faults, &source_file);

    let mut hir = to_hir(&ast, &COMPILER_OPTIONS, &mut faults);
    display_hir(paths, &hir)?;
    clear_hir_type_map(&mut hir);

    let mir = to_mir(&hir, &COMPILER_OPTIONS, &mut faults);
    display_mir(paths, &mir, &hir)?;

    Ok(Ouput {
        mir_response: mir,
        hir_response: hir,
        faults,
        source_file,
    })
}

fn run_llvm(output: &Ouput) -> bool {
    let request = IrRequest {
        mir: &output.mir_response,
        types: &output.hir_response.typed,
        context: &Context::create(),
    };

    let mut faults = vec![];
    let ir = to_llvm_ir(&request, &COMPILER_OPTIONS, &mut faults);
    log_faults(&faults, &output.source_file);

    #[cfg(not(debug_assertions))]
    if ir.is_fatal {
        return false
    }

    #[cfg(debug_assertions)]
    if ir.is_fatal {
        if let Err(err) = ir.module.print_to_file("output/fatal_out.ll") {
            error!("{err}");
            return false
        }
        return false
    }
    
    if let Err(err) = ir.module.print_to_file("output/out.ll") {
        error!("{err}");
        return false
    };

    true
}

fn display_tokenizer(paths: &Paths, source_file: &str) -> Result<()> {
    let token_stream = to_token_stream(source_file);
    let tokens = display_tokens(paths, source_file, token_stream)?;
    paths.write_to_output(&tokens, "tokenizer/tokens.soulc")
}

fn display_ast(paths: &Paths, ast: &AstResponse) -> Result<()> {
    paths.write_to_output(
        &ast.tree.root.display(DisplayKind::Parser),
        "ast/tree.soulc",
    )?;
    paths.write_to_output(
        &ast.tree.root.display(DisplayKind::NameResolver),
        "ast/NameResolved.soulc",
    )
}

fn display_hir(paths: &Paths, hir: &HirResponse) -> Result<()> {
    paths.write_to_output(&displayer_hir::display_hir(&hir.hir), "hir/tree.soulc")?;
    paths.write_to_output(&displayer_hir::display_thir(&hir.hir, &hir.typed), "thir/tree.soulc")?;
    paths.write_to_output(&displayer_hir::display_created_types(&hir.hir, &hir.typed), "thir/types.soulc")?;
    Ok(())
}

/// make sure that hir types are not used but thir types are
fn clear_hir_type_map(hir: &mut HirResponse) {
    hir.hir.info.types.clear();
    hir.hir.info.infers.clear();
}

fn display_mir(paths: &Paths, mir: &MirResponse, hir: &HirResponse) -> Result<()> {
    paths.write_to_output(
        &displayer_mir::display_mir(&mir.tree, &hir),
        "mir/tree.soulc",
    )
}

fn to_source_file(source_path: &str) -> Result<String> {
    let mut file = File::open(source_path)?;
    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;

    Ok(source_file)
}

fn log_faults(faults: &Vec<SementicFault>, source_file: &str) {
    for fault in faults {
        error!(
            "{}",
            fault.to_message("main.soul", source_file, MESSAGE_CONFIG)
        );
    }
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

fn is_fatal(faults: &Vec<SementicFault>, fatal_level: SementicLevel) -> bool {
    for fault in faults {
        if fault.is_fatal(fatal_level) {
            return true
        }
    } 

    false
}
