use std::path::PathBuf;

use ast_model::{AstStore, AstTree, FunctionKind};
use ast_parser::{ParseInfo, parse_module};
use mir_model::{ConstValue, Operand, Rvalue};
use soul_name_resolver::name_resolve;
use soul_tokenizer::to_token_stream;
use soul_utils::{
    FunctionId,
    collections::{crate_store::CrateStore, module_store::ModuleStore},
};

use crate::{LowerError, lower_function};

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
    assert_eq!(
        ast.faults().iter().count(),
        0,
        "test source failed to resolve: {:#?}",
        ast.faults()
    );
    ast
}

fn find_function(store: &AstStore, name: &str) -> FunctionId {
    store
        .functions
        .entries()
        .find_map(|(id, kind)| match kind {
            FunctionKind::Normal(function) if function.signature.value.name.as_str() == name => {
                Some(id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no function named `{name}` found"))
}

fn lower_source(source: &str, function_name: &str) -> Result<mir_model::MirFunction, LowerError> {
    let ast = resolve_source(source);
    let function_id = find_function(&ast.crates.store, function_name);
    lower_function(&ast.crates.store, &ast.declares, function_id)
}

#[test]
fn lowers_arithmetic_with_a_variable_and_a_return() {
    let mir = lower_source(
        "add(a: int, b: int): int {\n    c := a + b\n    return c\n}\n",
        "add",
    )
    .expect("expected successful lowering");

    assert_eq!(mir.arg_count, 2);
    assert_eq!(mir.locals.entries().count(), 4);
    assert_eq!(mir.blocks.entries().count(), 1);

    let (_, block) = mir.blocks.entries().next().expect("expected one block");
    assert_eq!(block.statements.len(), 2, "{:#?}", block.statements);
    assert!(matches!(block.terminator, mir_model::Terminator::Return));

    let mir_model::Statement::Assign(_, Rvalue::BinaryOp(op, left, right)) = &block.statements[0]
    else {
        panic!("expected first statement to assign a BinaryOp");
    };
    assert_eq!(*op, ast_model::operators::BinaryOperatorKind::Add);
    assert!(matches!(left, Operand::Copy(_)));
    assert!(matches!(right, Operand::Copy(_)));

    let mir_model::Statement::Assign(place, Rvalue::Use(Operand::Copy(_))) = &block.statements[1]
    else {
        panic!("expected second statement to assign a bare Use(Copy(..))");
    };
    assert_eq!(place.local, mir.return_local);
}

#[test]
fn lowers_a_bare_literal_return() {
    let mir =
        lower_source("answer(): int {\n    return 42\n}\n", "answer").expect("expected success");

    let (_, block) = mir.blocks.entries().next().unwrap();
    assert_eq!(block.statements.len(), 1);
    let mir_model::Statement::Assign(_, Rvalue::Use(Operand::Constant(ConstValue::Uint(42)))) =
        &block.statements[0]
    else {
        panic!(
            "expected a constant assignment, got {:#?}",
            block.statements[0]
        );
    };
}

#[test]
fn missing_return_is_rejected() {
    let result = lower_source("f(): int {\n    x := 1\n}\n", "f");
    assert!(matches!(result, Err(LowerError::MissingReturn)));
}

#[test]
fn non_primitive_return_type_is_rejected() {
    let result = lower_source("f() {\n    x := 1\n}\n", "f");
    assert!(matches!(result, Err(LowerError::UnsupportedType(_))));
}

#[test]
fn destructuring_variable_pattern_is_rejected() {
    let result = lower_source(
        "f(): int {\n    (a, b) := get_pair()\n    return a\n}\n",
        "f",
    );
    assert!(matches!(result, Err(LowerError::UnsupportedStatement(_))));
}

#[test]
fn struct_typed_parameter_is_rejected() {
    let result = lower_source(
        "struct Point { x: int }\nf(p: Point): int {\n    return p.x\n}\n",
        "f",
    );
    assert!(matches!(result, Err(LowerError::UnsupportedType(_))));
}

#[test]
fn nested_compound_expression_is_rejected() {
    let result = lower_source(
        "f(a: int, b: int, c: int): int {\n    return a + b * c\n}\n",
        "f",
    );
    assert!(matches!(result, Err(LowerError::UnsupportedExpression(_))));
}

#[test]
fn function_call_in_body_is_rejected() {
    let result = lower_source(
        "g(): int { return 1 }\nf(): int {\n    return g()\n}\n",
        "f",
    );
    assert!(matches!(result, Err(LowerError::UnsupportedExpression(_))));
}

#[test]
fn non_normal_function_is_rejected() {
    let ast = resolve_source(r#"extern "C" printf(fmt: &char): int {}"#);
    let function_id = ast
        .crates
        .store
        .functions
        .entries()
        .next()
        .map(|(id, _)| id)
        .expect("expected one function entry");

    let result = lower_function(&ast.crates.store, &ast.declares, function_id);
    assert!(matches!(result, Err(LowerError::NotANormalFunction)));
}
