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
fn field_access_value_can_be_used_in_a_binary_expression() {
    let ast = resolve_source(
        "struct Point {\n    x: i64\n}\nmain() {\n    p: Point = Point{x: 5}\n    y := p.x + 1\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 0);
}

#[test]
fn mismatched_field_access_value_in_a_binary_expression_reports_a_fault() {
    let ast = resolve_source(
        "struct Point {\n    x: i64\n}\nmain() {\n    p: Point = Point{x: 5}\n    y := p.x + \"hi\"\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 1);
}

#[test]
fn method_call_on_a_nested_field_access_resolves() {
    let ast = resolve_source(
        "struct Inner {\n    n: i64\n}\nuse Inner {\n    get(&this): i64 => this.n\n}\nstruct Outer {\n    inner: Inner\n}\nmain() {\n    o: Outer = Outer{inner: Inner{n: 5}}\n    x := o.inner.get()\n}\n",
    );
    assert_eq!(ast.faults().iter().count(), 0);
}

#[test]
fn field_access_on_an_undeclared_type_is_skipped_without_fault() {
    let ast = resolve_source("main() {\n    y := notAThing.field\n}\n");
    assert_eq!(fault_count_containing(&ast, "type mismatch"), 0);
}
