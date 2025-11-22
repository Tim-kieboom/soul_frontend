use crate::abstract_syntax_tree::{expression::BoxExpression, spanned::Spanned};

/// A unary operator wrapped with source location information.
pub type UnaryOperator = Spanned<UnaryOperatorKind>;
/// A binary operator wrapped with source location information.
pub type BinaryOperator = Spanned<BinaryOperatorKind>;

/// A unary operation expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Unary {
    /// The unary operator.
    pub operator: UnaryOperator,
    /// The operand expression.
    pub expression: BoxExpression,
}

/// A binary operation expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Binary {
    /// The left-hand side expression.
    pub left: BoxExpression,
    /// The binary operator.
    pub operator: BinaryOperator,
    /// The right-hand side expression.
    pub right: BoxExpression,
}

/// The kind of unary operator.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum UnaryOperatorKind {
    Invalid,
    Neg, // -
    Not, // !
    Increment{before_var: bool}, // ++
    Decrement{before_var: bool}, // --
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BinaryOperatorKind {
    Invalid,
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Log, // log
    Pow, // **
    Root, // </ 
    Mod, // % 

    BitAnd, // &
    BitOr, // |
    BitXor, // |
    
    LogAnd, // &&
    LogOr, // ||
    Eq, // ==
    NotEq, // !=
    Lt, // <
    Gt, // >
    Le, // <=
    Ge, // >=

    /// Range operator (`..`).
    Range,
    /// Type check operator (`typeof`).
    TypeOf,
}