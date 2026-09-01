use ast_model::{
    expression::ExpressionKind,
    literal::Literal,
    statements::{StatementKind, VarPattern, Variable},
};
use soul_utils::{TypeModifier, fault::Severity};

use crate::tests::{get_statement, parse};

#[test]
fn associated_constant_at_module_level() {
    let (module, store, context) = parse(r#"MAX :: 42"#);
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
        _ => panic!("expected Variable, got {:?}", stmt.node.variant_name()),
    };
    assert!(
        matches!(pattern, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "MAX")
    );
    assert_eq!(*modifier, TypeModifier::Const);
    assert!(ty.is_none());
    assert!(initialize_value.is_some());

    let init = &store.expressions[initialize_value.unwrap()];
    assert!(matches!(
        init.node,
        ExpressionKind::Literal((_, Literal::Uint(42)))
    ));
}

#[test]
fn associated_constant_inside_struct() {
    let (module, store, context) = parse(
        r#"
struct List {
    LIST_GROW :: 2
    mut len: uint = 0
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
    match &stmt.node {
        StatementKind::Struct(s) => {
            let fields = &s.fields;
            assert_eq!(fields.len(), 2, "{:#?}", fields);
            let var = &fields[0].value;
            assert!(
                matches!(&var.pattern, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "LIST_GROW")
            );
        }
        _ => panic!("expected Struct, got {:?}", stmt.node.variant_name()),
    }
}
