use parser_models::{ast::ReturnKind, scope::NodeId};
use soul_utils::{Ident, span::Spanned};

use crate::{ExpressionId, hir_type::HirType, item::Visibility};

pub type Statement = Spanned<StatementKind>;

/// Kinds of statements in HIR (desugared from AST).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    /// Assignment to a place (`x = expression`).
    Assign(Assign),
    /// Variable binding/declaration (`modifier x: T = expression`).
    Variable(Box<Variable>),
    /// Expression statement.
    Expression(StatementExpression),
    /// `fall` statement (return from first block).
    Fall(ReturnLike),
    /// `break` statement (exits/return enclosing loop).
    Break(ReturnLike),
    /// `return` statement (returns from enclosing function).
    Return(ReturnLike),
    Continue(ReturnLike),
}

/// Expression statement (`expression`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatementExpression {
    pub id: NodeId,
    pub expression: ExpressionId,
}

/// ReturnLike statement (`<return|break|fall|continue> value`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReturnLike {
    pub id: NodeId,
    pub kind: ReturnKind,
    pub value: Option<ExpressionId>,
}

/// Assignment statement (`left = right`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Assign {
    pub id: NodeId,
    pub left: ExpressionId,
    pub right: ExpressionId,
}

/// Variable declaration/binding in HIR (`ty.modifier name: ty = value`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub id: NodeId,
    pub ty: HirType,
    pub name: Ident,
    pub vis: Visibility,
    pub value: Option<ExpressionId>,
}
