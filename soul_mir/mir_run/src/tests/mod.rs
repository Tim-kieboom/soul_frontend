use std::path::PathBuf;

use ast_model::AstTree;
use ast_run::{AstRequest, to_ast};
use soul_tokenizer::to_token_stream;
use soul_utils::{
    CrateContext,
    collections::{benchmark::Benchmark, crate_store::CrateStore, module_store::ModuleStore},
    compiler_options::CompilerOptions,
    fault::Severity,
};

use crate::MirProgram;

fn create_mir(ast: &AstTree) -> (MirProgram, CrateContext) {
    let mut benchmark = Benchmark::new();
    create_mir_with_benchmark(ast, &mut benchmark)
}

fn create_mir_with_benchmark(
    ast: &AstTree,
    benchmark: &mut Benchmark,
) -> (MirProgram, CrateContext) {
    let mut context = CrateContext::default();
    let mir = crate::to_mir(ast, benchmark, &mut context, &CompilerOptions::default());
    (mir, context)
}

fn build_ast(source: &str) -> AstTree {
    let mut module_store = ModuleStore::new();
    module_store.insert_root(PathBuf::from("test.soul"));
    let root = module_store.get_root_id();
    let crate_store = CrateStore::new();

    let tokens = to_token_stream(source, root).expect("test source failed to tokenize");
    let mut benchmark = Benchmark::new();
    let options = CompilerOptions {
        fail_level: Severity::Error,
    };

    let ast = to_ast(
        tokens,
        AstRequest {
            source_folder: PathBuf::from("."),
            benchmark: &mut benchmark,
            module_store: &mut module_store,
            crate_store: &crate_store,
        },
        &options,
    );
    assert_eq!(
        ast.faults().iter().count(),
        0,
        "test source failed to resolve: {:#?}",
        ast.faults()
    );
    ast
}

#[test]
fn lowers_every_lowerable_function_with_no_errors() {
    let ast = build_ast("add(a: int, b: int): int {\n    c := a + b\n    return c\n}\n");
    let mut benchmark = Benchmark::new();

    let (program, context) = create_mir_with_benchmark(&ast, &mut benchmark);

    assert_eq!(program.functions.entries().count(), 1);
    assert_eq!(
        context.faults.iter().count(),
        0,
        "expected no lowering faults: {:#?}",
        context.faults.iter().collect::<Vec<_>>()
    );
    assert!(
        benchmark.iter().any(|(name, _)| name == "mir"),
        "expected a `mir` benchmark entry to be recorded"
    );
}

#[test]
fn out_of_scope_function_pushes_a_fault_into_the_context_not_a_panic() {
    let ast = build_ast("struct Point { x: int }\nf(p: Point): int {\n    return p.x\n}\n");

    let (program, context) = create_mir(&ast);

    assert_eq!(program.functions.entries().count(), 0);
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        1,
        "{:#?}",
        context.faults.iter().collect::<Vec<_>>()
    );
}

#[test]
fn extern_function_is_skipped_without_pushing_a_fault() {
    let ast = build_ast(r#"extern "C" printf(fmt: &char): int {}"#);

    let (program, context) = create_mir(&ast);

    assert_eq!(program.functions.entries().count(), 0);
    assert_eq!(
        context.faults.iter().count(),
        0,
        "extern declarations should be skipped, not pushed as a lowering fault: {:#?}",
        context.faults.iter().collect::<Vec<_>>()
    );
}

#[test]
fn mixed_program_lowers_what_it_can_and_faults_on_the_rest() {
    let ast = build_ast(
        "struct Point { x: int }\nokFn(a: int, b: int): int {\n    return a + b\n}\nbadFn(p: Point): int {\n    return p.x\n}\n",
    );

    let (program, context) = create_mir(&ast);

    assert_eq!(program.functions.entries().count(), 1);
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        1,
        "{:#?}",
        context.faults.iter().collect::<Vec<_>>()
    );
}
