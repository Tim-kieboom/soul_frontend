mod thir_ids;
mod expression;
pub use thir_ids::*;
pub use expression::*;

use hir_model::HirType;
use parser_models::{scope::NodeId};
use soul_utils::{sementic_level::SementicFault, span::Spanned, vec_map::VecMap};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThirResponse {
    pub tree: ThirTree,
    pub faults: Vec<SementicFault>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThirTree {
    pub items: VecMap<ItemId, Item>,
    pub global_expressions: VecMap<ExpressionId, TypedExpression>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Item {
    Function(Function),
    Global(Global),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Global {
    pub owner: NodeId,
    pub local: LocalId,
    pub value: Option<ExpressionId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    pub owner: NodeId,

    pub body: Body,
    pub locals: VecMap<LocalId, Local>,
    pub expressions: VecMap<ExpressionId, TypedExpression>,
}

pub type Local = Spanned<InnerLocal>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InnerLocal {
    pub ty: HirType,
    pub kind: LocalKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LocalKind {
    Parameter,
    Variable,
    Temporary,
}

pub type Body = Spanned<BodyKind>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BodyKind {
    pub statements: Vec<Statement>,
    pub tail: Option<ExpressionId>,
}

pub type Statement = Spanned<StatementKind>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    Assign {
        place: Place,
        value: ExpressionId,
    },
    Variable {
        local: LocalId,
        value: Option<ExpressionId>,
    },
    Block(Body),
    Expression(ExpressionId),
    Continue,
    Fall(Option<ExpressionId>),
    Break(Option<ExpressionId>),
    Return(Option<ExpressionId>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Place {
    Local {
        id: NodeId,
        local: LocalId,
    },
    Deref(ExpressionId),
    Index {
        base: ExpressionId,
        index: ExpressionId,
    },
}

