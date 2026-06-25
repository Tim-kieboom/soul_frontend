use ast_model::{
    expression::ExpressionKind,
    literal::Literal,
    soul_type::SoulType,
    statements::{StatementKind, Variable},
};
use soul_utils::{TypeModifier, fault::Severity};

use crate::tests::{get_statement, parse};

#[test]
fn variable_declaration_with_init() {
    let (module, store, context) = parse("x := 5");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable {
        name,
        modifier,
        ty,
        initialize_value,
        ..
    } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert_eq!(name.as_str(), "x");
    assert_eq!(*modifier, TypeModifier::Const);
    assert!(ty.is_none());
    assert!(initialize_value.is_some());

    let init = &store.expressions[initialize_value.unwrap()];
    assert!(matches!(init.node, ExpressionKind::Literal((_, Literal::Uint(5)))));
}

#[test]
fn variable_declaration_typed() {
    let (module, store, context) = parse("x: int = 10");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { ty, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert_eq!(
        *ty,
        Some(SoulType::Primitive(
            soul_utils::soul_names::PrimitiveTypes::Int
        ))
    );
}

#[test]
fn variable_declaration_no_init() {
    let (module, store, context) = parse("mut x: int");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable {
        initialize_value,
        name,
        modifier,
        ..
    } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert_eq!(name.as_str(), "x");
    assert_eq!(*modifier, TypeModifier::Mut);
    assert!(initialize_value.is_none());
}

#[test]
fn mutable_variable() {
    let (module, store, context) = parse("mut x := 5");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { name, modifier, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert_eq!(name.as_str(), "x");
    assert_eq!(*modifier, TypeModifier::Mut);
}
