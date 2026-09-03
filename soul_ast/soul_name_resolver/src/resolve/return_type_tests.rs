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
fn matching_tail_expression_reports_no_fault() {
    let ast = resolve_source("foo(): i64 {\n    1\n}\n");
    assert_eq!(fault_count_containing(&ast, "return type mismatch"), 0);
}

#[test]
fn mismatched_tail_expression_reports_exactly_one_fault() {
    let ast = resolve_source("foo(): str {\n    1\n}\n");
    assert_eq!(fault_count_containing(&ast, "return type mismatch"), 1);
}

#[test]
fn semicolon_terminated_tail_is_not_a_return_value() {
    let ast = resolve_source("foo(): i64 {\n    1;\n}\n");
    assert_eq!(fault_count_containing(&ast, "return type mismatch"), 0);
}

#[test]
fn void_function_with_stray_tail_expression_reports_a_fault() {
    let ast = resolve_source("foo() {\n    a: i64 = 1\n    a\n}\n");
    assert_eq!(fault_count_containing(&ast, "return type mismatch"), 1);
}

#[test]
fn empty_body_with_non_void_return_type_is_skipped() {
    let ast = resolve_source("foo(): i64 {}\n");
    assert_eq!(fault_count_containing(&ast, "return type mismatch"), 0);
}

#[test]
fn exhaustive_if_tail_checks_every_branch() {
    let ast = resolve_source(
        "foo(): i64 {\n    a: bool = true\n    if a {\n        1\n    } else {\n        \"hi\"\n    }\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "return type mismatch"), 1);
}

#[test]
fn exhaustive_if_tail_with_matching_branches_reports_no_fault() {
    let ast = resolve_source(
        "foo(): i64 {\n    a: bool = true\n    if a {\n        1\n    } else {\n        2\n    }\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "return type mismatch"), 0);
}

#[test]
fn non_exhaustive_if_tail_is_skipped() {
    let ast = resolve_source("foo(): i64 {\n    a: bool = true\n    if a {\n        \"hi\"\n    }\n}\n");
    assert_eq!(fault_count_containing(&ast, "return type mismatch"), 0);
}
