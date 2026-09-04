use ast_model::{
    expression::ExpressionKind,
    statements::{StatementKind, VarPattern},
};
use soul_utils::{TypeModifier, fault::Severity};

use crate::tests::{get_statement, parse};

#[test]
fn simple_lambda() {
    let (module, store, context) = parse("x => x + 1");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let expr = match &stmt.node {
        StatementKind::Expression { expression, .. } => expression,
        _ => panic!("expected Expression statement"),
    };
    let lambda = match &store.expressions[*expr].node {
        ExpressionKind::Lambda(l) => l,
        other => panic!("expected Lambda, got {:?}", other),
    };
    assert_eq!(lambda.parameters.len(), 1);
    assert!(matches!(
        &lambda.parameters[0],
        VarPattern::Simple {
            binding,
            modifier: TypeModifier::Const,
        } if binding.ident.as_str() == "x"
    ));
}

#[test]
fn multi_param_lambda() {
    let (module, store, context) = parse("(a, b) => a + b");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let expr = match &stmt.node {
        StatementKind::Expression { expression, .. } => expression,
        _ => panic!("expected Expression statement"),
    };
    let lambda = match &store.expressions[*expr].node {
        ExpressionKind::Lambda(l) => l,
        other => panic!("expected Lambda, got {:?}", other),
    };
    assert_eq!(lambda.parameters.len(), 2);
    assert!(matches!(
        &lambda.parameters[0],
        VarPattern::Simple { binding, .. } if binding.ident.as_str() == "a"
    ));
    assert!(matches!(
        &lambda.parameters[1],
        VarPattern::Simple { binding, .. } if binding.ident.as_str() == "b"
    ));
}

#[test]
fn discard_param_lambda() {
    let (module, store, context) = parse("_ => 42");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let expr = match &stmt.node {
        StatementKind::Expression { expression, .. } => expression,
        _ => panic!("expected Expression statement"),
    };
    let lambda = match &store.expressions[*expr].node {
        ExpressionKind::Lambda(l) => l,
        other => panic!("expected Lambda, got {:?}", other),
    };
    assert_eq!(lambda.parameters.len(), 1);
    assert!(matches!(&lambda.parameters[0], VarPattern::Discard));
}

#[test]
fn lambda_in_function_call() {
    let (module, store, context) = parse(r#"map(xs, x => x + 1)"#);
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let expr = match &stmt.node {
        StatementKind::Expression { expression, .. } => expression,
        _ => panic!("expected Expression statement"),
    };
    let call = match &store.expressions[*expr].node {
        ExpressionKind::FunctionCall(c) => c,
        other => panic!("expected FunctionCall, got {:?}", other),
    };
    assert_eq!(call.arguments.len(), 2);
    let second_arg = &store.expressions[call.arguments[1].value];
    let lambda = match &second_arg.node {
        ExpressionKind::Lambda(l) => l,
        other => panic!("expected Lambda argument, got {:?}", other),
    };
    assert_eq!(lambda.parameters.len(), 1);
}

// ----------------------------------------------------------------
//  Bad-path: malformed lambdas
// ----------------------------------------------------------------
#[test]
fn lambda_missing_body_is_rejected() {
    let (_, _, context) = parse("x => ");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when a lambda has no body"
    );
}

#[test]
fn lambda_unclosed_tuple_params_is_rejected() {
    let (_, _, context) = parse("(a, b => a + b");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error for an unclosed tuple-parameter list"
    );
}

#[test]
fn lambda_in_variable() {
    let (module, store, context) = parse("f := x => x + 1");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let var = match &stmt.node {
        StatementKind::Variable(v) => v,
        other => panic!("expected Variable, got {:?}", other),
    };
    assert!(var.initialize_value.is_some());
    let init = &store.expressions[var.initialize_value.unwrap()];
    let lambda = match &init.node {
        ExpressionKind::Lambda(l) => l,
        other => panic!("expected Lambda initializer, got {:?}", other),
    };
    assert_eq!(lambda.parameters.len(), 1);
}
