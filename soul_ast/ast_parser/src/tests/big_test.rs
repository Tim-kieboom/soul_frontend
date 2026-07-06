use ast_model::{
    expression::{ExpressionKind, FunctionCall, FunctionCalleeKind, MatchPattern, StructConstructor}, literal::Literal, operators::{BinaryOperatorKind, UnaryOperatorKind}, soul_type::{SoulType, Stub}, statements::{Import, ImportItem, ImportKind, StatementKind, TypeDef, VarPattern, Variable},
};
use soul_utils::{TypeModifier, fault::Severity, soul_names::PrimitiveTypes};

use crate::tests::parse;

const CODE: &str = r#"
import soul.core
import (
    soul.io.*
    soul.fmt.{Fmt}
)
/*
    comment and stuff
*/
test() {
    x := 42 // foo
    mut y := 10
    z: int = 20
    mut w: int
    w = 5
    a := -x
    b := !false
    c := &x
    d := &mut x
    e := 1 + 2
    f := 3 * 4
    g := (1 + 2) * 3
    g := 10 / 2
    h := 10 % 3
    i := 1 < 2
    j := 2 <= 2
    k := 3 > 1
    l := 4 >= 4
    m := 1 == 1
    n := 1 != 2
    o := true && false
    p := true || false
    q := null
    r := "hello"
    s := 3.14
    t := 'a'
    u := foo()
    v := add(1, 2)
    field := obj.field
    method := obj.method()
    chain := foo().bar()
    idx := arr[0]
    block_expr := { inner := 99 }
    point := Point { x: 1, y: 2 }
    heap := new(42)
    if true { return }
    match x {
        1 => false 
        _ => true
    }
    return 42
    break
    continue
}

type MyInt = int
type DistinctInt = distinct int
pub const GLOBAL := 100
"#;

