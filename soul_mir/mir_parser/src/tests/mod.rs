use std::path::PathBuf;

use ast_model::{AstStore, AstTree, FunctionKind, declare_store::DeclareStore};
use ast_parser::{ParseInfo, fault::AstErrorKind, parse_module};
use mir_model::{ConstValue, Operand, Rvalue};
use soul_name_resolver::name_resolve;
use soul_tokenizer::to_token_stream;
use soul_utils::{
    FunctionId,
    collections::{crate_store::CrateStore, module_store::ModuleStore},
};

use crate::{
    MirLowerer,
    fault::{MirErrorKind, MirResult},
};

fn resolve_source(source: &str) -> AstTree<AstErrorKind> {
    let mut module_store = ModuleStore::new();
    module_store.insert_root(PathBuf::from("test.soul"));
    let root = module_store.get_root_id();
    let crate_store = CrateStore::new();

    let tokens = to_token_stream(source, root).expect("test source failed to tokenize");

    let mut ast = AstTree::new(root);
    let info = ParseInfo {
        id: root,
        source_folder: PathBuf::from("."),
        crate_source_folder: PathBuf::from("."),
        parent: None,
        modules: &mut module_store,
        context: &mut ast.context,
        forest: &mut ast.crates,
        crate_store: &crate_store,
    };
    parse_module(tokens, "crate".to_string(), info);

    name_resolve(&mut module_store, &mut ast, &crate_store);
    assert_eq!(
        ast.faults().iter().count(),
        0,
        "test source failed to resolve: {:#?}",
        ast.faults()
    );
    ast
}

