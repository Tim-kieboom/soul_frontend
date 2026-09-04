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
fn variable_from_a_function_call_can_be_used_in_a_binary_expression() {
    let ast =
        resolve_source("foo(): i64 {\n    1\n}\nmain() {\n    a := foo()\n    b := a + 1\n}\n");
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 0);
}

#[test]
fn mismatched_use_of_a_function_call_initialized_variable_reports_a_fault() {
    let ast = resolve_source(
        "foo(): i64 {\n    1\n}\nmain() {\n    a := foo()\n    b := a + \"hi\"\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 1);
}

#[test]
fn variable_from_a_binary_expression_can_be_used_in_another_binary_expression() {
    let ast = resolve_source("main() {\n    a := 1 + 2\n    b := a + 3\n}\n");
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 0);
}

#[test]
fn variable_from_a_function_call_can_be_used_as_a_method_call_receiver() {
    let ast = resolve_source(
        "struct Counter {\n    n: i64\n}\nuse Counter {\n    get(&this): i64 => this.n\n}\nmakeCounter(): Counter => Counter{n: 5}\nmain() {\n    c := makeCounter()\n    x := c.get()\n}\n",
    );
    assert_eq!(ast.faults().iter().count(), 0);
}
