use hir_model::HirType;
use soul_utils::span::Spanned;
use parser_models::{ast::{BinaryOperator, Literal, UnaryOperator}, scope::NodeId};

use crate::{Body, ExpressionId, Place};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypedExpression {
    pub ty: HirType,
    pub value: Expression,
}

pub type Expression = Spanned<ExpressionKind>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    Literal(Literal),
    Place(Place),
    Binary {
        left: ExpressionId,
        operator: BinaryOperator,
        right: ExpressionId,
    },
    Unary {
        operator: UnaryOperator,
        value: ExpressionId,
    },
    Ref {
        mutable: bool,
        value: ExpressionId,
    },
    Deref {
        value: ExpressionId,
    },
    Index {
        base: ExpressionId,
        index: ExpressionId,
    },
    Array {
        values: Vec<ExpressionId>,
    },
    Call {
        function: NodeId,
        arguments: Vec<ExpressionId>,
    },
    If {
        condition: ExpressionId,
        body: Body,
        else_body: Body,
    },
    While {
        condition: ExpressionId,
        body: Body,
    },
    Block(Body),
}
