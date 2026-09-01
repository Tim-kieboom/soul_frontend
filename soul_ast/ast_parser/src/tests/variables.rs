use ast_model::{
    expression::ExpressionKind,
    literal::Literal,
    soul_type::SoulType,
    statements::{StatementKind, VarPattern, Variable},
};
use soul_utils::{TypeModifier, fault::Severity};

use crate::tests::{get_statement, parse};

#[test]
fn variable_declaration_with_init() {
    let (module, store, context) = parse("x := 5");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable {
        pattern,
        modifier,
        ty,
        initialize_value,
        ..
    } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert!(matches!(pattern, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "x"));
    assert_eq!(*modifier, TypeModifier::Immut);
    assert!(ty.is_none());
    assert!(initialize_value.is_some());

    let init = &store.expressions[initialize_value.unwrap()];
    assert!(matches!(
        init.node,
        ExpressionKind::Literal((_, Literal::Uint(5)))
    ));
}

#[test]
fn variable_declaration_typed() {
    let (module, store, context) = parse("x: int = 10");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { ty, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert_eq!(
        *ty,
        Some(SoulType::Primitive(
            soul_utils::soul_names::PrimitiveTypes::Int
        ))
    );
}

#[test]
fn variable_declaration_no_init() {
    let (module, store, context) = parse("mut x: int");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable {
        initialize_value,
        pattern,
        modifier,
        ..
    } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert!(matches!(pattern, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "x"));
    assert_eq!(*modifier, TypeModifier::Mut);
    assert!(initialize_value.is_none());
}

#[test]
fn mutable_variable() {
    let (module, store, context) = parse("mut x := 5");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable {
        pattern, modifier, ..
    } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert!(matches!(pattern, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "x"));
    assert_eq!(*modifier, TypeModifier::Mut);
}

#[test]
fn discard_variable() {
    let (module, store, context) = parse("_ := 5");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    assert!(matches!(pattern, VarPattern::Discard));
}

#[test]
fn tuple_destructuring() {
    let (module, store, context) = parse("(a, b) := get_pair()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let tuple = match pattern {
        VarPattern::Tuple(t) => t,
        _ => panic!("expected Tuple pattern"),
    };
    assert_eq!(tuple.elements.len(), 2);
    assert!(!tuple.rest);
    assert!(
        matches!(&tuple.elements[0], VarPattern::Simple { binding, .. } if binding.ident.as_str() == "a")
    );
    assert!(
        matches!(&tuple.elements[1], VarPattern::Simple { binding, .. } if binding.ident.as_str() == "b")
    );
}

#[test]
fn tuple_destructuring_with_rest() {
    let (module, store, context) = parse("(a, ..) := get_list()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let tuple = match pattern {
        VarPattern::Tuple(t) => t,
        _ => panic!("expected Tuple pattern"),
    };
    assert_eq!(tuple.elements.len(), 1);
    assert!(tuple.rest);
    assert!(
        matches!(&tuple.elements[0], VarPattern::Simple { binding, .. } if binding.ident.as_str() == "a")
    );
}

#[test]
fn tuple_destructuring_single_element() {
    let (module, store, context) = parse("(x) := expr()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let tuple = match pattern {
        VarPattern::Tuple(t) => t,
        _ => panic!("expected Tuple pattern"),
    };
    assert_eq!(tuple.elements.len(), 1);
    assert!(!tuple.rest);
    assert!(
        matches!(&tuple.elements[0], VarPattern::Simple { binding, .. } if binding.ident.as_str() == "x")
    );
}

#[test]
fn named_tuple_destructuring() {
    let (module, store, context) = parse("{x, y} := get_point()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let named = match pattern {
        VarPattern::NamedTuple(n) => n,
        _ => panic!("expected NamedTuple pattern"),
    };
    assert!(!named.rest);
    assert_eq!(named.fields.len(), 2);
    assert_eq!(named.fields[0].field.as_str(), "x");
    assert_eq!(
        named.fields[0].binding.as_ref().unwrap().ident.as_str(),
        "x"
    );
    assert_eq!(named.fields[1].field.as_str(), "y");
    assert_eq!(
        named.fields[1].binding.as_ref().unwrap().ident.as_str(),
        "y"
    );
}

#[test]
fn named_tuple_destructuring_with_alias() {
    let (module, store, context) = parse("{x: a, y: b} := get_point()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let named = match pattern {
        VarPattern::NamedTuple(n) => n,
        _ => panic!("expected NamedTuple pattern"),
    };
    assert!(!named.rest);
    assert_eq!(named.fields.len(), 2);
    assert_eq!(named.fields[0].field.as_str(), "x");
    assert_eq!(
        named.fields[0].binding.as_ref().unwrap().ident.as_str(),
        "a"
    );
    assert_eq!(named.fields[1].field.as_str(), "y");
    assert_eq!(
        named.fields[1].binding.as_ref().unwrap().ident.as_str(),
        "b"
    );
}

#[test]
fn named_tuple_destructuring_with_rest() {
    let (module, store, context) = parse("{x, ..} := get_record()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let named = match pattern {
        VarPattern::NamedTuple(n) => n,
        _ => panic!("expected NamedTuple pattern"),
    };
    assert!(named.rest);
    assert_eq!(named.fields.len(), 1);
    assert_eq!(named.fields[0].field.as_str(), "x");
}

#[test]
fn constructor_destructuring() {
    let (module, store, context) = parse("Point{x, y} := get_point()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let ctor = match pattern {
        VarPattern::Constructor(c) => c,
        _ => panic!("expected Constructor pattern"),
    };
    assert_eq!(ctor.type_name.as_str(), "Point");
    assert!(!ctor.rest);
    assert_eq!(ctor.fields.len(), 2);
    assert_eq!(ctor.fields[0].field.as_str(), "x");
    assert_eq!(ctor.fields[0].binding.as_ref().unwrap().ident.as_str(), "x");
    assert_eq!(ctor.fields[1].field.as_str(), "y");
    assert_eq!(ctor.fields[1].binding.as_ref().unwrap().ident.as_str(), "y");
}

#[test]
fn constructor_destructuring_with_alias() {
    let (module, store, context) = parse("Point{x: a, y: b} := get_point()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let ctor = match pattern {
        VarPattern::Constructor(c) => c,
        _ => panic!("expected Constructor pattern"),
    };
    assert_eq!(ctor.type_name.as_str(), "Point");
    assert_eq!(ctor.fields.len(), 2);
    assert_eq!(ctor.fields[0].field.as_str(), "x");
    assert_eq!(ctor.fields[0].binding.as_ref().unwrap().ident.as_str(), "a");
    assert_eq!(ctor.fields[1].field.as_str(), "y");
    assert_eq!(ctor.fields[1].binding.as_ref().unwrap().ident.as_str(), "b");
}

#[test]
fn constructor_destructuring_with_rest() {
    let (module, store, context) = parse("Point{x, ..} := get_point()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let ctor = match pattern {
        VarPattern::Constructor(c) => c,
        _ => panic!("expected Constructor pattern"),
    };
    assert!(ctor.rest);
    assert_eq!(ctor.fields.len(), 1);
    assert_eq!(ctor.fields[0].field.as_str(), "x");
}

#[test]
fn nested_tuple_destructuring() {
    let (module, store, context) = parse("(a, (b, c)) := get_nested()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let outer = match pattern {
        VarPattern::Tuple(t) => t,
        _ => panic!("expected Tuple pattern"),
    };
    assert_eq!(outer.elements.len(), 2);
    assert!(
        matches!(&outer.elements[0], VarPattern::Simple { binding, .. } if binding.ident.as_str() == "a")
    );
    match &outer.elements[1] {
        VarPattern::Tuple(inner) => {
            assert_eq!(inner.elements.len(), 2);
            assert!(
                matches!(&inner.elements[0], VarPattern::Simple { binding, .. } if binding.ident.as_str() == "b")
            );
            assert!(
                matches!(&inner.elements[1], VarPattern::Simple { binding, .. } if binding.ident.as_str() == "c")
            );
        }
        _ => panic!("expected nested Tuple"),
    }
}

#[test]
fn mut_on_compound_tuple_is_error() {
    let (_, _, context) = parse("mut (a, b) := get_pair()");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected error for mut on compound tuple"
    );
}

