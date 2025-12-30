use soul_ast::abstract_syntax_tree::{
    literal::Literal, operator::BinaryOperator, spanned::Spanned, statment::Ident,
};

use crate::{ExpressionId, HirBodyId, HirId, Todo, hir_type::HirType};

pub type Expression = Spanned<ExpressionKind>;

/// Expression kinds in HIR (desugared and resolved).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    /// Conditional expression.
    If(If),
    /// Reference creation (`&expr` or `@expr`).
    Ref(Ref),
    /// Block expression `{ ... }`.
    Block(HirId),
    /// Lambda expression `|params| body`.
    Lambda(Todo),
    /// Pattern matching.
    Match(Match),
    /// For loop (desugared from Soul `while` or `for`).
    For(For),
    /// Binary operation.
    Binary(Binary),
    /// Unary operation.
    Unary(Unary),
    /// Literal value.
    Literal(Literal),
    /// Dereference `*expr`.
    DeRef(ExpressionId),
    /// Resolved variable reference.
    ResolvedVariable(HirId),
    /// Field access `expr.field`.
    FieldAccess(FieldAccess),
    /// Function/method call.
    FunctionCall(FunctionCall),
    /// Struct literal `Type { field: expr, .. }`.
    StructContructor(StructContructor),
}

/// If expression (`if cond { then } else { else }`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct If {
    /// Condition expression.
    pub condition: ExpressionId,
    /// Then branch body.
    pub body: HirBodyId,
    /// Optional else branch.
    pub else_arm: Option<HirId>,
}

/// Reference expression details.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Ref {
    /// Mutable reference flag.
    pub mutable: bool,
    /// Referenced expression.
    pub expression: ExpressionId,
}

/// Pattern match expression.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Match {
    /// Scrutinee expression.
    pub expression: ExpressionId,
    /// Match arms.
    pub arms: Vec<MatchArm>,
}

/// Single match arm.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MatchArm {
    /// Pattern/condition expression.
    pub condition: ExpressionId,
    /// Arm body.
    pub body: HirBodyId,
}

/// For loop expression (desugared iterator loop).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct For {
    /// Loop body.
    pub body: HirBodyId,
    /// Loop variable binding.
    pub element: HirId,
    /// Iterator expression.
    pub iterator: ExpressionId,
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
    pub operator: BinaryOperator,
    /// Operand.
    pub expression: ExpressionId,
}

/// Function/method call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    /// Callee expression.
    pub callee: ExpressionId,
    /// Argument expressions.
    pub arguments: Vec<ExpressionId>,
}

/// Field access `receiver.field`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldAccess {
    /// Field name.
    pub field: Ident,
    /// Receiver expression.
    pub reciever: ExpressionId,
}

/// Static field access `Type::field`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StaticFieldAccess {
    /// Field name.
    pub field: Ident,
    /// Type receiver.
    pub reciever: ExpressionId,
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
