use ast_model::{expression::{ExpressionKind, FunctionCall}, soul_type::SoulType, statements::{Function, StatementKind}};

use crate::tests::{get_statement, parse};

#[test]
fn simple_function() {
    let (module, store, _) = parse("fn foo() {}");
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
    let (module, store, _) = parse("fn add(a: int, b: int) {}");
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
    let (module, store, _) = parse("fn add(a: int, b: int): int {}");
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
    let (module, store, _) = parse("fn foo() { x := 5 }");
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
    let (module, store, _) = parse("foo()");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::FunctionCall(FunctionCall { name, arguments, .. }) => {
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
    let (module, store, _) = parse("add(1, 2)");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::FunctionCall(FunctionCall { name, arguments, .. }) => {
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
    let (module, store, _) = parse(r#"extern "C" printf(fmt: &char): int {}"#);
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
    let (module, store, _) = parse("fn foo(&this) {}");
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
    assert_eq!(
        signature.value.function_kind,
        FunctionThisKind::MutRef
    );
}

// ----------------------------------------------------------------
//  Method call on value
// ----------------------------------------------------------------
#[test]
fn method_call() {
    let (module, store, _) = parse("obj.method()");
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