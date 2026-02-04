use hir_model::HirType;
use parser_models::{ast::{BinaryOperator, Literal, UnaryOperator}, scope::NodeId};
use soul_utils::span::Spanned;

use crate::{Body, statement::Place};

pub type Expression = Spanned<TypedExpression>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypedExpression {
    pub kind: ExpressionKind,
    pub ty: HirType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    Literal(Literal),

    Place(Place),

    Binary {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },

    Unary {
        operator: UnaryOperator,
        expression: Box<Expression>,
    },

    Ref {
        mutable: bool,
        expression: Box<Expression>,
    },

    Deref(Box<Expression>),

    Index {
        base: Box<Expression>,
        index: Box<Expression>,
    },

    Array(Vec<Expression>),

    Call {
        function: NodeId,
        arguments: Vec<Expression>,
    },

    If {
        condition: Box<Expression>,
        body: Body,
        else_branch: Body,
    },

    While {
        condition: Box<Expression>,
        body: Body,
    },

    Block(Body), 
}
