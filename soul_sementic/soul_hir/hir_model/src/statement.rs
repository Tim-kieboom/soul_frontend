use parser_models::scope::NodeId;
use soul_utils::{
    Ident,
    soul_names::TypeModifier,
    span::{Attribute, Span, Spanned},
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
        span: Span,
        attributes: Vec<Attribute>,
    ) -> Statement;
    fn new_variable(variable: Variable, span: Span, attributes: Vec<Attribute>) -> Statement;
    fn new_function(function: Function, span: Span, attributes: Vec<Attribute>) -> Statement;
    fn new_import(import: Import, span: Span, attributes: Vec<Attribute>) -> Statement;
    fn new_assign(assign: Assign, span: Span, attributes: Vec<Attribute>) -> Statement;
}
impl StatementHelper for Statement {
    fn new_variable(variable: Variable, span: Span, attributes: Vec<Attribute>) -> Statement {
        Statement::with_atribute(
            StatementKind::Variable(Box::new(variable)),
            span,
            attributes,
        )
    }

    fn new_function(function: Function, span: Span, attributes: Vec<Attribute>) -> Statement {
        Statement::with_atribute(StatementKind::Function(function), span, attributes)
    }

    fn new_import(import: Import, span: Span, attributes: Vec<Attribute>) -> Statement {
        Statement::with_atribute(StatementKind::Import(import), span, attributes)
    }

    fn new_assign(assign: Assign, span: Span, attributes: Vec<Attribute>) -> Statement {
        Statement::with_atribute(StatementKind::Assign(assign), span, attributes)
    }

    fn new_expression(
        expression: StatementExpression,
        span: Span,
        attributes: Vec<Attribute>,
    ) -> Statement {
        Statement::with_atribute(StatementKind::Expression(expression), span, attributes)
    }
}
