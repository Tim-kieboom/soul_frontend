use ast_model::{expression::ExpressionKind, literal::Literal, statements::StatementKind};

use crate::tests::{get_statement, parse};

#[test]
fn expression_literal_int() {
    let (module, store, _) = parse("42");
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
    let (module, store, _) = parse("3.14");
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
    let (module, store, _) = parse(r#""hello""#);
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
    let (module, store, _) = parse("true");
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
    let (module, store, _) = parse("null");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            assert_eq!(expr.node, ExpressionKind::Null(None));
        }
        _ => panic!("expected Expression statement"),
    }
}
