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
fn bare_undefined_variable_reports_exactly_one_fault() {
    let ast = resolve_source("main() {\n    b := a\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "variable 'a' is undefined in scope"),
        1
    );
}

#[test]
fn defined_variable_reports_no_undefined_fault() {
    let ast = resolve_source("main() {\n    a := 1\n    b := a\n}\n");
    assert_eq!(fault_count_containing(&ast, "is undefined in scope"), 0);
}

#[test]
fn calling_an_undefined_function_is_not_yet_caught() {
    let ast = resolve_source("main() {\n    thisFunctionDoesNotExist()\n}\n");
    assert_eq!(ast.faults().iter().count(), 0);
}

#[test]
fn calling_a_defined_function_reports_no_fault() {
    let ast = resolve_source("foo() {}\nmain() {\n    foo()\n}\n");
    assert_eq!(ast.faults().iter().count(), 0);
}
