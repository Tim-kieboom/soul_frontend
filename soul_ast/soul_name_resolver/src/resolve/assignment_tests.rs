use std::path::PathBuf;

use ast_model::AstTree;
use ast_parser::{ParseInfo, parse_module};
use soul_tokenizer::to_token_stream;
use soul_utils::collections::{crate_store::CrateStore, module_store::ModuleStore};

use crate::name_resolve;

fn resolve_source(source: &str) -> AstTree {
    let mut module_store = ModuleStore::new();
    module_store.insert_root(PathBuf::from("test.soul"));
    let root = module_store.get_root_id();
    let crate_store = CrateStore::new();

    let tokens = to_token_stream(source, root).expect("test source failed to tokenize");

    let mut ast = AstTree::new(root);
    let info = ParseInfo {
        id: root,
        source_folder: PathBuf::from("."),
        crate_source_folder: PathBuf::from("."),
        parent: None,
        modules: &mut module_store,
        context: &mut ast.context,
        forest: &mut ast.crates,
        crate_store: &crate_store,
    };
    parse_module(tokens, "crate".to_string(), info);

    name_resolve(&mut module_store, &mut ast, &crate_store);
    ast
}

fn fault_count_containing(ast: &AstTree, needle: &str) -> usize {
    ast.faults()
        .iter()
        .filter(|fault| fault.message().contains(needle))
        .count()
}

#[test]
fn matching_assignment_reports_no_fault() {
    let ast = resolve_source("main() {\n    mut a: i64 = 1\n    a = 2\n}\n");
    assert_eq!(fault_count_containing(&ast, "assignment type mismatch"), 0);
}

#[test]
fn mismatched_assignment_reports_exactly_one_fault() {
    let ast = resolve_source("main() {\n    mut a: i64 = 1\n    a = \"hi\"\n}\n");
    assert_eq!(fault_count_containing(&ast, "assignment type mismatch"), 1);
}

#[test]
fn compound_assignment_reuses_binary_type_checking() {
    let ast = resolve_source("main() {\n    mut a: i64 = 1\n    b: str = \"hi\"\n    a += b\n}\n");
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 1);
}

#[test]
fn matching_compound_assignment_reports_no_fault() {
    let ast = resolve_source("main() {\n    mut a: i64 = 1\n    a += 2\n}\n");
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 0);
}

#[test]
fn assigning_to_immutable_variable_reports_exactly_one_fault() {
    let ast = resolve_source("main() {\n    a: i64 = 1\n    a = 2\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "cannot assign to an immutable variable"),
        1
    );
}

#[test]
fn assigning_to_mutable_variable_reports_no_mutability_fault() {
    let ast = resolve_source("main() {\n    mut a: i64 = 1\n    a = 2\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "cannot assign to an immutable variable"),
        0
    );
}

#[test]
fn assigning_to_immutable_parameter_reports_exactly_one_fault() {
    let ast = resolve_source("foo(a: i64) {\n    a = 2\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "cannot assign to an immutable variable"),
        1
    );
}

#[test]
fn assigning_mismatched_type_to_mutable_parameter_reports_exactly_one_fault() {
    let ast = resolve_source("foo(mut a: i64) {\n    a = \"hi\"\n}\n");
    assert_eq!(fault_count_containing(&ast, "assignment type mismatch"), 1);
}

#[test]
fn assigning_matching_type_to_mutable_parameter_reports_no_fault() {
    let ast = resolve_source("foo(mut a: i64) {\n    a = 2\n}\n");
    assert_eq!(fault_count_containing(&ast, "assignment type mismatch"), 0);
    assert_eq!(
        fault_count_containing(&ast, "cannot assign to an immutable variable"),
        0
    );
}

#[test]
fn assignment_to_generic_parameter_is_skipped_without_fault() {
    let ast = resolve_source(
        "swap<T>(mut a: T, b: T) {\n    a = b\n}\nmain() {\n    x: i64 = 1\n    y: i64 = 2\n    swap(x, y)\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "assignment type mismatch"), 0);
}
