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
fn redeclaring_a_variable_in_the_same_scope_reports_exactly_one_fault() {
    let ast = resolve_source("main() {\n    a := 1\n    a := 2\n}\n");
    assert_eq!(fault_count_containing(&ast, "already exists in scope"), 1);
}

#[test]
fn distinct_variable_names_in_the_same_scope_report_no_fault() {
    let ast = resolve_source("main() {\n    a := 1\n    b := 2\n}\n");
    assert_eq!(fault_count_containing(&ast, "already exists in scope"), 0);
}

#[test]
fn same_variable_name_in_separate_function_scopes_reports_no_fault() {
    let ast = resolve_source("foo() {\n    a := 1\n}\nbar() {\n    a := 2\n}\n");
    assert_eq!(fault_count_containing(&ast, "already exists in scope"), 0);
}

#[test]
fn redeclaring_a_struct_in_the_same_scope_reports_exactly_one_fault() {
    let ast = resolve_source("struct Foo {}\nstruct Foo {}\n");
    assert_eq!(
        fault_count_containing(&ast, "type of name Foo already exists in scope"),
        1
    );
}

#[test]
fn distinct_struct_names_report_no_fault() {
    let ast = resolve_source("struct Foo {}\nstruct Bar {}\n");
    assert_eq!(fault_count_containing(&ast, "already exists in scope"), 0);
}

#[test]
fn redeclaring_an_enum_in_the_same_scope_reports_exactly_one_fault() {
    let ast = resolve_source("enum Foo {\n    A\n}\nenum Foo {\n    B\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "type of name Foo already exists in scope"),
        1
    );
}

#[test]
fn a_struct_and_an_enum_sharing_a_name_reports_exactly_one_fault() {
    let ast = resolve_source("struct Foo {}\nenum Foo {\n    A\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "type of name Foo already exists in scope"),
        1
    );
}
