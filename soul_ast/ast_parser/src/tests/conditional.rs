use ast_model::{
    expression::{ExpressionKind, If, IfBranch, IfCondition, Match, MatchPattern},
    literal::Literal,
    statements::StatementKind,
};
use soul_utils::fault::Severity;

use crate::tests::{get_statement, parse};

// ----------------------------------------------------------------
//  If / else
// ----------------------------------------------------------------
#[test]
fn if_statement() {
    let (module, store, context) = parse("if true {}");
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
                ExpressionKind::If(If {
                    condition,
                    block,
                    branch,
                    ..
                }) => {
                    let IfCondition::Expression(expression_id) = condition else {
                        panic!("condition is wrongtype");
                    };

                    let cond = &store.expressions[*expression_id];
                    assert!(matches!(
                        cond.node,
                        ExpressionKind::Literal((_, Literal::Bool(true)))
                    ));
                    assert!(branch.is_none());
                    let body = &store.blocks[*block];
                    assert!(body.statements.is_empty());
                }
                _ => panic!("expected If expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn if_else_statement() {
    let (module, store, context) = parse("if true {} else {}");
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
                ExpressionKind::If(If { branch, .. }) => match branch.as_ref().unwrap() {
                    IfBranch::Else(block_id) => {
                        let body = &store.blocks[*block_id];
                        assert!(body.statements.is_empty());
                    }
                    _ => panic!("expected Else branch"),
                },
                _ => panic!("expected If expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

// ----------------------------------------------------------------
//  Bad-path: if / else
// ----------------------------------------------------------------
#[test]
fn double_else_is_rejected() {
    let (_, _, context) = parse("if true {} else {} else {}");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error for a second 'else' after 'else': {:#?}",
        context.faults.faults
    );
}

#[test]
fn if_missing_condition_is_rejected() {
    let (_, _, context) = parse("if {}");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when 'if' has no condition"
    );
}

#[test]
fn if_missing_block_is_rejected() {
    let (_, _, context) = parse("if true");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when 'if' has no block"
    );
}

// ----------------------------------------------------------------
//  Match
// ----------------------------------------------------------------
#[test]
fn match_statement() {
    let (module, store, context) = parse(r#"match x { _ => true }"#);
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
                ExpressionKind::Match(Match {
                    scrutinee, arms, ..
                }) => {
                    let scrut = &store.expressions[*scrutinee];
                    match &scrut.node {
                        ExpressionKind::Variable(v) => assert_eq!(v.name.as_str(), "x"),
                        _ => panic!("expected Variable scrutinee"),
                    }
                    assert_eq!(arms.len(), 1);
                    assert_eq!(arms[0].pattern, MatchPattern::Wildcard);
                }
                _ => panic!("expected Match expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

// ----------------------------------------------------------------
//  Bad-path: match
// ----------------------------------------------------------------
#[test]
fn match_arm_missing_arrow_is_rejected() {
    let (_, _, context) = parse("match x { _ 1 }");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when a match arm has no '=>'"
    );
    assert!(
        context
            .faults
            .faults
            .iter()
            .any(|fault| fault.message().contains("expected '=>' in match arm")),
        "{:#?}",
        context.faults.faults
    );
}

#[test]
fn unclosed_match_body_is_rejected() {
    let (_, _, context) = parse("match x { _ => true");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error on an unclosed match body"
    );
}

// ----------------------------------------------------------------
//  Return / Break / Continue
// ----------------------------------------------------------------
#[test]
fn return_void() {
    let (module, store, context) = parse("return");
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
            assert_eq!(expr.node, ExpressionKind::Return(None));
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn return_with_value() {
    let (module, store, context) = parse("return 42");
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
                ExpressionKind::Return(Some(val)) => {
                    let val_expr = &store.expressions[*val];
                    assert!(matches!(
                        val_expr.node,
                        ExpressionKind::Literal((_, Literal::Uint(42)))
                    ));
                }
                _ => panic!("expected Return(Some)"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn break_statement() {
    let (module, store, context) = parse("break");
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
            assert_eq!(expr.node, ExpressionKind::Break);
        }
        _ => panic!("expected Expression statement"),
    }
}
