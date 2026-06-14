use std::path::PathBuf;

use ast_model::{
    AstStore, Module,
    expression::{
        ExpressionKind, FunctionCall,
        StructConstructor,
    },
    literal::Literal,
    operators::{BinaryOperatorKind},
    soul_type::SoulType,
    statements::{
        Assignment, Import, ImportKind, Statement, StatementKind, Variable,
    },
};
use soul_tokenizer::to_token_stream;
use soul_utils::{
    CrateContext,
    ids::IdAlloc,
    span::ModuleId,
};

use crate::{ParseInfo, parse_module};

mod variables;
mod functions;
mod literals;
mod conditional;
mod big_test;

fn module_id() -> ModuleId {
    ModuleId::begin()
}

fn parse(source: &str) -> (Module, AstStore, CrateContext) {
    let mid = module_id();
    let stream = to_token_stream(source, mid).unwrap();
    let mut store = AstStore::default();
    let mut context = CrateContext::default();
    let info = ParseInfo {
        id: mid,
        name: "test".into(),
        parent: None,
        source_folder: PathBuf::from("test"),
        store: &mut store,
        context: &mut context,
    };
    let module = parse_module(stream, info);
    (module, store, context)
}

fn get_statement<'a>(store: &'a AstStore, module: &Module, index: usize) -> &'a Statement {
    let block = &store.blocks[module.global];
    &store.statements[block.statements[index]]
}

// ----------------------------------------------------------------
//  Empty / minimal
// ----------------------------------------------------------------
#[test]
fn empty_module() {
    let (module, store, _) = parse("");
    let block = &store.blocks[module.global];
    assert!(block.statements.is_empty(), "expected no statements");
}

#[test]
fn only_newlines() {
    let (module, store, _) = parse("\n\n\n\n");
    let block = &store.blocks[module.global];
    assert!(block.statements.is_empty());
}

// ----------------------------------------------------------------
//  Binary expressions
// ----------------------------------------------------------------
#[test]
fn binary_addition() {
    let (module, store, _) = parse("1 + 2");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::Binary(bin) => {
                    assert_eq!(bin.operator.value, BinaryOperatorKind::Add);
                    let left = &store.expressions[bin.left];
                    let right = &store.expressions[bin.right];
                    assert_eq!(
                        left.node,
                        ExpressionKind::Literal((None, Literal::Uint(1)))
                    );
                    assert_eq!(
                        right.node,
                        ExpressionKind::Literal((None, Literal::Uint(2)))
                    );
                }
                _ => panic!("expected Binary expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

// ----------------------------------------------------------------
//  Assignments
// ----------------------------------------------------------------
#[test]
fn simple_assignment() {
    let (module, store, _) = parse("x = 5");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Assignment(Assignment { left, right, .. }) => {
            let l = &store.expressions[*left];
            match &l.node {
                ExpressionKind::Variable(v) => assert_eq!(v.name.as_str(), "x"),
                _ => panic!("expected Variable on LHS"),
            }
            let r = &store.expressions[*right];
            assert_eq!(r.node, ExpressionKind::Literal((None, Literal::Uint(5))));
        }
        _ => panic!("expected Assignment statement"),
    }
}

// ----------------------------------------------------------------
//  Struct constructor
// ----------------------------------------------------------------
#[test]
fn struct_constructor() {
    let (module, store, _) = parse("Point { x: 1, y: 2 }");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::StructConstructor(StructConstructor {
                    struct_type,
                    values,
                    ..
                }) => {
                    assert_eq!(
                        *struct_type,
                        SoulType::Stub(ast_model::soul_type::Stub {
                            name: "Point".into(),
                            generics: vec![]
                        })
                    );
                    assert_eq!(values.len(), 2);
                    assert_eq!(values[0].0.as_str(), "x");
                    assert_eq!(values[1].0.as_str(), "y");
                }
                _ => panic!("expected StructConstructor expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

#[test]
fn struct_constructor_shorthand() {
    let (module, store, _) = parse("Point { x, y }");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::StructConstructor(StructConstructor { values, .. }) => {
                    assert_eq!(values.len(), 2);
                }
                _ => panic!("expected StructConstructor expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

// ----------------------------------------------------------------
//  Imports
// ----------------------------------------------------------------
#[test]
fn simple_import() {
    let (module, store, _) = parse("import foo.bar");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Import(Import { paths, .. }) => {
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].kind, ImportKind::Module);
        }
        _ => panic!("expected Import statement"),
    }
}

#[test]
fn import_glob() {
    let (module, store, _) = parse("import foo.bar.*");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Import(Import { paths, .. }) => {
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].kind, ImportKind::Glob);
        }
        _ => panic!("expected Import statement"),
    }
}

