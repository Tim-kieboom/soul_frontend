use ast_model::{
    expression::{ExpressionKind, If, IfBranch, Match, MatchPattern},
    literal::Literal,
    statements::StatementKind,
};

use crate::tests::{get_statement, parse};

// ----------------------------------------------------------------
//  If / else
// ----------------------------------------------------------------
#[test]
fn if_statement() {
    let (module, store, _) = parse("if true {}");
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
                    let cond = &store.expressions[*condition];
                    assert_eq!(
                        cond.node,
                        ExpressionKind::Literal((None, Literal::Bool(true)))
                    );
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
    let (module, store, _) = parse("if true {} else {}");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::If(If { branch, .. }) => match branch.as_ref().unwrap().as_ref() {
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
//  Match
// ----------------------------------------------------------------
#[test]
fn match_statement() {
    let (module, store, _) = parse(r#"match x { _ => true }"#);
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
//  Return / Break / Continue
// ----------------------------------------------------------------
#[test]
fn return_void() {
    let (module, store, _) = parse("return");
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
    let (module, store, _) = parse("return 42");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::Return(Some(val)) => {
                    let val_expr = &store.expressions[*val];
                    assert_eq!(
                        val_expr.node,
                        ExpressionKind::Literal((None, Literal::Uint(42)))
                    );
                }
                _ => panic!("expected Return(Some)"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn break_statement() {
    let (module, store, _) = parse("break");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            assert_eq!(expr.node, ExpressionKind::Break);
        }
        _ => panic!("expected Expression statement"),
    }
}