#[test]
fn all_kinds() {
    let (module, store, context) = parse(CODE);
    assert_eq!(
        context.faults.count_severity(Severity::Error),
        0,
        "{:#?}",
        context.faults.faults
    );

    let block = &store.blocks[module.global];

    // 5 top-level statements: import, import glob, function, type alias, pub const
    assert_eq!(block.statements.len(), 6, "expected 5 top-level statements");

    // --- statement 0: import -------------------------------------------------
    let stmt0 = &store.statements[block.statements[0]];
    let paths0 = match &stmt0.node {
        StatementKind::Import(Import { paths, .. }) => paths,
        _ => panic!("statement 0: expected Import"),
    };
    assert_eq!(paths0.len(), 1);
    assert_eq!(paths0[0].kind, ImportKind::Module);

    // --- statement 1: import glob --------------------------------------------
    let stmt1 = &store.statements[block.statements[1]];
    let paths1 = match &stmt1.node {
        StatementKind::Import(Import { paths, .. }) => paths,
        _ => panic!("statement 1: expected Import"),
    };
    assert_eq!(paths1.len(), 2);
    assert_eq!(paths1[0].kind, ImportKind::Glob);
    let item = match &paths1[1].kind {
        ImportKind::Items {
            has_this: false,
            this_alias: None,
            items,
        } => &items[0],
        _ => panic!("paths 1: expected Items"),
    };

    match item {
        ImportItem::Normal(ident) => assert!(ident.as_str() == "Fmt"),
        _ => panic!("ImportItem 1: expected Normal"),
    }

    // --- statement 2: function -----------------------------------------------
    let stmt2 = &store.statements[block.statements[2]];
    let func_id = match &stmt2.node {
        StatementKind::Function(id) => *id,
        _ => panic!("statement 2: expected Function"),
    };
    let func = &store.functions[func_id];
    let func_body = match func {
        ast_model::FunctionKind::Normal(f) => f,
        _ => panic!("expected Normal function"),
    };
    let body = &store.blocks[func_body.block];
    assert_eq!(
        body.statements.len(),
        40,
        "expected 40 statements in function body"
    );

    // Spot-check a few body statements
    // 0: x := 42  — variable declaration with init
    let s0 = &store.statements[body.statements[0]];
    let Variable {
        pattern: v0_pat,
        modifier: v0_mod,
        initialize_value: v0_init,
        ..
    } = match &s0.node {
        StatementKind::Variable(v) => v,
        _ => panic!("body[0]: expected Variable"),
    };
    assert!(matches!(v0_pat, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "x"));
    assert_eq!(*v0_mod, TypeModifier::Const);
    assert!(v0_init.is_some());

    // 3: mut w: int  — typed variable with no init
    let s3 = &store.statements[body.statements[3]];
    let Variable {
        pattern: v3_pat,
        modifier: v3_mod,
        ty: v3_ty,
        initialize_value: v3_init,
        ..
    } = match &s3.node {
        StatementKind::Variable(v) => v,
        _ => panic!("body[3]: expected Variable"),
    };
    assert!(matches!(v3_pat, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "w"));
    assert_eq!(*v3_mod, TypeModifier::Mut);
    assert_eq!(
        *v3_ty,
        Some(SoulType::Primitive(
            soul_utils::soul_names::PrimitiveTypes::Int
        ))
    );
    assert!(v3_init.is_none());

    // 4: w = 5  — assignment
    let s4 = &store.statements[body.statements[4]];
    match &s4.node {
        StatementKind::Assignment(_) => (),
        other => panic!("body[4]: expected Assignment, got {:?}", other),
    }

    // 5: a := -x  — unary negation
    let s5 = &store.statements[body.statements[5]];
    let expr5 = match &s5.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[5]: expected Variable"),
    };
    let val5 = &store.expressions[expr5.unwrap()];
    match &val5.node {
        ExpressionKind::Unary(unary) => {
            assert_eq!(unary.operator.value, UnaryOperatorKind::Neg);
        }
        other => panic!("body[5]: expected Unary(Neg), got {:?}", other),
    }

    // 6: b := !false  — unary not
    let s6 = &store.statements[body.statements[6]];
    let expr6 = match &s6.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[6]: expected Variable"),
    };
    let val6 = &store.expressions[expr6.unwrap()];
    match &val6.node {
        ExpressionKind::Unary(unary) => {
            assert_eq!(unary.operator.value, UnaryOperatorKind::Not);
        }
        other => panic!("body[6]: expected Unary(Not), got {:?}", other),
    }

    // 7: c := &x  — mut ref
    let s7 = &store.statements[body.statements[7]];
    let expr7 = match &s7.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[7]: expected Variable"),
    };
    match &store.expressions[expr7.unwrap()].node {
        ExpressionKind::Ref(_) => (),
        other => panic!("body[7]: expected Ref, got {:?}", other),
    }

    // 8: d := @x  — const ref
    let s8 = &store.statements[body.statements[8]];
    let expr8 = match &s8.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[8]: expected Variable"),
    };
    match &store.expressions[expr8.unwrap()].node {
        ExpressionKind::Ref(_) => (),
        other => panic!("body[8]: expected Ref, got {:?}", other),
    }

    // 9: e := 1 + 2  — binary add
    let s9 = &store.statements[body.statements[9]];
    let expr9 = match &s9.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[9]: expected Variable"),
    };
    match &store.expressions[expr9.unwrap()].node {
        ExpressionKind::Binary(bin) => {
            assert_eq!(bin.operator.value, BinaryOperatorKind::Add);
        }
        other => panic!("body[9]: expected Binary(Add), got {:?}", other),
    }

    // 10: f := 3 * 4  — binary mul
    let s10 = &store.statements[body.statements[10]];
    let expr10 = match &s10.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[10]: expected Variable"),
    };
    match &store.expressions[expr10.unwrap()].node {
        ExpressionKind::Binary(bin) => {
            assert_eq!(bin.operator.value, BinaryOperatorKind::Mul);
        }
        other => panic!("body[10]: expected Binary(Mul), got {:?}", other),
    }

    // 11: g := (1 + 2) * 3  — parens with binary mul
    let s11 = &store.statements[body.statements[11]];
    let expr11 = match &s11.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[11]: expected Variable"),
    };
    let val11 = &store.expressions[expr11.unwrap()];
    match &val11.node {
        ExpressionKind::Binary(bin) => {
            assert_eq!(bin.operator.value, BinaryOperatorKind::Mul);
            let left = &store.expressions[bin.left];
            match &left.node {
                ExpressionKind::Binary(inner) => {
                    assert_eq!(inner.operator.value, BinaryOperatorKind::Add);
                }
                other => panic!("body[11].left: expected Binary(Add), got {:?}", other),
            }
        }
        other => panic!("body[11]: expected Binary(Mul), got {:?}", other),
    }

    // 22: q := null  — null literal
    let s22 = &store.statements[body.statements[22]];
    let expr22 = match &s22.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[22]: expected Variable"),
    };
    let val22 = &store.expressions[expr22.unwrap()];
    assert!(matches!(val22.node, ExpressionKind::Null(_)));

    // 23: r := "hello"  — string literal
    let s23 = &store.statements[body.statements[23]];
    let expr23 = match &s23.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[23]: expected Variable"),
    };
    match &store.expressions[expr23.unwrap()].node {
        ExpressionKind::Literal((_, Literal::Str(s))) => assert_eq!(s.as_str(), "hello"),
        other => panic!("body[23]: expected Literal(Str), got {:?}", other),
    }

    // 25: t := 'a'  — char literal
    let s25 = &store.statements[body.statements[25]];
    let expr25 = match &s25.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[25]: expected Variable"),
    };
    match &store.expressions[expr25.unwrap()].node {
        ExpressionKind::Literal((_, Literal::Char(c))) => assert_eq!(*c, 'a'),
        other => panic!("body[25]: expected Literal(Char), got {:?}", other),
    }

    // 26: u := foo()  — function call no args
    let s26 = &store.statements[body.statements[26]];
    let expr26 = match &s26.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[26]: expected Variable"),
    };
    match &store.expressions[expr26.unwrap()].node {
        ExpressionKind::FunctionCall(FunctionCall {
            name,
            callee,
            arguments,
            ..
        }) => {
            assert_eq!(name.as_str(), "foo");
            assert!(callee.is_none());
            assert!(arguments.is_empty());
        }
        other => panic!("body[26]: expected FunctionCall, got {:?}", other),
    }

    // 27: v := add(1, 2)  — function call with args
    let s27 = &store.statements[body.statements[27]];
    let expr27 = match &s27.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[27]: expected Variable"),
    };
    match &store.expressions[expr27.unwrap()].node {
        ExpressionKind::FunctionCall(FunctionCall {
            name, arguments, ..
        }) => {
            assert_eq!(name.as_str(), "add");
            assert_eq!(arguments.len(), 2);
        }
        other => panic!("body[27]: expected FunctionCall, got {:?}", other),
    }

    // 28: field := obj.field  — field access
    let s28 = &store.statements[body.statements[28]];
    let expr28 = match &s28.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[28]: expected Variable"),
    };
    match &store.expressions[expr28.unwrap()].node {
        ExpressionKind::FieldAccess(fa) => {
            assert_eq!(fa.field.as_str(), "field");
        }
        other => panic!("body[28]: expected FieldAccess, got {:?}", other),
    }

    // 29: method := obj.method()  — method call
    let s29 = &store.statements[body.statements[29]];
    let expr29 = match &s29.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[29]: expected Variable"),
    };
    match &store.expressions[expr29.unwrap()].node {
        ExpressionKind::FunctionCall(FunctionCall { name, callee, .. }) => {
            assert_eq!(name.as_str(), "method");
            assert!(callee.is_some());
        }
        other => panic!("body[29]: expected FunctionCall(method), got {:?}", other),
    }

    // 30: chain := foo().bar()  — chained call
    let s30 = &store.statements[body.statements[30]];
    let expr30 = match &s30.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[30]: expected Variable"),
    };
    match &store.expressions[expr30.unwrap()].node {
        ExpressionKind::FunctionCall(FunctionCall { name, callee, .. }) => {
            assert_eq!(name.as_str(), "bar");
            assert!(callee.is_some());
            let value = match callee.as_ref().unwrap().kind {
                FunctionCalleeKind::Type(_) => panic!("should be FunctionCalleeKind::Expression"),
                FunctionCalleeKind::Expression(val) => val,
            };

            let inner = &store.expressions[value];
            match &inner.node {
                ExpressionKind::FunctionCall(FunctionCall {
                    name: inner_name, ..
                }) => {
                    assert_eq!(inner_name.as_str(), "foo");
                }
                other => panic!(
                    "body[30]: expected inner FunctionCall(foo), got {:?}",
                    other
                ),
            }
        }
        other => panic!(
            "body[30]: expected outer FunctionCall(bar), got {:?}",
            other
        ),
    }

    // 31: idx := arr[0]  — index
    let s31 = &store.statements[body.statements[31]];
    let expr31 = match &s31.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[31]: expected Variable"),
    };
    match &store.expressions[expr31.unwrap()].node {
        ExpressionKind::Index(_) => (),
        other => panic!("body[31]: expected Index, got {:?}", other),
    }

    // 32: block_expr := { inner := 99 }  — block expression
    let s32 = &store.statements[body.statements[32]];
    let expr32 = match &s32.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[32]: expected Variable"),
    };
    match &store.expressions[expr32.unwrap()].node {
        ExpressionKind::Block(_) => (),
        other => panic!("body[32]: expected Block, got {:?}", other),
    }

    // 33: point := Point { x: 1, y: 2 }  — struct constructor
    let s33 = &store.statements[body.statements[33]];
    let expr33 = match &s33.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[33]: expected Variable"),
    };
    match &store.expressions[expr33.unwrap()].node {
        ExpressionKind::StructConstructor(StructConstructor { values, .. }) => {
            assert_eq!(values.len(), 2);
            assert_eq!(values[0].0.as_str(), "x");
            assert_eq!(values[1].0.as_str(), "y");
        }
        other => panic!("body[33]: expected StructConstructor, got {:?}", other),
    }

    // 34: heap := new(42)  — new ptr
    let s34 = &store.statements[body.statements[34]];
    let expr34 = match &s34.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[34]: expected Variable"),
    };
    match &store.expressions[expr34.unwrap()].node {
        ExpressionKind::New(_) => (),
        other => panic!("body[34]: expected New, got {:?}", other),
    }

    // 35: if true { return } — if
    let s35 = &store.statements[body.statements[35]];
    let expression = match &s35.node {
        StatementKind::Expression { expression: id, .. } => &store.expressions[*id],
        _ => panic!("body[35]: expected Expression(If)"),
    };

    let if_ = match &expression.node {
        ExpressionKind::If(val) => val,
        _ => panic!("body[35]: expected If"),
    };

    assert_eq!(if_.branch, None);
    assert!(matches!(
        store.expressions[if_.condition].node,
        ExpressionKind::Literal((_, Literal::Bool(true)))
    ));
    let statement_id = store.blocks[if_.block].statements[0];
    match &store.statements[statement_id].node {
        StatementKind::Expression { expression: id, .. } => {
            assert_eq!(store.expressions[*id].node, ExpressionKind::Return(None))
        }
        _ => panic!("body[35].body[0]: expected Expression(Return)"),
    };

    /*
        36:
        match x {
            1 => false
            _ => true
        } - match
    */
    let s36 = &store.statements[body.statements[36]];
    let match_ = match &s36.node {
        StatementKind::Expression { expression, .. } => {
            match &store.expressions[*expression].node {
                ExpressionKind::Match(val) => val,
                other => panic!("body[36]: expected Match, got {:?}", other),
            }
        }
        other => panic!("body[36]: expected Expression(Match), got {:?}", other),
    };

    match &store.expressions[match_.scrutinee].node {
        ExpressionKind::Variable(var) => assert_eq!(var.name.as_str(), "x"),
        other => panic!("body[36].scrutinee: expected Variable, got {:?}", other),
    }

    assert_eq!(match_.arms.len(), 2);
    assert_eq!(
        match_.arms[0].pattern,
        MatchPattern::Literal(Literal::Uint(1))
    );
    assert_eq!(match_.arms[1].pattern, MatchPattern::Wildcard);

    // 37: return 42  — return with value
    let s37 = &store.statements[body.statements[37]];
    match &s37.node {
        StatementKind::Expression { expression, .. } => {
            match &store.expressions[*expression].node {
                ExpressionKind::Return(Some(_)) => (),
                other => panic!("body[37]: expected Return(Some), got {:?}", other),
            }
        }
        other => panic!("body[37]: expected Expression(Return), got {:?}", other),
    }

    // 38: break
    let s38 = &store.statements[body.statements[38]];
    match &s38.node {
        StatementKind::Expression { expression, .. } => {
            assert_eq!(store.expressions[*expression].node, ExpressionKind::Break);
        }
        other => panic!("body[38]: expected Expression(Break), got {:?}", other),
    }

    // 39: continue
    let s39 = &store.statements[body.statements[39]];
    match &s39.node {
        StatementKind::Expression { expression, .. } => {
            assert_eq!(
                store.expressions[*expression].node,
                ExpressionKind::Continue
            );
        }
        other => panic!("body[39]: expected Expression(Continue), got {:?}", other),
    }

    // --- statement 3: type alias --------------------------------------------
    let stmt3 = &store.statements[block.statements[3]];
    match &stmt3.node {
        StatementKind::TypeDef(TypeDef {
            new_type,
            old_type,
            is_distinct,
            ..
        }) => {
            assert!(!is_distinct);
            assert_eq!(*new_type, SoulType::Stub(Stub::new("MyInt".to_string())));
            assert_eq!(*old_type, SoulType::Primitive(PrimitiveTypes::Int));
        }
        other => panic!("statement 3: expected TypeDef, got {:?}", other),
    }

    // --- statement 4: distinct type alias --------------------------------------------
    let stmt4 = &store.statements[block.statements[4]];
    match &stmt4.node {
        StatementKind::TypeDef(TypeDef {
            new_type,
            old_type,
            is_distinct,
            ..
        }) => {
            assert!(is_distinct);
            assert_eq!(
                *new_type,
                SoulType::Stub(Stub::new("DistinctInt".to_string()))
            );
            assert_eq!(*old_type, SoulType::Primitive(PrimitiveTypes::Int));
        }
        other => panic!("statement 3: expected TypeDef, got {:?}", other),
    }

    // --- statement 5: pub const ----------------------------------------------
    let stmt5 = &store.statements[block.statements[5]];
    let Variable {
        pattern: v_pub_pat,
        modifier: v_pub_mod,
        ..
    } = match &stmt5.node {
        StatementKind::Variable(v) => v,
        other => panic!("statement 5: expected Variable, got {:?}", other),
    };
    assert!(matches!(v_pub_pat, VarPattern::Simple { binding, .. } if binding.ident.as_str() == "GLOBAL"));
    assert_eq!(*v_pub_mod, TypeModifier::Const);
}