// ----------------------------------------------------------------
//  Blocks
// ----------------------------------------------------------------
#[test]
fn block_expression() {
    let (module, store, _) = parse("{ x := 1 }");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::Block(block_id) => {
                    let block = &store.blocks[*block_id];
                    assert_eq!(block.statements.len(), 1);
                }
                _ => panic!("expected Block expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

// ----------------------------------------------------------------
//  New expressions
// ----------------------------------------------------------------
#[test]
fn new_ptr() {
    let (module, store, _) = parse("new(42)");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::New(inner) => {
                    let inner_expr = &store.expressions[*inner];
                    assert_eq!(
                        inner_expr.node,
                        ExpressionKind::Literal((None, Literal::Uint(42)))
                    );
                }
                _ => panic!("expected New expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}



// ----------------------------------------------------------------
//  Type alias
// ----------------------------------------------------------------
#[test]
fn type_alias() {
    let (module, store, _) = parse("type MyInt = int");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::TypeDef(def) => {
            assert_eq!(
                def.new_type,
                SoulType::Stub(ast_model::soul_type::Stub {
                    name: "MyInt".into(),
                    generics: vec![]
                })
            );
            assert_eq!(
                def.old_type,
                SoulType::Primitive(soul_utils::soul_names::PrimitiveTypes::Int)
            );
            assert!(!def.is_distinct);
        }
        _ => panic!("expected TypeDef statement"),
    }
}

// ----------------------------------------------------------------
//  Multi-statement module
// ----------------------------------------------------------------
#[test]
fn multiple_statements() {
    let source = "x := 1\ny := 2\nz := 3";
    let (module, store, _) = parse(source);
    let block = &store.blocks[module.global];
    assert_eq!(block.statements.len(), 3);
}

// ----------------------------------------------------------------
//  Error recovery — parser does not panic on bad input
// ----------------------------------------------------------------
#[test]
fn error_on_bad_token() {
    let (_, _, context) = parse("???");
    assert!(!context.faults.faults.is_empty(), "expected faults on bad input");
}

#[test]
fn error_partial_expression() {
    let (_, _, context) = parse("1 +\n");
    assert!(context.faults.faults.len() > 0);
}

// ----------------------------------------------------------------
//  Field access
// ----------------------------------------------------------------
#[test]
fn field_access() {
    let (module, store, _) = parse("obj.field");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::FieldAccess(fa) => {
                    let obj = &store.expressions[fa.object];
                    match &obj.node {
                        ExpressionKind::Variable(v) => assert_eq!(v.name.as_str(), "obj"),
                        _ => panic!("expected Variable on LHS of field access"),
                    }
                    assert_eq!(fa.field.as_str(), "field");
                }
                _ => panic!("expected FieldAccess expression"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

// ----------------------------------------------------------------
//  Nested function call chain
// ----------------------------------------------------------------
#[test]
fn chained_function_calls() {
    let (module, store, _) = parse("foo().bar()");
    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Expression { expression, .. } => {
            let expr = &store.expressions[*expression];
            match &expr.node {
                ExpressionKind::FunctionCall(FunctionCall { name, callee, .. }) => {
                    assert_eq!(name.as_str(), "bar");
                    assert!(callee.is_some());
                    let inner = &store.expressions[callee.unwrap()];
                    match &inner.node {
                        ExpressionKind::FunctionCall(FunctionCall { name: inner_name, .. }) => {
                            assert_eq!(inner_name.as_str(), "foo");
                        }
                        _ => panic!("expected inner FunctionCall"),
                    }
                }
                _ => panic!("expected outer FunctionCall"),
            }
        }
        _ => panic!("expected Expression statement"),
    }
}

// ----------------------------------------------------------------
//  Option type wrapping
// ----------------------------------------------------------------
#[test]
fn optional_type_variable() {
    let (module, store, _) = parse("x: ?int = null");
    let stmt = get_statement(&store, &module, 0);
    let Variable { ty, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert_eq!(
        *ty,
        Some(SoulType::Optional(Box::new(SoulType::Primitive(
            soul_utils::soul_names::PrimitiveTypes::Int
        ))))
    );
}

// ----------------------------------------------------------------
//  Reference type variable
// ----------------------------------------------------------------
#[test]
fn reference_type_variable() {
    let (module, store, _) = parse("x: &int = null");
    let stmt = get_statement(&store, &module, 0);
    let Variable { ty, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert_eq!(
        *ty,
        Some(SoulType::Reference(ast_model::soul_type::ReferenceType::new(
            SoulType::Primitive(soul_utils::soul_names::PrimitiveTypes::Int),
            true
        )))
    );
}