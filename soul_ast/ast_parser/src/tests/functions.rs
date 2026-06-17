use ast_model::{
    expression::{ExpressionKind, FunctionCall},
    soul_type::SoulType,
    statements::{Function, StatementKind},
};
use soul_utils::fault::Severity;

use crate::tests::{get_statement, parse};

#[test]
fn simple_function() {
    let (module, store, context) = parse("foo() {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let func_id = match &stmt.node {
        StatementKind::Function(id) => *id,
        _ => panic!("expected Function"),
    };
    let func = &store.functions[func_id];
    let Function { signature, block } = match func {
        ast_model::FunctionKind::Normal(f) => f,
        _ => panic!("expected Normal function"),
    };
    assert_eq!(signature.value.name.as_str(), "foo");
    assert_eq!(signature.value.parameters.len(), 0);
    assert_eq!(signature.value.return_type, SoulType::None);

    let body = &store.blocks[*block];
    assert!(body.statements.is_empty());
}

#[test]
fn function_with_params() {
    let (module, store, context) = parse("add(a: int, b: int) {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let func_id = match &stmt.node {
        StatementKind::Function(id) => *id,
        _ => panic!("expected Function"),
    };
    let func = &store.functions[func_id];
    let Function { signature, .. } = match func {
        ast_model::FunctionKind::Normal(f) => f,
        _ => panic!("expected Normal function"),
    };
    assert_eq!(signature.value.parameters.len(), 2);
    assert_eq!(signature.value.parameters[0].name.as_str(), "a");
    assert_eq!(signature.value.parameters[1].name.as_str(), "b");
}

#[test]
fn function_with_return_type() {
    let (module, store, context) = parse("add(a: int, b: int): int {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let func_id = match &stmt.node {
        StatementKind::Function(id) => *id,
        _ => panic!("expected Function"),
    };
    let func = &store.functions[func_id];
    let Function { signature, .. } = match func {
        ast_model::FunctionKind::Normal(f) => f,
        _ => panic!("expected Normal function"),
    };
    assert_eq!(
        signature.value.return_type,
        SoulType::Primitive(soul_utils::soul_names::PrimitiveTypes::Int)
    );
}

#[test]
fn function_with_body() {
    let (module, store, context) = parse("foo() { x := 5 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let func_id = match &stmt.node {
        StatementKind::Function(id) => *id,
        _ => panic!("expected Function"),
    };
    let func = &store.functions[func_id];
    let Function { block, .. } = match func {
        ast_model::FunctionKind::Normal(f) => f,
        _ => panic!("expected Normal function"),
    };
    let body = &store.blocks[*block];
    assert_eq!(body.statements.len(), 1);
}

// ----------------------------------------------------------------
//  Function calls
// ----------------------------------------------------------------
#[test]
fn function_call_no_args() {
    let (module, store, context) = parse("foo()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    println!("{store:#?}");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::FunctionCall(FunctionCall {
                    name, arguments, ..
                }) => {
                    assert_eq!(name.as_str(), "foo");
                    assert!(arguments.is_empty());
                }
                _ => panic!("expected FunctionCall expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn function_call_with_args() {
    let (module, store, context) = parse("add(1, 2)");
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
                ExpressionKind::FunctionCall(FunctionCall {
                    name, arguments, ..
                }) => {
                    assert_eq!(name.as_str(), "add");
                    assert_eq!(arguments.len(), 2);
                }
                _ => panic!("expected FunctionCall expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

// ----------------------------------------------------------------
//  External function
// ----------------------------------------------------------------
#[test]
fn extern_function_c() {
    let (module, store, context) = parse(r#"extern "C" printf(fmt: &char): int {}"#);
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::ExternalFunction(id) => {
            let func = &store.functions[*id];
            match func {
                ast_model::FunctionKind::External(e) => {
                    assert_eq!(e.signature.value.name.as_str(), "printf");
                    assert_eq!(e.signature.value.parameters.len(), 1);
                }
                _ => panic!("expected External function"),
            }
        }
        _ => panic!("expected ExternalFunction statement"),
    }
}

// ----------------------------------------------------------------
//  Method-style functions (with `this`)
// ----------------------------------------------------------------
#[test]
fn function_with_this_ref() {
    let (module, store, context) = parse("foo(&this) {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let func_id = match &stmt.node {
        StatementKind::Function(id) => *id,
        _ => panic!("expected Function"),
    };
    let func = &store.functions[func_id];
    let Function { signature, .. } = match func {
        ast_model::FunctionKind::Normal(f) => f,
        _ => panic!("expected Normal function"),
    };
    use ast_model::statements::FunctionThisKind;
    assert_eq!(signature.value.function_kind, FunctionThisKind::ConstRef);
}

// ----------------------------------------------------------------
//  Method call on value
// ----------------------------------------------------------------
#[test]
fn method_call() {
    let (module, store, context) = parse("obj.method()");
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
                ExpressionKind::FunctionCall(FunctionCall { name, callee, .. }) => {
                    assert_eq!(name.as_str(), "method");
                    assert!(callee.is_some());
                }
                _ => panic!("expected FunctionCall for method"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}
