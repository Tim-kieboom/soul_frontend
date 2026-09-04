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
fn lambda_argument_is_no_longer_silently_unknown() {
    let ast = resolve_source("foo(a: i64) {}\nmain() {\n    foo(() => 1)\n}\n");
    assert_eq!(fault_count_containing(&ast, "argument type mismatch"), 1);
    assert!(
        ast.faults()
            .iter()
            .any(|fault| fault.message().contains("fn(0 args)")),
        "expected the fault to mention the lambda's arity"
    );
}

#[test]
fn zero_arg_and_one_arg_lambdas_are_distinct_types() {
    let ast = resolve_source("same<T>(a: T, b: T) {}\nmain() {\n    same(() => 1, x => x)\n}\n");
    assert_eq!(fault_count_containing(&ast, "generic parameter"), 1);
}

#[test]
fn same_arity_lambdas_used_for_same_generic_report_no_fault() {
    let ast = resolve_source("same<T>(a: T, b: T) {}\nmain() {\n    same(() => 1, () => 2)\n}\n");
    assert_eq!(fault_count_containing(&ast, "generic parameter"), 0);
}

#[test]
fn lambda_return_type_is_inferred_from_tail_position() {
    let ast = resolve_source("foo(a: str) {}\nmain() {\n    foo(() => 1)\n}\n");
    assert!(
        fault_count_containing(&ast, "fn(0 args) -> int") > 0,
        "expected the inferred return type in a fault message: {:#?}",
        ast.faults().iter().map(|f| f.message()).collect::<Vec<_>>()
    );
}

#[test]
fn lambda_first_return_establishes_type_even_from_a_non_exhaustive_if() {
    let ast = resolve_source(
        "foo(a: str) {}\nmain() {\n    foo(x => {\n        if x {\n            1\n        }\n    })\n}\n",
    );
    assert!(
        fault_count_containing(&ast, "-> int") > 0,
        "expected the first branch's value to establish `int`: {:#?}",
        ast.faults().iter().map(|f| f.message()).collect::<Vec<_>>()
    );
}

#[test]
fn lambda_with_exhaustive_if_tail_infers_shared_branch_type() {
    let ast = resolve_source(
        "foo(a: str) {}\nmain() {\n    foo(x => {\n        if x {\n            1\n        } else {\n            2\n        }\n    })\n}\n",
    );
    assert!(
        fault_count_containing(&ast, "-> int") > 0,
        "expected both `if`/`else` branches' shared type to be inferred: {:#?}",
        ast.faults().iter().map(|f| f.message()).collect::<Vec<_>>()
    );
}

#[test]
fn lambda_body_that_is_a_bare_return_infers_the_returned_values_type() {
    let ast = resolve_source("foo(a: str) {}\nmain() {\n    foo(() => return 2)\n}\n");
    assert!(
        fault_count_containing(&ast, "fn(0 args) -> int") > 0,
        "expected `return 2` in tail position to infer `untypedUint`: {:#?}",
        ast.faults().iter().map(|f| f.message()).collect::<Vec<_>>()
    );
}

#[test]
fn calling_a_variable_bound_lambda_uses_its_inferred_return_type() {
    let ast = resolve_source(
        "assertEq<T>(a: T, b: T) {}\nmain() {\n    fn := () => 2\n    assertEq(fn(), \"\")\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "generic parameter"), 1);
}

#[test]
fn calling_a_variable_bound_lambda_with_matching_type_reports_no_fault() {
    let ast = resolve_source(
        "assertEq<T>(a: T, b: T) {}\nmain() {\n    fn := () => 2\n    assertEq(fn(), 3)\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "generic parameter"), 0);
}

#[test]
fn calling_a_variable_bound_lambda_with_return_body_and_matching_type_reports_no_fault() {
    // The exact reported repro: `fn := () => return 2`, then both
    // `assertEq(fn(), "")` (should fault) and `assertEq(fn(), 2)` (should not).
    let ast = resolve_source(
        "assertEq<T>(a: T, b: T) {}\nmain() {\n    fn := () => return 2\n    assertEq(fn(), \"\")\n    assertEq(fn(), 2)\n}\n",
    );
    assert_eq!(fault_count_containing(&ast, "generic parameter"), 1);
}

#[test]
fn lambda_first_return_in_if_branch_establishes_type_and_later_return_faults() {
    let ast = resolve_source(
        "assertEq<T>(a: T, b: T) {}\nmain() {\n    fn := () => {\n        if true {\n            return 2\n        }\n        return \"\"\n    }\n}\n",
    );
    assert_eq!(
        fault_count_containing(&ast, "return type mismatch: expected `int`, got `str`"),
        1,
    );
}

#[test]
fn lambda_with_divergent_if_tail_branches_faults_on_the_later_branch() {
    let ast = resolve_source(
        "foo(a: str) {}\nmain() {\n    foo(x => {\n        if x {\n            1\n        } else {\n            \"hi\"\n        }\n    })\n}\n",
    );
    assert_eq!(
        fault_count_containing(&ast, "return type mismatch: expected `int`, got `str`"),
        1,
    );
    assert!(
        fault_count_containing(&ast, "-> int") > 0,
        "expected the first branch's value to establish `int`: {:#?}",
        ast.faults().iter().map(|f| f.message()).collect::<Vec<_>>()
    );
}
