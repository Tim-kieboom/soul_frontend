use std::path::PathBuf;

use ast_model::{
    AstTree,
    soul_type::SoulType,
    statements::{StatementKind, VarPattern},
};
use ast_parser::{ParseInfo, parse_module};
use soul_tokenizer::to_token_stream;
use soul_utils::{
    collections::{crate_store::CrateStore, module_store::ModuleStore},
    soul_names::PrimitiveTypes,
};

use crate::name_resolve;

/// Tokenizes, parses, and name-resolves a single-file snippet with no imports,
/// returning the resulting `AstTree` so tests can inspect its declares/faults.
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

/// Finds the inferred/declared type of the first `let`/`mut` binding named `name`.
fn variable_type(ast: &AstTree, name: &str) -> Option<SoulType> {
    ast.crates.store.statements.values().find_map(|statement| {
        let StatementKind::Variable(variable) = &statement.node else {
            return None;
        };
        let VarPattern::Simple { binding, .. } = &variable.pattern else {
            return None;
        };
        if binding.ident.as_str() != name {
            return None;
        }
        ast.declares
            .get_variable_type(variable.id)
            .and_then(|(_, ty, _)| ty.clone())
    })
}

#[test]
fn bare_uint_literal_binding_infers_uint() {
    // A bare non-negative literal tokenizes as `Number::Uint`, not `Int` —
    // see soul_tokenizer's lexer tests.
    let ast = resolve_source("main() {\n    a := 5\n}\n");
    assert_eq!(
        variable_type(&ast, "a"),
        Some(SoulType::Primitive(PrimitiveTypes::Uint))
    );
}

#[test]
fn negative_int_literal_binding_infers_int() {
    let ast = resolve_source("main() {\n    a := -5\n}\n");
    assert_eq!(
        variable_type(&ast, "a"),
        Some(SoulType::Primitive(PrimitiveTypes::Int))
    );
}

#[test]
fn bool_literal_binding_infers_bool() {
    let ast = resolve_source("main() {\n    a := true\n}\n");
    assert_eq!(
        variable_type(&ast, "a"),
        Some(SoulType::Primitive(PrimitiveTypes::Boolean))
    );
}

#[test]
fn str_literal_binding_infers_string() {
    let ast = resolve_source("main() {\n    a := \"hi\"\n}\n");
    assert_eq!(variable_type(&ast, "a"), Some(SoulType::String));
}

#[test]
fn explicit_annotation_is_not_overridden_by_literal_inference() {
    let ast = resolve_source("main() {\n    a: i64 = 5\n}\n");
    assert_eq!(
        variable_type(&ast, "a"),
        Some(SoulType::Primitive(PrimitiveTypes::Int64))
    );
}

#[test]
fn non_literal_initializer_is_left_unresolved() {
    let ast = resolve_source("main() {\n    b := 1\n    a := b\n}\n");
    assert_eq!(variable_type(&ast, "a"), None);
}
