use ast_model::statements::{StatementKind, VarPattern};
use soul_utils::fault::Severity;

use crate::tests::{get_statement, parse};

#[test]
fn test_attribute_on_function() {
    let (module, store, context) = parse("#[test]\ntest_add() { x := 1 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    assert_eq!(stmt.meta_data.attributes.len(), 1);
    assert_eq!(stmt.meta_data.attributes[0].name.as_str(), "test");
    assert!(matches!(stmt.node, StatementKind::Function(_)));
}

#[test]
fn test_negated_attribute_on_struct() {
    let (module, store, context) = parse("#[!Copy]\nstruct Foo { pub x: int = 5 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    assert_eq!(stmt.meta_data.attributes.len(), 1);
    assert_eq!(stmt.meta_data.attributes[0].name.as_str(), "!Copy");
    assert!(matches!(stmt.node, StatementKind::Struct(_)));
}

#[test]
fn test_multiple_attributes_stack_at_module_level() {
    let (module, store, context) = parse("#[a]\n#[b]\n#[!c]\ntest_many() {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let names: Vec<&str> = stmt
        .meta_data
        .attributes
        .iter()
        .map(|attr| attr.name.as_str())
        .collect();
    assert_eq!(names, ["a", "b", "!c"]);
    assert!(matches!(stmt.node, StatementKind::Function(_)));
}

#[test]
fn test_attribute_on_struct_field() {
    let (module, store, context) = parse("struct Foo { #[pass] x: int = 5 }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    match &stmt.node {
        StatementKind::Struct(s) => {
            assert_eq!(s.fields.len(), 1);
            let field = &s.fields[0].value;
            assert!(matches!(
                &field.pattern,
                VarPattern::Simple { binding, .. } if binding.ident.as_str() == "x"
            ));
        }
        _ => panic!("expected Struct, got {:?}", stmt.node.variant_name()),
    }
}

#[test]
fn test_unterminated_attribute_errors() {
    let (_, _, context) = parse("#[test");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected error for unterminated attribute",
    );
}

#[test]
fn test_attribute_missing_opening_bracket_errors() {
    let (_, _, context) = parse("#test\nfoo() {}");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected error when '#' isn't followed by '['",
    );
}

#[test]
fn test_attribute_missing_name_errors() {
    let (_, _, context) = parse("#[]\nfoo() {}");
    assert!(
        context.faults.count_severity(Severity::Error) > 0,
        "expected error when an attribute has no name",
    );
}
