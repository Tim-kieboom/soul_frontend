use ast_model::{
    FunctionKind,
    soul_type::{SoulType, Stub},
    statements::{Import, ImportKind, StatementKind, Struct},
};
use soul_utils::{SharedStr, fault::Severity, soul_names::PrimitiveTypes};

use crate::tests::{get_statement, parse};

#[test]
fn use_block_empty() {
    let (module, store, context) = parse("use Foo {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.ty, SoulType::Stub(Stub::new("Foo")));
    assert!(use_block.use_generics.is_empty());
    assert!(use_block.methods.is_empty());
    assert!(use_block.impls.is_empty());
    assert!(use_block.statements.is_empty());
}

#[test]
fn use_block_method() {
    let (module, store, context) = parse("use Foo { bar() {} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.ty, SoulType::Stub(Stub::new("Foo")));
    assert_eq!(use_block.methods.len(), 1);
    assert!(!use_block.methods[0].is_public);
    let func = &store.functions[use_block.methods[0].id];
    let FunctionKind::Normal(f) = func else {
        panic!("expected Normal function");
    };
    assert_eq!(f.signature.value.name.as_str(), "bar");
    assert!(use_block.impls.is_empty());
    assert!(use_block.statements.is_empty());
}

#[test]
fn use_block_pub_method() {
    let (module, store, context) = parse("use Foo { pub bar() {} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.methods.len(), 1);
    assert!(use_block.methods[0].is_public);
    let func = &store.functions[use_block.methods[0].id];
    let FunctionKind::Normal(f) = func else {
        panic!("expected Normal function");
    };
    assert_eq!(f.signature.value.name.as_str(), "bar");
}

#[test]
fn use_block_multiple_methods() {
    let (module, store, context) = parse("use Foo { bar() {}\nbaz() {} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.methods.len(), 2);
    let func0 = &store.functions[use_block.methods[0].id];
    let FunctionKind::Normal(f0) = func0 else {
        panic!("expected Normal function");
    };
    assert_eq!(f0.signature.value.name.as_str(), "bar");
    let func1 = &store.functions[use_block.methods[1].id];
    let FunctionKind::Normal(f1) = func1 else {
        panic!("expected Normal function");
    };
    assert_eq!(f1.signature.value.name.as_str(), "baz");
}

#[test]
fn use_block_impl_block() {
    let (module, store, context) = parse("use Foo { impl Bar { baz() {} } }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.impls.len(), 1);
    assert_eq!(
        use_block.impls[0].impl_trait,
        SoulType::Stub(Stub::new("Bar"))
    );
    assert_eq!(use_block.impls[0].methods.len(), 1);
    let func = &store.functions[use_block.impls[0].methods[0]];
    let FunctionKind::Normal(f) = func else {
        panic!("expected Normal function");
    };
    assert_eq!(f.signature.value.name.as_str(), "baz");
    assert!(use_block.methods.is_empty());
    assert!(use_block.statements.is_empty());
}

#[test]
fn use_block_with_generic_type() {
    let (module, store, context) = parse("use Foo<int> { bar() {} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(
        use_block.ty,
        SoulType::Stub(Stub {
            name: SharedStr::new("Foo"),
            generics: vec![SoulType::Primitive(PrimitiveTypes::Int)]
        })
    );
    assert!(use_block.use_generics.is_empty());
    assert_eq!(use_block.methods.len(), 1);
}

#[test]
fn use_block_struct_inside() {
    let (module, store, context) = parse("use Foo { struct Inner {} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.statements.len(), 1);
    let inner = &store.statements[use_block.statements[0]];
    match &inner.node {
        StatementKind::Struct(Struct { name, .. }) => {
            assert_eq!(name.as_str(), "Inner");
        }
        _ => panic!("expected Struct inside use block"),
    }
    assert!(use_block.methods.is_empty());
    assert!(use_block.impls.is_empty());
}

#[test]
fn use_block_import_inside() {
    let (module, store, context) = parse("use Foo {\nimport bar.baz\n}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.statements.len(), 1);
    let inner = &store.statements[use_block.statements[0]];
    match &inner.node {
        StatementKind::Import(Import { paths, .. }) => {
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].kind, ImportKind::Module);
        }
        _ => panic!("expected Import inside use block"),
    }
    assert!(use_block.methods.is_empty());
    assert!(use_block.impls.is_empty());
}

#[test]
fn use_block_type_alias_inside() {
    let (module, store, context) = parse("use Foo { type MyInt = int }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.statements.len(), 1);
    let inner = &store.statements[use_block.statements[0]];
    match &inner.node {
        StatementKind::TypeDef(_) => (),
        other => panic!("expected TypeDef inside use block, got {:?}", other),
    }
    assert!(use_block.methods.is_empty());
    assert!(use_block.impls.is_empty());
}

#[test]
fn use_block_inline_method() {
    let (module, store, context) = parse("use Foo bar() {}");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.ty, SoulType::Stub(Stub::new("Foo")));
    assert_eq!(use_block.methods.len(), 1);
    assert!(!use_block.methods[0].is_public);
    let func = &store.functions[use_block.methods[0].id];
    let FunctionKind::Normal(f) = func else {
        panic!("expected Normal function");
    };
    assert_eq!(f.signature.value.name.as_str(), "bar");
    assert!(use_block.impls.is_empty());
    assert!(use_block.statements.is_empty());
}

#[test]
fn use_block_mixed_methods_and_impl() {
    let (module, store, context) = parse("use Foo { bar() {}\nimpl Baz { qux() {} }\nbaz() {} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.methods.len(), 2);
    assert_eq!(use_block.impls.len(), 1);
    assert_eq!(
        use_block.impls[0].impl_trait,
        SoulType::Stub(Stub::new("Baz"))
    );
    assert_eq!(use_block.impls[0].methods.len(), 1);
    let func = &store.functions[use_block.impls[0].methods[0]];
    let FunctionKind::Normal(f) = func else {
        panic!("expected Normal function");
    };
    assert_eq!(f.signature.value.name.as_str(), "qux");
}

#[test]
fn use_block_method_with_return_type() {
    let (module, store, context) = parse("use Foo { bar(): int { 42 } }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.methods.len(), 1);
    let func = &store.functions[use_block.methods[0].id];
    let FunctionKind::Normal(f) = func else {
        panic!("expected Normal function");
    };
    assert_eq!(
        f.signature.value.return_type,
        SoulType::Primitive(PrimitiveTypes::Int)
    );
}

#[test]
fn use_block_method_with_params() {
    let (module, store, context) = parse("use Foo { bar(x: int, y: string) {} }");
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let stmt = get_statement(&store, &module, 0);
    let use_block = match &stmt.node {
        StatementKind::UseBlock(b) => b,
        _ => panic!("expected UseBlock"),
    };
    assert_eq!(use_block.methods.len(), 1);
    let func = &store.functions[use_block.methods[0].id];
    let FunctionKind::Normal(f) = func else {
        panic!("expected Normal function");
    };
    assert_eq!(f.signature.value.parameters.len(), 2);
    assert_eq!(f.signature.value.parameters[0].name.as_str(), "x");
    assert_eq!(
        f.signature.value.parameters[0].ty,
        SoulType::Primitive(PrimitiveTypes::Int)
    );
}
