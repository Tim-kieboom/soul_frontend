use ast_model::{
    expression::ExpressionKind,
    soul_type::SoulType,
    statements::{EnumVariant, StatementKind, UnionKind},
};
use soul_utils::{fault::Severity, soul_names::PrimitiveTypes};

use crate::tests::{get_statement, parse};

#[test]
fn enum_empty() {
    let (module, store, context) = parse("enum Foo {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.name.as_str(), "Foo");
    assert!(enum_.variants.is_empty());
    assert!(enum_.impl_type.is_none());
}

#[test]
fn enum_normal_variants() {
    let (module, store, context) = parse("enum Foo { A, B, C }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.variants.len(), 3);
    match &enum_.variants[0] {
        EnumVariant::Normal(name) => assert_eq!(name.as_str(), "A"),
        _ => panic!("expected Normal variant"),
    }
    match &enum_.variants[1] {
        EnumVariant::Normal(name) => assert_eq!(name.as_str(), "B"),
        _ => panic!("expected Normal variant"),
    }
    match &enum_.variants[2] {
        EnumVariant::Normal(name) => assert_eq!(name.as_str(), "C"),
        _ => panic!("expected Normal variant"),
    }
}

#[test]
fn enum_normal_variants_trailing_comma() {
    let (module, store, context) = parse("enum Foo { A, B, }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.variants.len(), 2);
}

#[test]
fn enum_single_variant() {
    let (module, store, context) = parse("enum Foo { Bar }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.variants.len(), 1);
    match &enum_.variants[0] {
        EnumVariant::Normal(name) => assert_eq!(name.as_str(), "Bar"),
        _ => panic!("expected Normal variant"),
    }
}

#[test]
fn enum_assigned_variants() {
    let (module, store, context) = parse("enum Foo { A = 1, B = 2 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.variants.len(), 2);
    match &enum_.variants[0] {
        EnumVariant::Assigned { name, value } => {
            assert_eq!(name.as_str(), "A");
            let expr = &store.expressions[*value];
            match &expr.node {
                ExpressionKind::Literal((_, lit)) => {
                    assert_eq!(*lit, ast_model::literal::Literal::Uint(1))
                }
                _ => panic!("expected Literal expression"),
            }
        }
        _ => panic!("expected Assigned variant"),
    }
    match &enum_.variants[1] {
        EnumVariant::Assigned { name, value } => {
            assert_eq!(name.as_str(), "B");
            let expr = &store.expressions[*value];
            match &expr.node {
                ExpressionKind::Literal((_, lit)) => {
                    assert_eq!(*lit, ast_model::literal::Literal::Uint(2))
                }
                _ => panic!("expected Literal expression"),
            }
        }
        _ => panic!("expected Assigned variant"),
    }
}

#[test]
fn enum_named_union_variants() {
    let (module, store, context) =
        parse("enum Foo { Bar{x: int}, Baz{field: int, condition: bool} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.variants.len(), 2);
    match &enum_.variants[0] {
        EnumVariant::Union(UnionKind::NamedTuple { name, parameters }) => {
            assert_eq!(name.as_str(), "Bar");
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].0.as_str(), "x");
        }
        _ => panic!("expected Union variant"),
    }
    match &enum_.variants[1] {
        EnumVariant::Union(UnionKind::NamedTuple { name, parameters }) => {
            assert_eq!(name.as_str(), "Baz");
            assert_eq!(parameters.len(), 2);
            assert_eq!(parameters[0].0.as_str(), "field");
            assert_eq!(parameters[1].0.as_str(), "condition");
        }
        _ => panic!("expected Union variant"),
    }
}

#[test]
fn enum_tuple_union_variants() {
    let (module, store, context) = parse("enum Foo { Bar(int), Baz(int, bool) }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.variants.len(), 2);
    match &enum_.variants[0] {
        EnumVariant::Union(UnionKind::Tuple { name, parameters }) => {
            assert_eq!(name.as_str(), "Bar");
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0], SoulType::Primitive(PrimitiveTypes::Int));
        }
        _ => panic!("expected Union variant"),
    }
    match &enum_.variants[1] {
        EnumVariant::Union(UnionKind::Tuple { name, parameters }) => {
            assert_eq!(name.as_str(), "Baz");
            assert_eq!(parameters.len(), 2);
            assert_eq!(parameters[0], SoulType::Primitive(PrimitiveTypes::Int));
            assert_eq!(parameters[1], SoulType::Primitive(PrimitiveTypes::Boolean));
        }
        _ => panic!("expected Union variant"),
    }
}

#[test]
fn enum_named_union_variant_single_field() {
    let (module, store, context) = parse("enum Foo { Bar{x: int} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.variants.len(), 1);
    let EnumVariant::Union(UnionKind::NamedTuple { name, parameters }) = &enum_.variants[0] else {
        panic!("expected Union variant");
    };
    assert_eq!(name.as_str(), "Bar");
    assert_eq!(parameters.len(), 1);
}

#[test]
fn enum_with_underlying_type() {
    let (module, store, context) = parse("enum Foo: int { A = 1, B = 2 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    assert_eq!(enum_.variants.len(), 2);
    assert_eq!(
        enum_.impl_type,
        Some(ast_model::soul_type::SoulType::Primitive(
            PrimitiveTypes::Int
        ))
    );
}

// ----------------------------------------------------------------
//  Bad-path: malformed enum declarations
// ----------------------------------------------------------------
#[test]
fn enum_missing_name_is_rejected() {
    let (_, _, context) = parse("enum {}");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when an enum declaration has no name"
    );
}

#[test]
fn enum_variants_missing_comma_is_rejected() {
    let (_, _, context) = parse("enum Foo { A B }");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when variants aren't comma-separated"
    );
}

#[test]
fn enum_assigned_variant_missing_value_is_rejected() {
    let (_, _, context) = parse("enum Foo { A = }");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when an assigned variant has no value"
    );
}

#[test]
fn enum_tuple_union_unclosed_paren_is_rejected() {
    let (_, _, context) = parse("enum Foo { Bar(int }");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error on an unclosed tuple-union parameter list"
    );
}

#[test]
fn enum_named_union_missing_field_type_is_rejected() {
    let (_, _, context) = parse("enum Foo { Bar{x} }");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when a named-union field has no type"
    );
}

#[test]
fn enum_named_union_unclosed_outer_brace_is_rejected() {
    let (_, _, context) = parse("enum Foo { Bar{x: int}");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected an error when the enum body itself is left unclosed"
    );
}

#[test]
fn enum_named_union_variant_field_types() {
    let (module, store, context) = parse("enum Foo { Bar{x: int, y: bool} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let enum_ = match &stmt.node {
        StatementKind::Enum(e) => e,
        _ => panic!("expected Enum"),
    };
    let EnumVariant::Union(UnionKind::NamedTuple { name, parameters }) = &enum_.variants[0] else {
        panic!("expected Union variant");
    };
    assert_eq!(name.as_str(), "Bar");
    assert_eq!(parameters.len(), 2);
    assert_eq!(
        parameters[0].1,
        ast_model::soul_type::SoulType::Primitive(PrimitiveTypes::Int)
    );
    assert_eq!(
        parameters[1].1,
        ast_model::soul_type::SoulType::Primitive(PrimitiveTypes::Boolean)
    );
}
