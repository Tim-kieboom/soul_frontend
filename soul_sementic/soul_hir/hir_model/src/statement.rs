use parser_models::scope::NodeId;
use soul_utils::{
    Ident,
    soul_names::TypeModifier,
    span::{NodeMetaData, Spanned},
};

use crate::{ExpressionId, Function, Import, hir_type::HirType, item::Visibility};

pub type Statement = Spanned<StatementKind>;

/// Kinds of statements in HIR (desugared from AST).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    Import(Import),
    /// Assignment to a place (`x = expression`).
    Assign(Assign),
    /// Variable binding/declaration (`modifier x: T = expression`).
    Variable(Box<Variable>),
    /// Expression statement.
    Expression(StatementExpression),
    Function(Function),
}

/// Expression statement (`expression`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatementExpression {
    pub id: NodeId,
    pub expression: ExpressionId,
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
    pub ty: VarTypeKind,
    pub name: Ident,
    pub vis: Visibility,
    pub value: Option<ExpressionId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum VarTypeKind {
    NonInveredType(HirType),
    InveredType(TypeModifier),
}

pub trait StatementHelper {
    fn new_expression(
        expression: StatementExpression,
        meta: NodeMetaData
    ) -> Statement;
    fn new_variable(variable: Variable, meta: NodeMetaData) -> Statement;
    fn new_function(function: Function, meta: NodeMetaData) -> Statement;
    fn new_import(import: Import, meta: NodeMetaData) -> Statement;
    fn new_assign(assign: Assign, meta: NodeMetaData) -> Statement;
}
impl StatementHelper for Statement {
    fn new_variable(variable: Variable, meta: NodeMetaData) -> Statement {
        Statement::with_meta_data(
            StatementKind::Variable(Box::new(variable)),
            meta
        )
    }

    fn new_function(function: Function, meta: NodeMetaData) -> Statement {
        Statement::with_meta_data(StatementKind::Function(function), meta)
    }

    fn new_import(import: Import, meta: NodeMetaData) -> Statement {
        Statement::with_meta_data(StatementKind::Import(import), meta)
    }

    fn new_assign(assign: Assign, meta: NodeMetaData) -> Statement {
        Statement::with_meta_data(StatementKind::Assign(assign), meta)
    }

    fn new_expression(
        expression: StatementExpression,
        meta: NodeMetaData
    ) -> Statement {
        Statement::with_meta_data(StatementKind::Expression(expression), meta)
    }
}
