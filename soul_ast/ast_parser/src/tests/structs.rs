use ast_model::{soul_type::SoulType, statements::StatementKind};
use soul_utils::fault::Severity;

use crate::tests::{get_statement, parse};

#[test]
fn empty_struct() {
    let (module, store, context) = parse("struct Foo {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let struct_ = match &stmt.node {
        StatementKind::Struct(s) => s,
        _ => panic!("expected Struct"),
    };
    assert_eq!(struct_.name.as_str(), "Foo");
    assert!(struct_.fields.is_empty());
    assert!(struct_.generics.is_empty());
    assert!(struct_.statements.is_empty());
}

#[test]
fn struct_with_fields() {
    let (module, store, context) = parse("struct Foo { pub x: int = 5 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let struct_ = match &stmt.node {
        StatementKind::Struct(s) => s,
        _ => panic!("expected Struct"),
    };
    assert_eq!(struct_.fields.len(), 1);
    assert_eq!(struct_.fields[0].value.name.as_str(), "x");
    assert!(struct_.fields[0].is_public);
    assert_eq!(
        struct_.fields[0].value.ty,
        Some(SoulType::Primitive(
            soul_utils::soul_names::PrimitiveTypes::Int
        ))
    );
    assert!(struct_.fields[0].value.initialize_value.is_some());
}

#[test]
fn struct_with_multiple_fields() {
    let (module, store, context) =
        parse("struct Foo { pub x: int = 1\npub y: string = \"hello\" }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let struct_ = match &stmt.node {
        StatementKind::Struct(s) => s,
        _ => panic!("expected Struct"),
    };
    assert_eq!(struct_.fields.len(), 2);
    assert_eq!(struct_.fields[0].value.name.as_str(), "x");
    assert_eq!(struct_.fields[1].value.name.as_str(), "y");
}

#[test]
fn struct_with_public_field() {
    let (module, store, context) = parse("struct Foo { pub mut x: int }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let struct_ = match &stmt.node {
        StatementKind::Struct(s) => s,
        _ => panic!("expected Struct"),
    };
    assert_eq!(struct_.fields.len(), 1);
    assert!(struct_.fields[0].is_public);
    assert_eq!(struct_.fields[0].value.name.as_str(), "x");
    assert_eq!(
        struct_.fields[0].value.modifier,
        soul_utils::TypeModifier::Mut
    );
}

#[test]
fn struct_with_method() {
    let (module, store, context) = parse("struct Foo { bar() {} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let struct_ = match &stmt.node {
        StatementKind::Struct(s) => s,
        _ => panic!("expected Struct"),
    };
    assert!(struct_.fields.is_empty());
    assert_eq!(struct_.statements.len(), 1);
    let func_id = match &store.statements[struct_.statements[0]].node {
        StatementKind::Function(id) => *id,
        _ => panic!("expected Function in struct body"),
    };
    let func = &store.functions[func_id];
    match func {
        ast_model::FunctionKind::Normal(f) => {
            assert_eq!(f.signature.value.name.as_str(), "bar");
        }
        _ => panic!("expected Normal function"),
    }
}

#[test]
fn struct_with_type_alias() {
    let (module, store, context) = parse("struct Foo { type MyInt = int }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let struct_ = match &stmt.node {
        StatementKind::Struct(s) => s,
        _ => panic!("expected Struct"),
    };
    assert!(struct_.fields.is_empty());
    assert_eq!(struct_.statements.len(), 1);
    match &store.statements[struct_.statements[0]].node {
        StatementKind::TypeDef(_) => (),
        other => panic!("expected TypeDef in struct body, got {:?}", other),
    }
}
