use std::path::PathBuf;

use ast_model::AstTree;
use ast_parser::{ParseInfo, fault::AstErrorKind, parse_module};
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

fn fault_count_matching(ast: &AstTree, predicate: impl Fn(&AstErrorKind) -> bool) -> usize {
    ast.faults()
        .iter()
        .filter(|fault| predicate(fault.kind()))
        .count()
}

fn is_already_exists_in_scope(kind: &AstErrorKind) -> bool {
    matches!(
        kind,
        AstErrorKind::TypeAlreadyExistsInScope { .. }
            | AstErrorKind::ValueAlreadyExistsInScope { .. }
    )
}

fn is_type_already_exists_named(name: &str) -> impl Fn(&AstErrorKind) -> bool + '_ {
    move |kind| {
        matches!(
            kind,
            AstErrorKind::TypeAlreadyExistsInScope { name: got } if got.as_ref() == name
        )
    }
}

#[test]
fn redeclaring_a_variable_in_the_same_scope_reports_exactly_one_fault() {
    let ast = resolve_source("main() {\n    a := 1\n    a := 2\n}\n");
    assert_eq!(fault_count_matching(&ast, is_already_exists_in_scope), 1);
}

#[test]
fn distinct_variable_names_in_the_same_scope_report_no_fault() {
    let ast = resolve_source("main() {\n    a := 1\n    b := 2\n}\n");
    assert_eq!(fault_count_matching(&ast, is_already_exists_in_scope), 0);
}

#[test]
fn same_variable_name_in_separate_function_scopes_reports_no_fault() {
    let ast = resolve_source("foo() {\n    a := 1\n}\nbar() {\n    a := 2\n}\n");
    assert_eq!(fault_count_matching(&ast, is_already_exists_in_scope), 0);
}

#[test]
fn redeclaring_a_struct_in_the_same_scope_reports_exactly_one_fault() {
    let ast = resolve_source("struct Foo {}\nstruct Foo {}\n");
    assert_eq!(
        fault_count_matching(&ast, is_type_already_exists_named("Foo")),
        1
    );
}

#[test]
fn distinct_struct_names_report_no_fault() {
    let ast = resolve_source("struct Foo {}\nstruct Bar {}\n");
    assert_eq!(fault_count_matching(&ast, is_already_exists_in_scope), 0);
}

#[test]
fn redeclaring_an_enum_in_the_same_scope_reports_exactly_one_fault() {
    let ast = resolve_source("enum Foo {\n    A\n}\nenum Foo {\n    B\n}\n");
    assert_eq!(
        fault_count_matching(&ast, is_type_already_exists_named("Foo")),
        1
    );
}

#[test]
fn a_struct_and_an_enum_sharing_a_name_reports_exactly_one_fault() {
    let ast = resolve_source("struct Foo {}\nenum Foo {\n    A\n}\n");
    assert_eq!(
        fault_count_matching(&ast, is_type_already_exists_named("Foo")),
        1
    );
}
