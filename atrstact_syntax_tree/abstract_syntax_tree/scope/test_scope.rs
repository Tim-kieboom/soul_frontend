use crate::steps::abstract_syntax_tree::scope::{scope::{ScopeId, TypeSymbol, ValueSymbol}, scope_builder::ScopeBuilder};

#[test]
fn scope_push_pop() {
    let mut sb = ScopeBuilder::new();
    assert_eq!(sb.current, ScopeId::new(0));
    sb.push_scope();
    assert_eq!(sb.current, ScopeId::new(1));
    sb.pop_scope();
    assert_eq!(sb.current, ScopeId::new(0));
}

#[test]
fn insert_and_resolve_value() {
    let mut sb = ScopeBuilder::new();
    sb.insert_value(
        "x".into(),
        ValueSymbol::Variable(todo!()),
    );
    let resolved = sb.resolve_value("x");
    assert!(resolved.is_some());
}

#[test]
fn insert_and_resolve_type() {
    let mut sb = ScopeBuilder::new();
    sb.insert_type("MyStruct".into(), TypeSymbol::Struct("MyStruct".into())).unwrap();
    let resolved = sb.resolve_type("MyStruct");
    assert!(resolved.is_some());
}