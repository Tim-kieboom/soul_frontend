use std::{
    fs::{File, OpenOptions},
    io::{Read, stdout},
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::Result;
use ast::{AbtractSyntaxTree};
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
    compile_options::{Arch, CompilerOptions, Os, TargetInfo},
    ids::IdAlloc,
    sementic_level::{CompilerContext, MessageConfig, SementicFault, SementicLevel},
    span::ModuleId,
};

use crate::{
    convert_soul_error::{ToMessage},
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
    backtrace: false,
    colors: true,
};

const OS: Os = Os::Windows;
const ARCH: Arch = Arch::X86_64;
const TARGET: TargetInfo = TargetInfo::new(ARCH, OS);
pub const COMPILER_OPTIONS: CompilerOptions = CompilerOptions::new_default(TARGET);

struct Ouput {
    mir_response: MirResponse,
    hir_response: HirResponse,
}

fn main() -> Result<()> {
    let paths: Paths = serde_json::from_slice(PATHS)?;
    init_logger(&paths.log_file)?;
    
    let source_folder = paths.to_source_path();
    let main_file_path = paths.to_entry_file_path();
    let mut context = CompilerContext::new(source_folder, main_file_path, MESSAGE_CONFIG);

    let mut timer = Instant::now();
    let root_module = context.root_module_id();
    let main_file_path = paths.to_entry_file_path();
    let mut output = run_crate_fontend(&paths, &main_file_path, &mut context, root_module, "crate")?;
    log_faults(&context);
    if is_fatal(&context.faults, COMPILER_OPTIONS.fatal_level()) {
        return Ok(());
    }

    info!(
        "{GREEN}frontend success: {}ms{DEFAULT}",
        timer.elapsed().as_millis()
    );

    timer = Instant::now();
    if run_llvm(&mut output, &mut context) {
        info!(
            "{GREEN}llvm success: {}ms{DEFAULT}",
            timer.elapsed().as_millis()
        )
    }
    Ok(())
}

fn run_crate_fontend(paths: &Paths, entry_path: &PathBuf, context: &mut CompilerContext, root_module: ModuleId, lib_name: &str) -> Result<Ouput> {
    
    let source_file = to_source_file(entry_path)?;
    let tokens = to_token_stream(&source_file, root_module);
    display_tokenizer(paths, root_module, &source_file, lib_name)?;
    
    let ast = to_ast(tokens, &COMPILER_OPTIONS, context);
    display_ast(paths, &context, &ast, lib_name)?;

    let mut hir = to_hir(&ast, &COMPILER_OPTIONS, context, root_module);
    display_hir(paths, &hir, &ast, lib_name)?;
    clear_hir_type_map(&mut hir);

    let mir = to_mir(&hir, &ast, &COMPILER_OPTIONS, context, root_module);
    display_mir(paths, &mir, &hir, &ast, lib_name)?;

    Ok(Ouput {
        mir_response: mir,
        hir_response: hir,
    })
}

fn run_llvm(output: &mut Ouput, context: &mut CompilerContext) -> bool {
    let request = IrRequest {
        mir: &output.mir_response,
        types: &output.hir_response.typed,
        context: &Context::create(),
    };

    context.faults.clear();
    let ir = to_llvm_ir(&request, &COMPILER_OPTIONS, &mut context.faults);
    log_faults(&context);

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

fn display_tokenizer(paths: &Paths, module: ModuleId, source_file: &str, lib_name: &str) -> Result<()> {
    let token_stream = to_token_stream(source_file, module);
    let tokens = display_tokens(paths, source_file, token_stream)?;
    paths.write_to_output(&tokens, &format!("{lib_name}/tokenizer/tokens.soulc"))
}

fn display_ast(paths: &Paths, context: &CompilerContext, ast_context: &AbtractSyntaxTree, lib_name: &str) -> Result<()> {
    let root = context.module_store.get_root_id();
    paths.write_to_output(
        &displayer_ast::display_ast(root, context, ast_context),
        &format!("{lib_name}/ast/tree.soulc"),
    )?;
    paths.write_to_output(
        &displayer_ast::display_ast_name_resolved(root, context, ast_context),
        &format!("{lib_name}/ast/NameResolved.soulc"),
    )
}

fn display_hir(paths: &Paths, hir: &HirResponse, ast_context: &AbtractSyntaxTree, lib_name: &str) -> Result<()> {
    paths.write_to_output(
        &displayer_hir::display_hir(ast_context, &hir.hir),
        &format!("{lib_name}/hir/tree.soulc"),
    )?;
    paths.write_to_output(
        &displayer_hir::display_thir(ast_context, &hir.hir, &hir.typed),
        &format!("{lib_name}/thir/tree.soulc"),
    )?;
    paths.write_to_output(
        &displayer_hir::display_created_types(&hir.hir, &hir.typed),
        &format!("{lib_name}/thir/types.soulc"),
    )?;
    Ok(())
}

/// make sure that hir types are not used but thir types are
fn clear_hir_type_map(hir: &mut HirResponse) {
    hir.hir.info.types.clear();
    hir.hir.info.infers.clear();
}

fn display_mir(paths: &Paths, mir: &MirResponse, hir: &HirResponse, ast_context: &AbtractSyntaxTree, lib_name: &str) -> Result<()> {
    paths.write_to_output(
        &displayer_mir::display_mir(&mir.tree, hir, &ast_context.modules),
        &format!("{lib_name}/mir/tree.soulc"),
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
