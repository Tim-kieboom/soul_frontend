use ast_model::{
    expression::ExpressionKind,
    literal::Literal,
    operators::BinaryOperatorKind,
    statements::StatementKind,
};
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
fn expression_fstring() {
    let (module, store, context) = parse(r#"f"hello {1 + 2} world""#);
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
            match &expr.node {
                ExpressionKind::StringFormat(fmt) => {
                    assert!(fmt.to_string);
                    assert_eq!(fmt.trailing, " world");
                    assert_eq!(fmt.parts.len(), 1);
                    let (text, expr_id) = &fmt.parts[0];
                    assert_eq!(text, "hello ");
                    let inner = &store.expressions[*expr_id];
                    match &inner.node {
                        ExpressionKind::Binary(bin) => {
                            assert_eq!(bin.operator.value, BinaryOperatorKind::Add);
                            let left = &store.expressions[bin.left];
                            let right = &store.expressions[bin.right];
                            assert_eq!(left.node, ExpressionKind::Literal((None, Literal::Uint(1))));
                            assert_eq!(right.node, ExpressionKind::Literal((None, Literal::Uint(2))));
                        }
                        other => panic!("expected Binary expression, got {other:?}"),
                    }
                }
                other => panic!("expected StringFormat expression, got {other:?}"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn expression_fstring_fstr_tag() {
    let (module, store, context) = parse(r#"fstr"hello {42}""#);
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
            match &expr.node {
                ExpressionKind::StringFormat(fmt) => {
                    assert!(!fmt.to_string);
                    assert_eq!(fmt.trailing, "");
                    assert_eq!(fmt.parts.len(), 1);
                    let (text, expr_id) = &fmt.parts[0];
                    assert_eq!(text, "hello ");
                    let inner = &store.expressions[*expr_id];
                    assert_eq!(inner.node, ExpressionKind::Literal((None, Literal::Uint(42))));
                }
                other => panic!("expected StringFormat expression, got {other:?}"),
            }
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
