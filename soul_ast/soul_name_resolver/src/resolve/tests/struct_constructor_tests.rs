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
fn matching_field_types_report_no_fault() {
    let ast = resolve_source(
        "struct Point { x: i64\n    y: i64 }\nmain() {\n    Point { x: 1, y: 2 }\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 0);
    assert_eq!(fault_count_containing(&ast, "has no field"), 0);
}

#[test]
fn mismatched_field_type_reports_exactly_one_fault() {
    let ast = resolve_source(
        "struct Point { x: i64\n    y: i64 }\nmain() {\n    Point { x: 1, y: \"hi\" }\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "field `y` type mismatch"), 1);
}

#[test]
fn unknown_field_name_reports_exactly_one_fault() {
    let ast = resolve_source(
        "struct Point { x: i64\n    y: i64 }\nmain() {\n    Point { x: 1, z: 2 }\n}\n",
    );
    assert_eq!(
        fault_count_containing(&ast, "struct `Point` has no field `z`"),
        1
    );
}

#[test]
fn generic_struct_field_is_skipped_without_fault() {
    let ast = resolve_source("struct Box<T> { value: T }\nmain() {\n    Box { value: 1 }\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "field `value` type mismatch"),
        0
    );
}
