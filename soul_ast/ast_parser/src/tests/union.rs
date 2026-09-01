use ast_model::{
    soul_type::SoulType,
    statements::{EnumVariant, StatementKind, UnionKind},
};
use soul_utils::{fault::Severity, soul_names::PrimitiveTypes};

use crate::tests::{get_statement, parse};

#[test]
fn union_mixed_variants() {
    let (module, store, context) = parse(
        r#"
union Literal {
    None,
    Int(int),
    Str{tag: str, value: str},
}
"#,
    );
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let union_ = match &stmt.node {
        StatementKind::Union(u) => u,
        _ => panic!("expected Union, got {:?}", stmt.node.variant_name()),
    };
    assert_eq!(union_.name.as_str(), "Literal");
    assert!(union_.impl_type.is_none());
    assert_eq!(union_.variants.len(), 3);

    match &union_.variants[0] {
        EnumVariant::Normal(name) => assert_eq!(name.as_str(), "None"),
        _ => panic!("expected Normal variant for None"),
    }

    match &union_.variants[1] {
        EnumVariant::Union(UnionKind::Tuple { name, parameters }) => {
            assert_eq!(name.as_str(), "Int");
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0], SoulType::Primitive(PrimitiveTypes::Int));
        }
        other => panic!("expected Tuple union variant for Int, got {:?}", other),
    }

    match &union_.variants[2] {
        EnumVariant::Union(UnionKind::NamedTuple { name, parameters }) => {
            assert_eq!(name.as_str(), "Str");
            assert_eq!(parameters.len(), 2);
            assert_eq!(parameters[0].0.as_str(), "tag");
            assert_eq!(parameters[0].1, SoulType::String);
            assert_eq!(parameters[1].0.as_str(), "value");
            assert_eq!(parameters[1].1, SoulType::String);
        }
        other => panic!("expected NamedTuple union variant for Str, got {:?}", other),
    }
}

#[test]
fn union_empty() {
    let (module, store, context) = parse("union Foo {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let union_ = match &stmt.node {
        StatementKind::Union(u) => u,
        _ => panic!("expected Union"),
    };
    assert_eq!(union_.name.as_str(), "Foo");
    assert!(union_.variants.is_empty());
    assert!(union_.impl_type.is_none());
}
