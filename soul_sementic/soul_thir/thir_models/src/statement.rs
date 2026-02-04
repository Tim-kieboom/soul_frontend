use hir_model::HirType;
use parser_models::scope::NodeId;
use soul_utils::span::Spanned;

use crate::expression::Expression;

pub type Statement = Spanned<StatementKind>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    Variable {
        id: NodeId,
        ty: HirType,
        value: Option<Expression>,
    },

    Assign {
        place: Place,
        value: Expression,
    },

    Expression(Expression),

    Return(Option<Expression>),
    Break(Option<Expression>),
    Fall(Option<Expression>),
    Continue,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Place {
    Local {
        id: NodeId,
        ty: HirType,
    },

    Deref {
        base: Box<Expression>,
        ty: HirType,
    },

    Index {
        base: Box<Expression>,
        index: Box<Expression>,
        ty: HirType,
    },
}
