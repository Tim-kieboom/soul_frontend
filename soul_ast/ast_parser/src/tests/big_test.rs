use ast_model::{expression::{ExpressionKind, FunctionCall, StructConstructor}, literal::Literal, operators::{BinaryOperatorKind, UnaryOperatorKind}, soul_type::SoulType, statements::{Import, ImportKind, StatementKind, Variable}};
use soul_utils::TypeModifier;

use crate::tests::parse;

#[test]
fn all_kinds() {
    let code = r#"
import soul.core
import soul.io.*

fn test() {
x := 42
mut y := 10
z: int = 20
mut w: int
w = 5
a := -x
b := !false
c := &x
d := @x
e := 1 + 2
f := 3 * 4
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
match x { _ => true }
return 42
break
continue
}
type MyInt = int
pub const GLOBAL := 100
"#;
    let (module, store, _) = parse(code);
    let block = &store.blocks[module.global];

    // 5 top-level statements: import, import glob, function, type alias, pub const
    assert_eq!(block.statements.len(), 5, "expected 5 top-level statements");

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
    assert_eq!(paths1.len(), 1);
    assert_eq!(paths1[0].kind, ImportKind::Glob);

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
        39,
        "expected 39 statements in function body"
    );

    // Spot-check a few body statements
    // 0: x := 42  — variable declaration with init
    let s0 = &store.statements[body.statements[0]];
    let Variable { name: v0_name, modifier: v0_mod, initialize_value: v0_init, .. } = match &s0.node {
        StatementKind::Variable(v) => v,
        _ => panic!("body[0]: expected Variable"),
    };
    assert_eq!(v0_name.as_str(), "x");
    assert_eq!(*v0_mod, TypeModifier::Const);
    assert!(v0_init.is_some());

    // 3: mut w: int  — typed variable with no init
    let s3 = &store.statements[body.statements[3]];
    let Variable { name: v3_name, modifier: v3_mod, ty: v3_ty, initialize_value: v3_init, .. } = match &s3.node {
        StatementKind::Variable(v) => v,
        _ => panic!("body[3]: expected Variable"),
    };
    assert_eq!(v3_name.as_str(), "w");
    assert_eq!(*v3_mod, TypeModifier::Mut);
    assert_eq!(*v3_ty, Some(SoulType::Primitive(soul_utils::soul_names::PrimitiveTypes::Int)));
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

    // 21: q := null  — null literal
    let s21 = &store.statements[body.statements[21]];
    let expr21 = match &s21.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[21]: expected Variable"),
    };
    let val21 = &store.expressions[expr21.unwrap()];
    assert_eq!(val21.node, ExpressionKind::Null(None));

    // 22: r := "hello"  — string literal
    let s22 = &store.statements[body.statements[22]];
    let expr22 = match &s22.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[22]: expected Variable"),
    };
    match &store.expressions[expr22.unwrap()].node {
        ExpressionKind::Literal((_, Literal::Str(s))) => assert_eq!(s.as_str(), "hello"),
        other => panic!("body[22]: expected Literal(Str), got {:?}", other),
    }

    // 24: t := 'a'  — char literal
    let s24 = &store.statements[body.statements[24]];
    let expr24 = match &s24.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[24]: expected Variable"),
    };
    match &store.expressions[expr24.unwrap()].node {
        ExpressionKind::Literal((_, Literal::Char(c))) => assert_eq!(*c, 'a'),
        other => panic!("body[24]: expected Literal(Char), got {:?}", other),
    }

    // 25: u := foo()  — function call no args
    let s25 = &store.statements[body.statements[25]];
    let expr25 = match &s25.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[25]: expected Variable"),
    };
    match &store.expressions[expr25.unwrap()].node {
        ExpressionKind::FunctionCall(FunctionCall { name, callee, arguments, .. }) => {
            assert_eq!(name.as_str(), "foo");
            assert!(callee.is_none());
            assert!(arguments.is_empty());
        }
        other => panic!("body[25]: expected FunctionCall, got {:?}", other),
    }

    // 26: v := add(1, 2)  — function call with args
    let s26 = &store.statements[body.statements[26]];
    let expr26 = match &s26.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[26]: expected Variable"),
    };
    match &store.expressions[expr26.unwrap()].node {
        ExpressionKind::FunctionCall(FunctionCall { name, arguments, .. }) => {
            assert_eq!(name.as_str(), "add");
            assert_eq!(arguments.len(), 2);
        }
        other => panic!("body[26]: expected FunctionCall, got {:?}", other),
    }

    // 27: field := obj.field  — field access
    let s27 = &store.statements[body.statements[27]];
    let expr27 = match &s27.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[27]: expected Variable"),
    };
    match &store.expressions[expr27.unwrap()].node {
        ExpressionKind::FieldAccess(fa) => {
            assert_eq!(fa.field.as_str(), "field");
        }
        other => panic!("body[27]: expected FieldAccess, got {:?}", other),
    }

    // 28: method := obj.method()  — method call
    let s28 = &store.statements[body.statements[28]];
    let expr28 = match &s28.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[28]: expected Variable"),
    };
    match &store.expressions[expr28.unwrap()].node {
        ExpressionKind::FunctionCall(FunctionCall { name, callee, .. }) => {
            assert_eq!(name.as_str(), "method");
            assert!(callee.is_some());
        }
        other => panic!("body[28]: expected FunctionCall(method), got {:?}", other),
    }

    // 29: chain := foo().bar()  — chained call
    let s29 = &store.statements[body.statements[29]];
    let expr29 = match &s29.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[29]: expected Variable"),
    };
    match &store.expressions[expr29.unwrap()].node {
        ExpressionKind::FunctionCall(FunctionCall { name, callee, .. }) => {
            assert_eq!(name.as_str(), "bar");
            assert!(callee.is_some());
            let inner = &store.expressions[callee.unwrap()];
            match &inner.node {
                ExpressionKind::FunctionCall(FunctionCall { name: inner_name, .. }) => {
                    assert_eq!(inner_name.as_str(), "foo");
                }
                other => panic!("body[29]: expected inner FunctionCall(foo), got {:?}", other),
            }
        }
        other => panic!("body[29]: expected outer FunctionCall(bar), got {:?}", other),
    }

    // 30: idx := arr[0]  — index
    let s30 = &store.statements[body.statements[30]];
    let expr30 = match &s30.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[30]: expected Variable"),
    };
    match &store.expressions[expr30.unwrap()].node {
        ExpressionKind::Index(_) => (),
        other => panic!("body[30]: expected Index, got {:?}", other),
    }

    // 31: block_expr := { inner := 99 }  — block expression
    let s31 = &store.statements[body.statements[31]];
    let expr31 = match &s31.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[31]: expected Variable"),
    };
    match &store.expressions[expr31.unwrap()].node {
        ExpressionKind::Block(_) => (),
        other => panic!("body[31]: expected Block, got {:?}", other),
    }

    // 32: point := Point { x: 1, y: 2 }  — struct constructor
    let s32 = &store.statements[body.statements[32]];
    let expr32 = match &s32.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[32]: expected Variable"),
    };
    match &store.expressions[expr32.unwrap()].node {
        ExpressionKind::StructConstructor(StructConstructor { values, .. }) => {
            assert_eq!(values.len(), 2);
            assert_eq!(values[0].0.as_str(), "x");
            assert_eq!(values[1].0.as_str(), "y");
        }
        other => panic!("body[32]: expected StructConstructor, got {:?}", other),
    }

    // 33: heap := new(42)  — new ptr
    let s33 = &store.statements[body.statements[33]];
    let expr33 = match &s33.node {
        StatementKind::Variable(v) => v.initialize_value,
        _ => panic!("body[33]: expected Variable"),
    };
    match &store.expressions[expr33.unwrap()].node {
        ExpressionKind::New(_) => (),
        other => panic!("body[33]: expected New, got {:?}", other),
    }

    // 36: return 42  — return with value
    let s36 = &store.statements[body.statements[36]];
    match &s36.node {
        StatementKind::Expression { expression, .. } => {
            match &store.expressions[*expression].node {
                ExpressionKind::Return(Some(_)) => (),
                other => panic!("body[36]: expected Return(Some), got {:?}", other),
            }
        }
        other => panic!("body[36]: expected Expression(Return), got {:?}", other),
    }

    // 37: break
    let s37 = &store.statements[body.statements[37]];
    match &s37.node {
        StatementKind::Expression { expression, .. } => {
            assert_eq!(store.expressions[*expression].node, ExpressionKind::Break);
        }
        other => panic!("body[37]: expected Expression(Break), got {:?}", other),
    }

    // 38: continue
    let s38 = &store.statements[body.statements[38]];
    match &s38.node {
        StatementKind::Expression { expression, .. } => {
            assert_eq!(store.expressions[*expression].node, ExpressionKind::Continue);
        }
        other => panic!("body[38]: expected Expression(Continue), got {:?}", other),
    }

    // --- statement 3: type alias --------------------------------------------
    let stmt3 = &store.statements[block.statements[3]];
    match &stmt3.node {
        StatementKind::TypeDef(_) => (),
        other => panic!("statement 3: expected TypeDef, got {:?}", other),
    }

    // --- statement 4: pub const ----------------------------------------------
    let stmt4 = &store.statements[block.statements[4]];
    let Variable { name: v_pub, modifier: v_pub_mod, .. } = match &stmt4.node {
        StatementKind::Variable(v) => v,
        other => panic!("statement 4: expected Variable, got {:?}", other),
    };
    assert_eq!(v_pub.as_str(), "GLOBAL");
    assert_eq!(*v_pub_mod, TypeModifier::Const);
}