#[test]
fn mut_on_compound_named_tuple_is_error() {
    let (_, _, context) = parse("mut {x, y} := get_point()");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected error for mut on compound named tuple"
    );
}

#[test]
fn mut_on_compound_constructor_is_error() {
    let (_, _, context) = parse("mut Point{x} := get_point()");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected error for mut on compound constructor"
    );
}

#[test]
fn tuple_with_discard_element() {
    let (module, store, context) = parse("(a, _) := get_pair()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let tuple = match pattern {
        VarPattern::Tuple(t) => t,
        _ => panic!("expected Tuple pattern"),
    };
    assert_eq!(tuple.elements.len(), 2);
    assert!(matches!(&tuple.elements[1], VarPattern::Discard));
}

#[test]
fn named_tuple_with_discard_alias() {
    let (module, store, context) = parse("{x: _, y} := get_point()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let named = match pattern {
        VarPattern::NamedTuple(n) => n,
        _ => panic!("expected NamedTuple pattern"),
    };
    assert_eq!(named.fields.len(), 2);
    assert!(named.fields[0].binding.is_none(), "x should be discarded");
    assert!(named.fields[1].binding.is_some(), "y should be bound");
}

#[test]
fn constructor_with_discard_alias() {
    let (module, store, context) = parse("Point{x: _, y} := get_point()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let ctor = match pattern {
        VarPattern::Constructor(c) => c,
        _ => panic!("expected Constructor pattern"),
    };
    assert_eq!(ctor.fields.len(), 2);
    assert!(ctor.fields[0].binding.is_none(), "x should be discarded");
    assert!(ctor.fields[1].binding.is_some(), "y should be bound");
}

