use ast_model::{AstStore, Module, statements::StatementKind};
use soul_utils::fault::Severity;

use crate::tests::{get_statement, parse};

fn error_count(source: &str) -> usize {
    let (_, _, context) = parse(source);
    context.faults.count_severity(Severity::Error)
}

fn top_level_statement_kinds(store: &AstStore, module: &Module) -> Vec<&'static str> {
    store.blocks[module.global]
        .statements
        .iter()
        .map(|id| store.statements[*id].node.variant_name())
        .collect()
}

fn function_body_statement_kinds(
    store: &AstStore,
    module: &Module,
    index: usize,
) -> Vec<&'static str> {
    let stmt = get_statement(store, module, index);
    let StatementKind::Function(func_id) = &stmt.node else {
        panic!(
            "expected a Function statement, got {:?}",
            stmt.node.variant_name()
        );
    };
    let ast_model::FunctionKind::Normal(function) = &store.functions[*func_id] else {
        panic!("expected a Normal function");
    };
    store.blocks[function.block]
        .statements
        .iter()
        .map(|id| store.statements[*id].node.variant_name())
        .collect()
}

// ----------------------------------------------------------------
//  Regression: a compound/plain assignment as the last statement of a
//  block must not swallow the block's closing `}` (previously caused
//  `expected: `}` but found: `<end of file>` and unmasked 19 cascading
//  EOF faults on testCompiler.soul).
// ----------------------------------------------------------------
#[test]
fn assignment_as_last_block_statement_keeps_closing_brace() {
    for (source, expected_body_kinds) in [
        ("f() { total += v }", vec!["assignment"]),
        ("f() { x = 5 }", vec!["assignment"]),
        (
            "f() {\n    total := 0\n    total += v\n}",
            vec!["variable", "assignment"],
        ),
        ("f() { for v in values { total += v } }", vec!["expression"]),
    ] {
        assert_eq!(
            error_count(source),
            0,
            "expected no parse errors for `{source}`: {:#?}",
            parse(source).2.faults.faults
        );
        let (module, store, _) = parse(source);
        assert_eq!(
            top_level_statement_kinds(&store, &module),
            vec!["function"],
            "unexpected top-level shape for `{source}`"
        );
        assert_eq!(
            function_body_statement_kinds(&store, &module, 0),
            expected_body_kinds,
            "unexpected body shape for `{source}`"
        );
    }

    let source = "f(x) { x -= 1 }";
    assert_eq!(
        error_count(source),
        0,
        "expected no parse errors for `{source}`: {:#?}",
        parse(source).2.faults.faults
    );
    let (module, store, _) = parse(source);
    assert_eq!(
        top_level_statement_kinds(&store, &module),
        vec!["expression", "expression"]
    );
}

// ----------------------------------------------------------------
//  Regression: `int.parse(value)` and other primitive receiver method
//  calls must parse (Types token as a primary / variable).
// ----------------------------------------------------------------
#[test]
fn primitive_type_receiver_method_call_parses() {
    for source in [
        "f() { x := int.parse(\"42\") }",
        "f() { y := str.trim(\" hi \") }",
    ] {
        assert_eq!(
            error_count(source),
            0,
            "expected no parse errors for `{source}`: {:#?}",
            parse(source).2.faults.faults
        );
        let (module, store, _) = parse(source);
        assert_eq!(
            function_body_statement_kinds(&store, &module, 0),
            vec!["variable"],
            "unexpected body shape for `{source}`"
        );
    }
}

// ----------------------------------------------------------------
//  Regression: compound assignment used inside a `for` body must parse
//  (the assignment path previously consumed the loop body's `}`).
// ----------------------------------------------------------------
#[test]
fn foreach_with_assignment_body() {
    let source = "f() {\n    total := 0\n    for v in values { total += v }\n}";
    assert_eq!(
        error_count(source),
        0,
        "{:#?}",
        parse(source).2.faults.faults
    );
    let (module, store, _) = parse(source);
    assert_eq!(
        function_body_statement_kinds(&store, &module, 0),
        vec!["variable", "expression"]
    );
}

// ----------------------------------------------------------------
//  Regression: `while ... limit N { ... }` (limit keyword) inside an
//  expression must parse.
// ----------------------------------------------------------------
#[test]
fn for_while_with_limit_inline() {
    let source = "f() { result := for counter <= 0 limit 4 { counter -= 1 } }";
    assert_eq!(
        error_count(source),
        0,
        "{:#?}",
        parse(source).2.faults.faults
    );
    let (module, store, _) = parse(source);
    assert_eq!(
        function_body_statement_kinds(&store, &module, 0),
        vec!["variable"]
    );
}

// ----------------------------------------------------------------
//  Regression: generic type in expression position `Res<str>` must not
//  be mis-parsed as a less-than comparison (previously broke `assertEq`.
//  typeof, Res<str>)).
// ----------------------------------------------------------------
#[test]
fn generic_type_in_expression_position() {
    let source = "assertEq(mapped.typeof, Res<str>)";
    assert_eq!(
        error_count(source),
        0,
        "expected no parse errors for `{source}`: {:#?}",
        parse(source).2.faults.faults
    );
    let (module, store, _) = parse(source);
    assert_eq!(
        top_level_statement_kinds(&store, &module),
        vec!["expression"]
    );

    let source = "f() { assertEq(mapped.typeof, Res<str>) }";
    assert_eq!(
        error_count(source),
        0,
        "expected no parse errors for `{source}`: {:#?}",
        parse(source).2.faults.faults
    );
    let (module, store, _) = parse(source);
    assert_eq!(
        function_body_statement_kinds(&store, &module, 0),
        vec!["expression"]
    );
}
