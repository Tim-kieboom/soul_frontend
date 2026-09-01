use soul_utils::fault::Severity;

use crate::tests::parse;

fn error_count(source: &str) -> usize {
    let (_, _, context) = parse(source);
    context.faults.count_severity(Severity::Error)
}

// ----------------------------------------------------------------
//  Regression: a compound/plain assignment as the last statement of a
//  block must not swallow the block's closing `}` (previously caused
//  `expected: `}` but found: `<end of file>` and unmasked 19 cascading
//  EOF faults on testCompiler.soul).
// ----------------------------------------------------------------
#[test]
fn assignment_as_last_block_statement_keeps_closing_brace() {
    for source in [
        "f() { total += v }",
        "f() { x = 5 }",
        "f(x) { x -= 1 }",
        "f() {\n    total := 0\n    total += v\n}",
        "f() { for v in values { total += v } }",
    ] {
        assert_eq!(
            error_count(source),
            0,
            "expected no parse errors for `{source}`: {:#?}",
            parse(source).2.faults.faults
        );
    }
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
}

// ----------------------------------------------------------------
//  Regression: generic type in expression position `Res<str>` must not
//  be mis-parsed as a less-than comparison (previously broke `assertEq`.
//  typeof, Res<str>)).
// ----------------------------------------------------------------
#[test]
fn generic_type_in_expression_position() {
    for source in [
        "assertEq(mapped.typeof, Res<str>)",
        "f() { assertEq(mapped.typeof, Res<str>) }",
    ] {
        assert_eq!(
            error_count(source),
            0,
            "expected no parse errors for `{source}`: {:#?}",
            parse(source).2.faults.faults
        );
    }
}
