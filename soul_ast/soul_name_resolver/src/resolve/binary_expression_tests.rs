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

fn expression_type_of_binding(ast: &AstTree, name: &str) -> Option<SoulType> {
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
            .get_expression_type(variable.initialize_value?)
            .cloned()
    })
}

fn type_mismatch_fault_count(ast: &AstTree) -> usize {
    ast.faults()
        .iter()
        .filter(|fault| fault.message().contains("type mismatch"))
        .count()
}

#[test]
fn same_type_arithmetic_binary_infers_operand_type() {
    let ast = resolve_source("main() {\n    a: i64 = 1\n    b: i64 = 2\n    c := a + b\n}\n");
    assert_eq!(
        expression_type_of_binding(&ast, "c"),
        Some(SoulType::Primitive(PrimitiveTypes::Int64))
    );
    assert_eq!(type_mismatch_fault_count(&ast), 0);
}

#[test]
fn same_type_comparison_binary_infers_bool() {
    let ast = resolve_source("main() {\n    a: i64 = 1\n    b: i64 = 2\n    c := a == b\n}\n");
    assert_eq!(
        expression_type_of_binding(&ast, "c"),
        Some(SoulType::Primitive(PrimitiveTypes::Boolean))
    );
}

#[test]
fn mismatched_operand_types_report_exactly_one_fault() {
    let ast = resolve_source("main() {\n    a: i64 = 1\n    b: u64 = 2\n    c := a + b\n}\n");
    assert_eq!(type_mismatch_fault_count(&ast), 1);
    assert_eq!(expression_type_of_binding(&ast, "c"), None);
}

#[test]
fn untyped_literal_operand_coerces_to_concrete_operand_type() {
    let ast = resolve_source("main() {\n    a: i64 = 1\n    c := a + 5\n}\n");
    assert_eq!(type_mismatch_fault_count(&ast), 0);
    assert_eq!(
        expression_type_of_binding(&ast, "c"),
        Some(SoulType::Primitive(PrimitiveTypes::Int64))
    );
}

#[test]
fn incompatible_untyped_literal_operand_reports_exactly_one_fault() {
    let ast = resolve_source("main() {\n    a: str = \"hi\"\n    c := a + 5\n}\n");
    assert_eq!(type_mismatch_fault_count(&ast), 1);
    assert_eq!(expression_type_of_binding(&ast, "c"), None);
}

#[test]
fn two_untyped_int_literals_combine_and_stay_untyped() {
    let ast = resolve_source("main() {\n    c := 5 + -3\n}\n");
    assert_eq!(type_mismatch_fault_count(&ast), 0);
    assert_eq!(
        expression_type_of_binding(&ast, "c"),
        Some(SoulType::Primitive(PrimitiveTypes::UntypedInt))
    );
}

#[test]
fn untyped_chain_with_float_stays_untyped_float() {
    let ast = resolve_source("main() {\n    c := 5 + -3 + 2.0\n}\n");
    assert_eq!(type_mismatch_fault_count(&ast), 0);
    assert_eq!(
        expression_type_of_binding(&ast, "c"),
        Some(SoulType::Primitive(PrimitiveTypes::UntypedFloat))
    );
}

#[test]
fn operand_from_function_call_uses_its_return_type() {
    let ast = resolve_source(
        "foo(): i64 {\n    return 1\n}\nmain() {\n    a: i64 = 1\n    c := a + foo()\n}\n",
    );
    assert_eq!(type_mismatch_fault_count(&ast), 0);
    assert_eq!(
        expression_type_of_binding(&ast, "c"),
        Some(SoulType::Primitive(PrimitiveTypes::Int64))
    );
}
