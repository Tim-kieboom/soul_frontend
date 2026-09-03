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
fn unresolved_variable_inside_call_argument_now_reports_a_fault() {
    let ast = resolve_source("foo(a: i64) {}\nmain() {\n    foo(x)\n}\n");
    assert_eq!(fault_count_containing(&ast, "is undefined in scope"), 1);
}

#[test]
fn matching_positional_argument_type_reports_no_fault() {
    let ast = resolve_source("foo(a: i64) {}\nmain() {\n    foo(1)\n}\n");
    assert_eq!(fault_count_containing(&ast, "argument type mismatch"), 0);
}

#[test]
fn mismatched_positional_argument_type_reports_exactly_one_fault() {
    let ast = resolve_source("foo(a: i64) {}\nmain() {\n    foo(\"hi\")\n}\n");
    assert_eq!(fault_count_containing(&ast, "argument type mismatch"), 1);
}

#[test]
fn named_argument_call_is_skipped_without_fault() {
    let ast = resolve_source("foo(a: i64) {}\nmain() {\n    foo(a: \"hi\")\n}\n");
    assert_eq!(fault_count_containing(&ast, "argument type mismatch"), 0);
}

#[test]
fn arity_mismatched_call_is_skipped_without_type_fault() {
    let ast = resolve_source("foo(a: i64, b: i64) {}\nmain() {\n    foo(1)\n}\n");
    assert_eq!(fault_count_containing(&ast, "argument type mismatch"), 0);
}
