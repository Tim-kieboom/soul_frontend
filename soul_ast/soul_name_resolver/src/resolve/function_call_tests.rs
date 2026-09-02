use std::path::PathBuf;

use ast_model::AstTree;
use ast_parser::{ParseInfo, parse_module};
use soul_tokenizer::to_token_stream;
use soul_utils::collections::{crate_store::CrateStore, module_store::ModuleStore};

use crate::name_resolve;

/// Tokenizes, parses, and name-resolves a single-file snippet with no imports,
/// returning the resulting `AstTree` so tests can inspect its faults/resolves.
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

fn intrinsic_fault_count(ast: &AstTree) -> usize {
    ast.faults()
        .iter()
        .filter(|fault| fault.message().contains("intrinsic"))
        .count()
}

#[test]
fn known_intrinsics_with_correct_arity_resolve_without_faults() {
    let ast = resolve_source(
        r#"
main() {
    a := intrinsic.fieldIndex(int, 0)
    b := intrinsic.fieldCount(int)
    c := intrinsic.typeinfo(int)
    d := intrinsic.ptr.offset(int, 1)
}
"#,
    );

    assert_eq!(intrinsic_fault_count(&ast), 0);
}

#[test]
fn unknown_intrinsic_reports_exactly_one_fault() {
    let ast = resolve_source(
        r#"
main() {
    a := intrinsic.doesNotExist(1)
}
"#,
    );

    assert_eq!(intrinsic_fault_count(&ast), 1);
}

#[test]
fn wrong_arity_reports_exactly_one_fault() {
    let ast = resolve_source(
        r#"
main() {
    a := intrinsic.typeinfo()
}
"#,
    );

    assert_eq!(intrinsic_fault_count(&ast), 1);
}
