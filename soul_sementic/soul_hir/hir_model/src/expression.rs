use parser_models::{
    ast::{BinaryOperator, Literal, ReturnKind, UnaryOperator},
    scope::NodeId,
};
use soul_utils::{Ident, span::Spanned};

use crate::{BodyId, ExpressionId, GenericDefine, hir_type::HirType};

pub type Expression = Spanned<ExpressionKind>;

/// Expression kinds in HIR (desugared and resolved).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    Default,
    /// Conditional expression.
    If(If),
    /// Reference creation (`&expr` or `@expr`).
    Ref(Ref),
    /// Block expression `{ ... }`.
    Block(BodyId),
    Index(Index),
    /// while loop (desugared from Soul `while` or `for`).
    While(While),
    /// Binary operation.
    Binary(Binary),
    /// Unary operation.
    Unary(Unary),
    /// Literal value.
    Literal(Literal),
    /// Dereference `*expr`.
    DeRef(ExpressionId),
    /// Resolved variable reference.
    ResolvedVariable(NodeId),
    /// Field access `expr.field`.
    FieldAccess(FieldAccess),
    /// Methode call `Type.func()`.
    StaticMethode(StaticMethode),
    /// Field access `Type.field`.
    StaticFieldAccess(StaticFieldAccess),
    /// Function'func()'/method'expr.func()' call.
    FunctionCall(FunctionCall),
    /// Struct literal `Type { field: expr, .. }`.
    StructContructor(StructContructor),
    /// array and tuple
    ExpressionGroup(ExpressionGroup),
    /// `fall` statement (return from first block).
    Fall(ReturnLike),
    /// `break` statement (exits/return enclosing loop).
    Break(ReturnLike),
    /// `return` statement (returns from enclosing function).
    Return(ReturnLike),
    /// `break` statement (continue enclosing loop).
    Continue(ReturnLike),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExpressionGroup {
    /// Array '[expr,expr,expr]'
    Array(Vec<ExpressionId>),
    /// Tuple '(0: expr, 1: expr)' NamedTuple '(name: expr, name: expr)'
    Tuple {
        values: Vec<(Ident, ExpressionId)>,
        insert_defaults: bool,
    },
}

/// ReturnLike statement (`<return|break|fall|continue> value`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReturnLike {
    pub id: NodeId,
    pub kind: ReturnKind,
    pub value: Option<ExpressionId>,
}

/// If expression (`if cond { then } else { else }`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct If {
    /// Condition expression.
    pub condition: ExpressionId,
    /// Then branch body.
    pub body: BodyId,
    /// Optional else branch.
    pub else_arm: Option<Box<IfArm>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum IfArm {
    ElseIf(If),
    Else(BodyId),
}

/// Reference expression details.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Ref {
    /// Mutable reference flag.
    pub mutable: bool,
    /// Referenced expression.
    pub expression: ExpressionId,
}

/// While loop expression (desugared iterator loop).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct While {
    /// Loop body.
    pub body: BodyId,
    /// loop till condition.
    pub condition: Option<ExpressionId>,
}

/// index operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Index {
    /// Left operand.
    pub collection: ExpressionId,
    /// Right operand.
    pub index: ExpressionId,
}

/// Binary operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Binary {
    /// Left operand.
    pub left: ExpressionId,
    /// Binary operator.
    pub operator: BinaryOperator,
    /// Right operand.
    pub right: ExpressionId,
}

/// Unary operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Unary {
    /// Unary operator.
    pub operator: UnaryOperator,
    /// Operand.
    pub expression: ExpressionId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    /// Callee expression.
    pub callee: Option<ExpressionId>,
    pub name: Ident,
    /// Argument expressions.
    pub arguments: Vec<ExpressionId>,
    pub generics: Vec<GenericDefine>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StaticMethode {
    pub name: Ident,
    pub callee: HirType,
    pub arguments: Vec<ExpressionId>,
}

/// Field access `receiver.field`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldAccess {
    /// Field name.
    pub field: Ident,
    /// Receiver expression.
    pub parent: ExpressionId,
}

/// Field access `receiver.field`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StaticFieldAccess {
    /// Field name.
    pub field: Ident,
    /// Receiver expression.
    pub reciever: HirType,
}

/// Struct literal constructor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StructContructor {
    /// Target struct type.
    pub ty: HirType,
    /// insert other fields with the default value.
    pub insert_defaults: bool,
    /// Field initializers (name -> expression).
    pub fields: Vec<(Ident, ExpressionId)>,
}
