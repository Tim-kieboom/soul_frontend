use ast_model::{
    soul_type::SoulType,
    statements::{Function, StatementKind, VarPattern},
};
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
    assert_eq!(struct_.fields[0].value.name().unwrap().as_str(), "x");
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
    assert_eq!(struct_.fields[0].value.name().unwrap().as_str(), "x");
    assert_eq!(struct_.fields[1].value.name().unwrap().as_str(), "y");
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
    assert_eq!(struct_.fields[0].value.name().unwrap().as_str(), "x");
    assert_eq!(
        struct_.fields[0].value.modifier,
        soul_utils::TypeModifier::Mut
    );
}

#[test]
fn struct_with_bare_typed_fields() {
    let (module, store, context) = parse("struct Foo { x: int\nlen: uint }");
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
    assert_eq!(struct_.fields[0].value.name().unwrap().as_str(), "x");
    assert_eq!(struct_.fields[1].value.name().unwrap().as_str(), "len");
    assert!(struct_.fields[0].value.initialize_value.is_none());
    assert!(struct_.fields[1].value.initialize_value.is_none());
}

#[test]
fn bare_typed_variable_declaration() {
    let (module, store, context) = parse("x: int");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let variable = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert!(
        matches!(&variable.pattern, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "x")
    );
    assert!(variable.initialize_value.is_none());
}

#[test]
fn struct_with_expression_bodied_method() {
    let (module, store, context) = parse("struct Foo {\npub len(&this): uint => this.len\n}");
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
    assert_eq!(struct_.statements.len(), 1);
    let func_id = match &store.statements[struct_.statements[0]].node {
        StatementKind::Function(id) => *id,
        _ => panic!("expected Function in struct body"),
    };
    let func = &store.functions[func_id];
    let Function { signature, .. } = match func {
        ast_model::FunctionKind::Normal(f) => f,
        _ => panic!("expected Normal function"),
    };
    assert_eq!(signature.value.name.as_str(), "len");
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

// ----------------------------------------------------------------
//  Bad-path: statements that aren't allowed in a struct body
// ----------------------------------------------------------------
#[test]
fn struct_body_rejects_assignment_statement() {
    let (_, _, context) = parse("struct Foo { x = 5 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        1,
        "{:#?}",
        context.faults.faults
    );
    assert!(
        context
            .faults
            .faults
            .iter()
            .any(|fault| fault.message().contains("can not be used in struct body")),
        "{:#?}",
        context.faults.faults
    );
}

#[test]
fn struct_body_rejects_expression_statement() {
    let (_, _, context) = parse("struct Foo { 5 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        1,
        "{:#?}",
        context.faults.faults
    );
    assert!(
        context
            .faults
            .faults
            .iter()
            .any(|fault| fault.message().contains("can not be used in struct body")),
        "{:#?}",
        context.faults.faults
    );
}

#[test]
fn struct_recovers_after_a_rejected_statement_and_still_parses_later_fields() {
    let (module, store, context) = parse("struct Foo { x = 5\ny: int }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        1,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let struct_ = match &stmt.node {
        StatementKind::Struct(s) => s,
        _ => panic!("expected Struct"),
    };
    assert_eq!(struct_.fields.len(), 1);
    assert_eq!(struct_.fields[0].value.name().unwrap().as_str(), "y");
}

#[test]
fn struct_missing_name_is_rejected() {
    let (_, _, context) = parse("struct { x: int }");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when a struct declaration has no name"
    );
}

#[test]
fn struct_field_missing_type_after_colon_is_rejected() {
    let (_, _, context) = parse("struct Foo { x: }");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when a struct field's type is missing"
    );
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
