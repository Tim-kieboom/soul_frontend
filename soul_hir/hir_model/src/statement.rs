use crate::{ExpressionId, Place, Variable};

/// A HIR statement.
///
/// Statements represent executable units inside a block.
/// Control-flow altering constructs are modeled explicitly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Statement {
    /// Variable declaration.
    Variable(Variable),

    /// Assignment to a place.
    Assign(Assign),

    /// Standalone expression statement.
    Expression{value: ExpressionId, ends_semicolon: bool},

    /// Fall-through statement with an optional value.
    Fall(Option<ExpressionId>),

    /// Breaks out of the current loop, optionally yielding a value.
    Break(Option<ExpressionId>),

    /// Returns from the current function, optionally yielding a value.
    Return(Option<ExpressionId>),

    /// Continues execution of the current loop.
    Continue,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Assign {
    pub place: Place,
    pub value: ExpressionId,
}