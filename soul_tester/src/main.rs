use std::{
    fs::{File, OpenOptions},
    io::{Read, stdout},
    path::{Path, PathBuf},
    str::FromStr,
    time::Instant,
};

use anyhow::Result;
use ast::AstContext;
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
    char_colors::{DEFAULT, GREEN}, compile_options::{Arch, CompilerOptions, Os, TargetInfo}, ids::IdAlloc, sementic_level::{CompilerContext, SementicFault, SementicLevel}, span::ModuleId
};

use crate::{
    convert_soul_error::{MessageConfig, ToMessage},
    displayer_tokenizer::display_tokens,
};

mod convert_soul_error;
mod displayer_ast;
mod displayer_hir;
mod displayer_mir;
mod displayer_tokenizer;
mod paths;

static PATHS: &[u8] = include_bytes!("../paths.json");

pub const MESSAGE_CONFIG: MessageConfig = MessageConfig {
    backtrace: true,
    colors: true,
};

const OS: Os = Os::Windows;
const ARCH: Arch = Arch::X86_64;
pub const COMPILER_OPTIONS: CompilerOptions =
    CompilerOptions::new_default(TargetInfo::new(ARCH, OS));

struct Ouput {
    mir_response: MirResponse,
    hir_response: HirResponse,
    context: CompilerContext,
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    init_logger(&paths.log_file)?;

    let mut timer = Instant::now();
    let mut output = run_fontend(&paths)?;
    log_faults(&output.context);
    if is_fatal(&output.context.faults, COMPILER_OPTIONS.fatal_level()) {
        return Ok(());
    }

    info!(
        "{GREEN}frontend success: {}ms{DEFAULT}",
        timer.elapsed().as_millis()
    );

    timer = Instant::now();
    if run_llvm(&mut output) {
        info!(
            "{GREEN}llvm success: {}ms{DEFAULT}",
            timer.elapsed().as_millis()
        )
    }
    Ok(())
}

fn run_fontend(paths: &Paths) -> Result<Ouput> {
    let root = paths.to_entry_file_path();
    let source_folder = PathBuf::from_str(&paths.source_folder).expect("error is infallible");
    let mut context = CompilerContext::new(source_folder, root.clone());
    let root_module = context.module_store.get_root_id();

    let source_file = to_source_file(&root)?;
    let tokens = to_token_stream(&source_file, root_module);
    display_tokenizer(paths, root_module, &source_file)?;

    let mut ast_context = AstContext::new();
    to_ast(tokens, &COMPILER_OPTIONS, &mut context, &mut ast_context);
    display_ast(paths, &context, &ast_context)?;

    let mut hir = to_hir(&COMPILER_OPTIONS, &mut context, &ast_context);
    clear_hir_type_map(&mut hir);
    display_hir(paths, &hir)?;

    let mir = to_mir(&hir, &COMPILER_OPTIONS, &mut context);
    display_mir(paths, &mir, &hir)?;

    Ok(Ouput {
        mir_response: mir,
        hir_response: hir,
        context,
    })
}

fn run_llvm(output: &mut Ouput) -> bool {
    let request = IrRequest {
        mir: &output.mir_response,
        types: &output.hir_response.typed,
        context: &Context::create(),
    };

    output.context.faults.clear();
    let ir = to_llvm_ir(&request, &COMPILER_OPTIONS, &mut output.context.faults);
    log_faults(&output.context);

    #[cfg(not(debug_assertions))]
    if ir.is_fatal {
        return false;
    }

    #[cfg(debug_assertions)]
    if ir.is_fatal {
        if let Err(err) = ir.module.print_to_file("output/fatal_out.ll") {
            error!("{err}");
            return false;
        }
        return false;
    }

    if let Err(err) = ir.module.print_to_file("output/out.ll") {
        error!("{err}");
        return false;
    };

    true
}

fn display_tokenizer(paths: &Paths, module: ModuleId, source_file: &str) -> Result<()> {
    let token_stream = to_token_stream(source_file, module);
    let tokens = display_tokens(paths, source_file, token_stream)?;
    paths.write_to_output(&tokens, "tokenizer/tokens.soulc")
}

fn display_ast(paths: &Paths, context: &CompilerContext, ast_context: &AstContext) -> Result<()> {
    let root = context.module_store.get_root_id();
    paths.write_to_output(&displayer_ast::display_ast(root, context, ast_context), "ast/tree.soulc")?;
    paths.write_to_output(
        &displayer_ast::display_ast_name_resolved(root, context, ast_context),
        "ast/NameResolved.soulc",
    )
}

fn display_hir(paths: &Paths, hir: &HirResponse) -> Result<()> {
    paths.write_to_output(&displayer_hir::display_hir(&hir.hir), "hir/tree.soulc")?;
    paths.write_to_output(
        &displayer_hir::display_thir(&hir.hir, &hir.typed),
        "thir/tree.soulc",
    )?;
    paths.write_to_output(
        &displayer_hir::display_created_types(&hir.hir, &hir.typed),
        "thir/types.soulc",
    )?;
    Ok(())
}

/// make sure that hir types are not used but thir types are
fn clear_hir_type_map(hir: &mut HirResponse) {
    hir.hir.info.types.clear();
    hir.hir.info.infers.clear();
}

fn display_mir(paths: &Paths, mir: &MirResponse, hir: &HirResponse) -> Result<()> {
    paths.write_to_output(
        &displayer_mir::display_mir(&mir.tree, hir),
        "mir/tree.soulc",
    )
}

fn to_source_file(source_path: &Path) -> Result<String> {
    let mut file = File::open(source_path)?;
    let mut source_file = String::new();
    file.read_to_string(&mut source_file)?;

    Ok(source_file)
}

fn log_faults(constext: &CompilerContext) {
    let modules = &constext.module_store;
    let mut source_file = String::new();
    let mut module = ModuleId::error();

    for fault in &constext.faults {
        let module_id = match fault.get_soul_error().span {
            Some(val) => val.module,
            None => modules.get_root_id(),
        };

        let path = match modules.get_path(module_id) {
            Some(val) => val,
            None => &PathBuf::new(),
        };

        if module_id != module {
            source_file = to_source_file(path).unwrap_or(String::new());
            module = module_id;
        }

        error!("{}", fault.to_message(path, &source_file, MESSAGE_CONFIG));
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
            return true;
        }
    }

    false
}
