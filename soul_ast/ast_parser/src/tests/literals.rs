use ast_model::{expression::ExpressionKind, literal::Literal, statements::StatementKind};
use soul_utils::fault::Severity;

use crate::tests::{get_statement, parse};

#[test]
fn expression_literal_int() {
    let (module, store, context) = parse("42");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            assert_eq!(
                expr.node,
                ExpressionKind::Literal((None, Literal::Uint(42)))
            );
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn expression_literal_float() {
    let (module, store, context) = parse("3.14");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            assert_eq!(
                expr.node,
                ExpressionKind::Literal((None, Literal::Float(3.14)))
            );
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn expression_literal_string() {
    let (module, store, context) = parse(r#""hello""#);
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            assert_eq!(
                expr.node,
                ExpressionKind::Literal((None, Literal::Str("hello".into())))
            );
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn expression_literal_bool() {
    let (module, store, context) = parse("true");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            assert_eq!(
                expr.node,
                ExpressionKind::Literal((None, Literal::Bool(true)))
            );
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn expression_null() {
    let (module, store, context) = parse("null");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            assert_eq!(expr.node, ExpressionKind::Null(None));
        }
        _ => panic!("expected Expression statement"),
    }
}
