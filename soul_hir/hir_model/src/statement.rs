use crate::{Expression, Place, Variable};

/// A HIR statement.
///
/// Statements represent executable units inside a block.
/// Control-flow altering constructs are modeled explicitly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Statement {
    /// Variable declaration.
    Variable(Variable),

    /// Assignment to a place.
    Assign {
        place: Place,
        value: Expression,
    },

    /// Standalone expression statement.
    Expression(Expression),

    /// Fall-through statement with an optional value.
    Fall(Option<Expression>),

    /// Breaks out of the current loop, optionally yielding a value.
    Break(Option<Expression>),

    /// Returns from the current function, optionally yielding a value.
    Return(Option<Expression>),

    /// Continues execution of the current loop.
    Continue,
}