fn find_function(store: &AstStore, name: &str) -> FunctionId {
    store
        .functions
        .entries()
        .find_map(|(id, kind)| match kind {
            FunctionKind::Normal(function) if function.signature.value.name.as_str() == name => {
                Some(id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no function named `{name}` found"))
}

fn lower_function(
    store: &AstStore,
    declares: &DeclareStore,
    id: FunctionId,
) -> MirResult<mir_model::Function> {
    let mut lowerer = MirLowerer::new(store, declares);
    lowerer.lower_function(id)?;
    Ok(lowerer.functions.into_values().next().unwrap())
}

fn lower_source(source: &str, function_name: &str) -> MirResult<mir_model::Function> {
    let ast = resolve_source(source);
    let function_id = find_function(&ast.crates.store, function_name);
    lower_function(&ast.crates.store, &ast.declares, function_id)
}

fn assert_rejected_matching(
    result: &MirResult<mir_model::Function>,
    predicate: impl Fn(&MirErrorKind) -> bool,
) {
    let Err(fault) = result else {
        panic!("expected lowering to fail, got {:#?}", result.as_ref().ok());
    };
    assert!(
        predicate(fault.kind()),
        "unexpected fault kind: {:?}",
        fault.kind()
    );
    assert!(
        fault.span().is_some(),
        "expected the fault to carry a span, got {fault:#?}"
    );
}

fn assert_rejected_with(result: &MirResult<mir_model::Function>, expected: MirErrorKind) {
    assert_rejected_matching(result, |kind| *kind == expected);
}

#[test]
fn lowers_arithmetic_with_a_variable_and_a_return() {
    let mir = lower_source(
        "add(a: int, b: int): int {\n    c := a + b\n    return c\n}\n",
        "add",
    )
    .expect("expected successful lowering");

    assert_eq!(mir.arg_count, 2);
    assert_eq!(mir.locals.entries().count(), 4);
    assert_eq!(mir.blocks.entries().count(), 1);

    let (_, block) = mir.blocks.entries().next().expect("expected one block");
    assert_eq!(block.statements.len(), 2, "{:#?}", block.statements);
    assert!(matches!(block.terminator, mir_model::Terminator::Return));

    let mir_model::Statement::Assign(_, Rvalue::BinaryOp(op, left, right)) = &block.statements[0]
    else {
        panic!("expected first statement to assign a BinaryOp");
    };
    assert_eq!(*op, ast_model::operators::BinaryOperatorKind::Add);
    assert!(matches!(left, Operand::Copy(_)));
    assert!(matches!(right, Operand::Copy(_)));

    let mir_model::Statement::Assign(place, Rvalue::Use(Operand::Copy(_))) = &block.statements[1]
    else {
        panic!("expected second statement to assign a bare Use(Copy(..))");
    };
    assert_eq!(place.local, mir.return_local);
}

#[test]
fn lowers_a_bare_literal_return() {
    let mir =
        lower_source("answer(): int {\n    return 42\n}\n", "answer").expect("expected success");

    let (_, block) = mir.blocks.entries().next().unwrap();
    assert_eq!(block.statements.len(), 1);
    let mir_model::Statement::Assign(_, Rvalue::Use(Operand::Constant(ConstValue::Uint(42)))) =
        &block.statements[0]
    else {
        panic!(
            "expected a constant assignment, got {:#?}",
            block.statements[0]
        );
    };
}

#[test]
fn missing_return_is_rejected() {
    let result = lower_source("f(): int {\n    x := 1\n}\n", "f");
    assert_rejected_with(&result, MirErrorKind::MissingReturnStatement);
}

#[test]
fn non_primitive_return_type_is_rejected() {
    let result = lower_source("f() {\n    x := 1\n}\n", "f");
    assert_rejected_matching(&result, |kind| {
        matches!(kind, MirErrorKind::NonPrimitiveType { .. })
    });
}

#[test]
fn destructuring_variable_pattern_is_rejected() {
    let result = lower_source(
        "f(): int {\n    (a, b) := get_pair()\n    return a\n}\n",
        "f",
    );
    assert_rejected_with(&result, MirErrorKind::NonSimpleVariablePatternUnsupported);
}

#[test]
fn struct_typed_parameter_is_rejected() {
    let result = lower_source(
        "struct Point { x: int }\nf(p: Point): int {\n    return p.x\n}\n",
        "f",
    );
    assert_rejected_matching(&result, |kind| {
        matches!(kind, MirErrorKind::NonPrimitiveType { .. })
    });
}

#[test]
fn nested_compound_expression_is_lowered_via_a_temporary() {
    let mir = lower_source(
        "f(a: int, b: int, c: int): int {\n    return a + b * c\n}\n",
        "f",
    )
    .expect("expected successful lowering");

    assert_eq!(mir.locals.entries().count(), 5);

    let (_, block) = mir.blocks.entries().next().unwrap();
    assert_eq!(block.statements.len(), 2, "{:#?}", block.statements);

    let mir_model::Statement::Assign(temp_place, Rvalue::BinaryOp(op, _, _)) = &block.statements[0]
    else {
        panic!(
            "expected first statement to assign the nested `b * c` to a temp, got {:#?}",
            block.statements[0]
        );
    };
    assert_eq!(*op, ast_model::operators::BinaryOperatorKind::Mul);

    let mir_model::Statement::Assign(ret_place, Rvalue::BinaryOp(op, _, right)) =
        &block.statements[1]
    else {
        panic!(
            "expected second statement to assign the outer `a + ..` to the return local, got {:#?}",
            block.statements[1]
        );
    };
    assert_eq!(*op, ast_model::operators::BinaryOperatorKind::Add);
    assert_eq!(ret_place.local, mir.return_local);
    assert!(
        matches!(right, Operand::Copy(place) if place.local == temp_place.local),
        "expected the outer expression's right operand to read back the temp from statement 0"
    );

    let temp_decl = mir
        .locals
        .get(temp_place.local)
        .expect("expected the temp to have a local declaration");
    assert_eq!(
        temp_decl.mutability,
        soul_utils::TypeModifier::Immut,
        "a temp holding a runtime-computed sub-expression must not be marked `Comptime`"
    );
}

#[test]
fn function_call_in_body_is_rejected() {
    let result = lower_source(
        "g(): int { return 1 }\nf(): int {\n    return g()\n}\n",
        "f",
    );
    assert_rejected_with(&result, MirErrorKind::UnsupportedOperandExpression);
}

#[test]
fn nested_function_call_operand_is_rejected() {
    let result = lower_source(
        "g(): int { return 1 }\nf(a: int): int {\n    return a + g()\n}\n",
        "f",
    );
    assert_rejected_with(&result, MirErrorKind::UnsupportedOperandExpression);
}

#[test]
fn non_normal_function_is_rejected() {
    let ast = resolve_source(r#"extern "C" printf(fmt: &char): int {}"#);
    let function_id = ast
        .crates
        .store
        .functions
        .entries()
        .next()
        .map(|(id, _)| id)
        .expect("expected one function entry");

    let result = lower_function(&ast.crates.store, &ast.declares, function_id);
    assert_rejected_with(&result, MirErrorKind::SignatureOnlyFunctionHasNoBody);
}

#[test]
fn if_without_else_joins_after_the_then_branch() {
    let mir = lower_source(
        "f(): int {\n    if true {\n        return 1\n    }\n    return 2\n}\n",
        "f",
    )
    .expect("expected successful lowering");

    // entry (switchInt) + then (return 1) + join (return 2)
    assert_eq!(mir.blocks.entries().count(), 3, "{:#?}", mir.blocks);

    let switch_blocks = mir
        .blocks
        .entries()
        .filter(|(_, block)| matches!(block.terminator, mir_model::Terminator::SwitchInt { .. }))
        .count();
    assert_eq!(switch_blocks, 1, "expected exactly one switchInt block");

    let return_blocks = mir
        .blocks
        .entries()
        .filter(|(_, block)| matches!(block.terminator, mir_model::Terminator::Return))
        .count();
    assert_eq!(
        return_blocks, 2,
        "expected both the then-branch and the join block to return"
    );
}

#[test]
fn if_else_where_both_branches_return_has_no_join_block() {
    let mir = lower_source(
        "f(): int {\n    if true {\n        return 1\n    } else {\n        return 2\n    }\n}\n",
        "f",
    )
    .expect("expected successful lowering");

    // entry (switchInt) + then (return 1) + else (return 2) — no join block,
    // since both branches terminate and nothing reaches it.
    assert_eq!(mir.blocks.entries().count(), 3, "{:#?}", mir.blocks);
}

#[test]
fn statement_after_an_if_else_that_always_returns_is_unreachable() {
    let result = lower_source(
        "f(): int {\n    if true {\n        return 1\n    } else {\n        return 2\n    }\n    return 3\n}\n",
        "f",
    );
    assert_rejected_with(&result, MirErrorKind::UnreachableStatement);
}

#[test]
fn statement_after_a_return_is_unreachable() {
    let result = lower_source("f(): int {\n    return 1\n    return 2\n}\n", "f");
    assert_rejected_with(&result, MirErrorKind::UnreachableStatement);
}

#[test]
fn non_bool_if_condition_is_rejected() {
    let result = lower_source(
        "f(): int {\n    if 1 {\n        return 1\n    }\n    return 2\n}\n",
        "f",
    );
    assert_rejected_with(&result, MirErrorKind::UnsupportedConditionExpression);
}

#[test]
fn while_loop_with_break_reaches_the_exit_block() {
    let mir = lower_source(
        "f(): int {\n    for true {\n        break\n    }\n    return 1\n}\n",
        "f",
    )
    .expect("expected successful lowering");

    // entry (goto header) + header (switchInt) + body (goto exit via break) + exit (return)
    assert_eq!(mir.blocks.entries().count(), 4, "{:#?}", mir.blocks);
}

#[test]
fn continue_in_while_body_jumps_back_to_the_header() {
    let mir = lower_source(
        "f(): int {\n    for true {\n        continue\n    }\n    return 1\n}\n",
        "f",
    )
    .expect("expected successful lowering");

    let (header_id, _) = mir
        .blocks
        .entries()
        .find(|(_, block)| matches!(block.terminator, mir_model::Terminator::SwitchInt { .. }))
        .expect("expected a header block with a switchInt terminator");

    let continues_to_header = mir.blocks.entries().any(|(_, block)| {
        matches!(block.terminator, mir_model::Terminator::Goto(target) if target == header_id)
    });
    assert!(
        continues_to_header,
        "expected `continue` to jump back to the loop header, {:#?}",
        mir.blocks
    );
}

#[test]
fn break_nested_inside_an_if_targets_the_enclosing_loops_exit_block() {
    let mir = lower_source(
        "f(): int {\n    for true {\n        if true {\n            break\n        }\n    }\n    return 1\n}\n",
        "f",
    )
    .expect("expected successful lowering");

    let (_, header) = mir
        .blocks
        .entries()
        .find(|(_, block)| matches!(block.terminator, mir_model::Terminator::SwitchInt { .. }))
        .expect("expected a loop header block");
    let mir_model::Terminator::SwitchInt {
        otherwise: exit_id, ..
    } = header.terminator
    else {
        unreachable!("just matched on SwitchInt above")
    };

    let break_targets_exit = mir.blocks.entries().any(|(_, block)| {
        matches!(block.terminator, mir_model::Terminator::Goto(target) if target == exit_id)
    });
    assert!(
        break_targets_exit,
        "expected the nested `break` to jump to the loop's exit block, {:#?}",
        mir.blocks
    );
}

#[test]
fn break_outside_a_loop_is_rejected() {
    let result = lower_source("f(): int {\n    break\n    return 1\n}\n", "f");
    assert_rejected_with(&result, MirErrorKind::BreakOutsideLoop);
}

#[test]
fn continue_outside_a_loop_is_rejected() {
    let result = lower_source("f(): int {\n    continue\n    return 1\n}\n", "f");
    assert_rejected_with(&result, MirErrorKind::ContinueOutsideLoop);
}

#[test]
fn bare_for_loop_without_a_condition_is_rejected() {
    let result = lower_source(
        "f(): int {\n    for {\n        break\n    }\n    return 1\n}\n",
        "f",
    );
    assert_rejected_with(&result, MirErrorKind::UnsupportedLoopCondition);
}
