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
fn correct_arity_and_type_reports_no_fault() {
    let ast = resolve_source(
        "union Literal {\n    None,\n    Int(int)\n}\nmain() {\n    x := Literal.Int(1)\n}\n",
    );
    assert_eq!(
        fault_count_containing(&ast, "variant"),
        0,
        "{:#?}",
        ast.faults()
    );
}

#[test]
fn wrong_arity_reports_exactly_one_fault() {
    let ast = resolve_source(
        "union Literal {\n    None,\n    Int(int)\n}\nmain() {\n    x := Literal.Int(1, 2)\n}\n",
    );
    assert_eq!(
        fault_count_containing(&ast, "expects 1 argument(s), got 2"),
        1,
        "{:#?}",
        ast.faults()
    );
}

#[test]
fn wrong_argument_type_reports_exactly_one_fault() {
    let ast = resolve_source(
        "union Literal {\n    None,\n    Int(int)\n}\nmain() {\n    x := Literal.Int(\"hi\")\n}\n",
    );
    assert_eq!(
        fault_count_containing(&ast, "argument type mismatch"),
        1,
        "{:#?}",
        ast.faults()
    );
}

#[test]
fn unrelated_call_on_undeclared_type_is_left_unresolved_without_fault() {
    let ast = resolve_source("main() {\n    x := NotAType.Whatever(1)\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "variant"),
        0,
        "{:#?}",
        ast.faults()
    );
}

#[test]
fn method_call_on_a_variable_is_not_treated_as_variant_construction() {
    let ast = resolve_source(
        "union Literal {\n    None,\n    Int(int)\n}\nuse Literal {\n    Int(&this): int => 1\n}\nmain() {\n    x := Literal.None\n    y := x.Int()\n}\n",
    );
    assert_eq!(
        fault_count_containing(&ast, "expects"),
        0,
        "{:#?}",
        ast.faults()
    );
}
