use crate::{ExpressionId, Place, StatementId, Variable};

/// A HIR statement.
///
/// Statements represent executable units inside a block.
/// Control-flow altering constructs are modeled explicitly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Statement {
    /// Variable declaration.
    Variable(Variable, StatementId),

    /// Assignment to a place.
    Assign(Assign, StatementId),

    /// Standalone expression statement.
    Expression {
        id: StatementId,
        value: ExpressionId,
        ends_semicolon: bool,
    },

    /// Fall-through statement with an optional value.
    Fall(Option<ExpressionId>, StatementId),

    /// Breaks out of the current loop, optionally yielding a value.
    Break(Option<ExpressionId>, StatementId),

    /// Returns from the current function, optionally yielding a value.
    Return(Option<ExpressionId>, StatementId),

    /// Continues execution of the current loop.
    Continue(StatementId),
}
impl Statement {
    pub fn get_id(&self) -> StatementId {
        match self {
            Statement::Fall(_, id)
            | Statement::Continue(id)
            | Statement::Break(_, id)
            | Statement::Assign(_, id)
            | Statement::Return(_, id)
            | Statement::Variable(_, id)
            | Statement::Expression { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Assign {
    pub place: Place,
    pub value: ExpressionId,
}
