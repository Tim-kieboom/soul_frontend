use soul_ast::abstract_syntax_tree::{
    FieldVisibility, Visibility, spanned::Spanned, statment::Ident,
};

use crate::{ExpressionId, HirId, hir_type::HirType};

pub type Statement = Spanned<StatementKind>;

/// Kinds of statements in HIR (desugared from AST).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    /// Assignment to a place (`x = expression`).
    Assign(Assign),
    /// Variable binding/declaration (`modifier x: T = expression`).
    Variable(Box<Variable>),
    Binding(Binding),
    /// Expression statement.
    Expression(StatementExpression),
    /// `fall` statement (return from first block).
    Fall(ReturnLike),
    /// `break` statement (exits/return enclosing loop).
    Break(ReturnLike),
    /// `return` statement (returns from enclosing function).
    Return(ReturnLike),
    Field(Field),
}
impl StatementKind {
    pub fn get_id(&self) -> HirId {
        match self {
            StatementKind::Field(field) => field.id,
            StatementKind::Assign(assign) => assign.id,
            StatementKind::Binding(binding) => binding.id,
            StatementKind::Variable(variable) => variable.id,
            StatementKind::Fall(return_like) => return_like.id,
            StatementKind::Break(return_like) => return_like.id,
            StatementKind::Return(return_like) => return_like.id,
            StatementKind::Expression(expression) => expression.id,
        }
    }
    pub fn get_variant_name(&self) -> &'static str {
        match self {
            StatementKind::Fall(_) => "fall",
            StatementKind::Break(_) => "break",
            StatementKind::Field(_) => "Field",
            StatementKind::Assign(_) => "Assign",
            StatementKind::Return(_) => "return",
            StatementKind::Binding(_) => "Binding",
            StatementKind::Variable(_) => "Variable",
            StatementKind::Expression(_) => "Expression",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Binding {
    pub id: HirId,
    pub ty: Option<HirType>,
    pub variables: Vec<(Ident, ExpressionId)>,
}

/// Expression statement (`expression`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatementExpression {
    pub id: HirId,
    pub expression: ExpressionId,
}

/// ReturnLike statement (`<return|break|fall> value`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReturnLike {
    pub id: HirId,
    pub value: Option<ExpressionId>,
}

/// Assignment statement (`left = right`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Assign {
    pub id: HirId,
    pub left: ExpressionId,
    pub right: ExpressionId,
}

/// Variable declaration/binding in HIR (`ty.modifier name: ty = value`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub id: HirId,
    pub ty: HirType,
    pub name: Ident,
    pub vis: Visibility,
    pub value: Option<ExpressionId>,
}

/// Field declaration/binding in HIR (`ty.modifier name: ty access = value`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub id: HirId,
    pub name: Ident,
    pub ty: HirType,
    pub value: Option<ExpressionId>,
    pub access: FieldVisibility,
}
