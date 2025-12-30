use soul_ast::abstract_syntax_tree::{spanned::Spanned, statment::Ident};

use crate::{ExpressionId, HirId, hir_type::HirType};

pub type Statement = Spanned<StatementKind>;

/// Kinds of statements in HIR (desugared from AST).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    /// Assignment to a place (`x = expression`).
    Assign(Assign),
    /// Variable binding/declaration (`modifier x: T = expression`).
    Variable(Box<Variable>),
    /// Expression statement.
    Expression(ExpressionId),
    /// `fall` statement (return from first block).
    Fall(Option<ExpressionId>),
    /// `break` statement (exits/return enclosing loop).
    Break(Option<ExpressionId>),
    /// `return` statement (returns from enclosing function).
    Return(Option<ExpressionId>),
}

/// Assignment statement (`left = right`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Assign {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

/// Variable declaration/binding in HIR (`ty.modifier name: ty = value`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub id: HirId,
    pub ty: HirType,
    pub name: Ident,
    pub value: Option<ExpressionId>,
}