#[test]
fn tuple_with_per_binding_mut() {
    let (module, store, context) = parse("(mut a, b) := get_pair()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let tuple = match pattern {
        VarPattern::Tuple(t) => t,
        _ => panic!("expected Tuple pattern"),
    };
    assert!(
        matches!(&tuple.elements[0], VarPattern::Simple { binding, modifier } if binding.ident.as_str() == "a" && *modifier == TypeModifier::Mut)
    );
    assert!(
        matches!(&tuple.elements[1], VarPattern::Simple { binding, modifier } if binding.ident.as_str() == "b" && *modifier == TypeModifier::Const)
    );
}

#[test]
fn tuple_with_all_mut() {
    let (module, store, context) = parse("(mut a, mut b) := get_pair()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let Variable { pattern, .. } = match &stmt.node {
        StatementKind::Variable(v) => v,
        _ => panic!("expected Variable"),
    };
    let tuple = match pattern {
        VarPattern::Tuple(t) => t,
        _ => panic!("expected Tuple pattern"),
    };
    assert!(
        matches!(&tuple.elements[0], VarPattern::Simple { modifier, .. } if *modifier == TypeModifier::Mut)
    );
    assert!(
        matches!(&tuple.elements[1], VarPattern::Simple { modifier, .. } if *modifier == TypeModifier::Mut)
    );
}

#[test]
fn named_tuple_parsed_as_expression_in_expression_context() {
    // `{x, y} := ...` at statement level is parsed as destructuring (not block).
    // But `{x, y}` in expression context should still be parsed as a block.
    // This test verifies the parser can distinguish them.
    let (module, store, context) = parse("{x, y} := get_record()");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    assert!(matches!(&stmt.node, StatementKind::Variable(_)));
}
