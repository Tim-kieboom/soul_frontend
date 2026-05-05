use std::{fs::File, io::Write, path::PathBuf, time::Instant};

use chrono::Local;

use run_ast::to_ast;
use run_hir::to_hir;
use run_mir::to_mir;
use soul_tokenizer::to_token_stream;
use soul_utils::{
    CrateExports, CrateStore, IdAlloc, ModuleId, SoulToml,
    compile_options::{Arch, CompilerOptions, Os, TargetInfo},
    crate_store::CrateContext,
    sementic_level::ModuleStore,
};

#[cfg(not(debug_assertions))]
use inkwell::context::Context;
#[cfg(not(debug_assertions))]
use soul_ir::{IrRequest, to_llvm_ir};

const OS: Os = Os::Windows;
const ARCH: Arch = Arch::X86_64;
const TARGET: TargetInfo = TargetInfo::new(ARCH, OS);
const COMPILER_OPTIONS: CompilerOptions = CompilerOptions::new_default(TARGET);

fn main() {
    let now = chrono::Local::now();
    let mut logger = Logger {
        text: String::new(),
    };
    let paths_json: PathsJson = serde_json::from_slice(include_bytes!("../paths.json")).unwrap();
    let project_path = PathBuf::from(&paths_json.project);

    logger.logln(format!("date: {}", now.format("%Y_%m_%d %H:%M:%S")));
    logger.logln("=== Benchmark ===");
    logger.logln(format!("Project: {:?}", project_path));

    let entry_path = get_entry_path(&project_path);
    let source = load_source(&entry_path).unwrap();
    logger.logln(format!("Source size: {} bytes", source.len()));

    let timer = Instant::now();
    let manifest = SoulToml::from_path(&project_path.join("Soul.toml")).unwrap();
    let mut crate_store = CrateStore::new();
    crate_store.insert(
        manifest.package.name.clone(),
        project_path.clone(),
        ModuleId::error(),
    );
    let load_time = timer.elapsed();
    logger.logln(format!("Load deps:   {:?}", load_time));

    let message_config = soul_utils::sementic_level::MessageConfig::default();
    let mut module_store = ModuleStore::new(entry_path.clone());
    let mut context = CrateContext::new(true, message_config);
    let root = module_store.get_root_id();

    let timer = Instant::now();
    let tokens = to_token_stream(&source, root);
    let tokenizer_time = timer.elapsed();
    logger.logln(format!("Tokenizer: {:?}", tokenizer_time));

    let timer = Instant::now();
    let ast = to_ast(
        tokens,
        &COMPILER_OPTIONS,
        &mut module_store,
        &mut context,
        &crate_store,
        project_path.clone(),
    );
    let ast_time = timer.elapsed();
    logger.logln(format!("AST:       {:?}", ast_time));

    let all_exports = CrateExports::default();
    let timer = Instant::now();
    let hir = to_hir(&ast, &COMPILER_OPTIONS, &mut context, &all_exports, root);
    let hir_time = timer.elapsed();
    logger.logln(format!("HIR:       {:?}", hir_time));

    let timer = Instant::now();
    let mir = to_mir(
        &hir,
        &ast,
        &COMPILER_OPTIONS,
        &mut context,
        &all_exports,
        root,
    );
    let mir_time = timer.elapsed();
    logger.logln(format!("MIR:       {:?}", mir_time));

    #[cfg(debug_assertions)]
    let _ = mir; // to avoid unused warning (mir is used in release)

    #[cfg(not(debug_assertions))]
    let llvm_time = {
        let timer = Instant::now();
        let request = IrRequest {
            mir: &mir,
            types: &hir.typed,
            context: &Context::create(),
            crate_name: "benchmark".to_string(),
        };
        let mut faults = Vec::new();
        to_llvm_ir(&request, &COMPILER_OPTIONS, &mut faults);
        timer.elapsed()
    };

    #[cfg(debug_assertions)]
    let llvm_time = std::time::Duration::ZERO;

    logger.logln(format!("LLVM IR:   {:?}", llvm_time));

    let frontend_total = tokenizer_time + ast_time + hir_time + mir_time;
    let full_total = frontend_total + llvm_time;

    let pct = |part: std::time::Duration, total: std::time::Duration| -> f64 {
        if total.as_nanos() == 0 {
            0.0
        } else {
            100.0 * part.as_secs_f64() / total.as_secs_f64()
        }
    };

    logger.logln("");
    logger.logln("=== Frontend Percentages ===");
    logger.logln(format!(
        "Tokenizer: {:.1}%",
        pct(tokenizer_time, frontend_total)
    ));
    logger.logln(format!("AST:       {:.1}%", pct(ast_time, frontend_total)));
    logger.logln(format!("HIR:       {:.1}%", pct(hir_time, frontend_total)));
    logger.logln(format!("MIR:       {:.1}%", pct(mir_time, frontend_total)));
    logger.logln("");
    logger.logln(format!("Frontend Total: {:?}", frontend_total));
    logger.logln("");
    logger.logln("=== Full Pipeline Percentages ===");
    logger.logln(format!(
        "Tokenizer: {:.1}%",
        pct(tokenizer_time, full_total)
    ));
    logger.logln(format!("AST:       {:.1}%", pct(ast_time, full_total)));
    logger.logln(format!("HIR:       {:.1}%", pct(hir_time, full_total)));
    logger.logln(format!("MIR:       {:.1}%", pct(mir_time, full_total)));
    logger.logln(format!("LLVM IR:   {:.1}%", pct(llvm_time, full_total)));
    logger.logln("");
    logger.logln(format!("Full Total: {:?}", full_total));

    println!("{}", logger.as_str());
    write_results(now, logger.as_str());
}
struct Logger {
    text: String,
}
impl Logger {
    fn logln(&mut self, text: impl Into<String>) {
        self.text.push_str(&text.into());
        self.text.push('\n');
    }

    fn as_str(&self) -> &str {
        &self.text
    }
}

fn get_entry_path(project_path: &PathBuf) -> PathBuf {
    let src = project_path.join("src");
    let main = src.join("main.soul");
    if main.exists() {
        main
    } else {
        src.join("lib.soul")
    }
}

fn load_source(path: &PathBuf) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

#[derive(serde::Deserialize)]
struct PathsJson {
    project: String,
}

fn write_results(now: chrono::DateTime<Local>, output: &str) {
    #[cfg(not(debug_assertions))]
    let folder = "release";

    #[cfg(debug_assertions)]
    let folder = "debug";

    let filename = format!(
        "example/results/{}/{}.txt",
        folder,
        now.format("%Y%m%d_%H%M%S")
    );

    let mut file = File::create(&filename).expect("Failed to create results file");
    file.write_all(output.as_bytes())
        .expect("Failed to write results");

    println!("Results written to: {}", filename);
}
