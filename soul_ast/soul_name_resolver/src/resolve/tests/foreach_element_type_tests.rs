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
fn foreach_element_from_array_literal_can_be_used_in_binary_expression() {
    let ast = resolve_source("main() {\n    for x in [1, 2, 3] {\n        y := x + 1\n    }\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "type mismatch"),
        0,
        "{:#?}",
        ast.faults()
    );
}

#[test]
fn foreach_element_type_mismatch_in_body_reports_a_fault() {
    let ast =
        resolve_source("main() {\n    for x in [1, 2, 3] {\n        y := x + \"hi\"\n    }\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "type mismatch"),
        1,
        "{:#?}",
        ast.faults()
    );
}

#[test]
fn foreach_element_from_typed_variable_collection_is_usable() {
    let ast = resolve_source(
        "main() {\n    xs: []int = [1, 2, 3]\n    for x in xs {\n        y := x + 1\n    }\n}\n",
    );
    assert_eq!(
        fault_count_containing(&ast, "type mismatch"),
        0,
        "{:#?}",
        ast.faults()
    );
}

#[test]
fn foreach_over_undeclared_collection_is_skipped_without_fault() {
    let ast =
        resolve_source("main() {\n    for x in notAThing {\n        y := x + \"hi\"\n    }\n}\n");
    assert_eq!(
        fault_count_containing(&ast, "type mismatch"),
        0,
        "{:#?}",
        ast.faults()
    );
}
