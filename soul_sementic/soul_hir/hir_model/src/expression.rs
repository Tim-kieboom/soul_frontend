use parser_models::{
    ast::{BinaryOperator, Literal, ReturnKind, UnaryOperator},
    scope::NodeId,
};
use soul_utils::{Ident, span::{Span, Spanned}};

use crate::{BodyId, ExpressionId, hir_type::HirType};

pub type Expression = Spanned<ExpressionKind>;

/// Expression kinds in HIR (desugared and resolved).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    Null,
    Default,
    /// Conditional expression.
    If(If),
    /// Reference creation (`&expr` or `@expr`).
    Ref(Ref),
    /// Block expression `{ ... }`.
    Block(BodyId),
    Index(Index),
    Array(Array),
    /// while loop (desugared from Soul `while` or `for`).
    While(While),
    /// Binary operation.
    Binary(Binary),
    /// Unary operation.
    Unary(Unary),
    /// Literal value.
    Literal(Literal),
    /// Dereference `*expr`.
    DeRef(DeRef),
    /// Resolved variable reference.
    ResolvedVariable(NodeId),
    /// Function'func()'/method'expr.func()' call.
    FunctionCall(FunctionCall),
    /// `fall` statement (return from first block).
    Fall(ReturnLike),
    /// `break` statement (exits/return enclosing loop).
    Break(ReturnLike),
    /// `return` statement (returns from enclosing function).
    Return(ReturnLike),
    /// `break` statement (continue enclosing loop).
    Continue(NodeId),
    AsCastType(AsCastType),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeRef {
    pub id: NodeId,
    pub inner: ExpressionId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AsCastType {
    pub id: NodeId,
    pub left: ExpressionId,
    pub cast_type: HirType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Array {
    pub id: NodeId,
    pub element_type: Option<HirType>,
    pub values: Vec<(ExpressionId, Span)>,
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
    pub id: NodeId, 
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
    pub callee: Option<Spanned<ExpressionId>>,
    pub name: Ident,
    /// Argument expressions.
    pub arguments: Vec<Spanned<ExpressionId>>,
    pub resolved: NodeId,
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
