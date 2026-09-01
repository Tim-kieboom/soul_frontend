use soul_utils::{define_str_enum, span::Spanned};

use crate::expression::ExpressionId;

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
    pub expression: ExpressionId,
}

/// A binary operation expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Binary {
    /// The left-hand side expression.
    pub left: ExpressionId,
    /// The binary operator.
    pub operator: BinaryOperator,
    /// The right-hand side expression.
    pub right: ExpressionId,
}

define_str_enum! {
    /// The kind of unary operator.
    pub enum UnaryOperatorKind {
        Invalid => "<invalid>", 0,
        /// `-`
        Neg => "-", 8,
        /// `!`
        Not => "!", 8,
    }
}

define_str_enum! {
    /// The kind of binary operator.
    pub enum BinaryOperatorKind {
        Invalid => "<invalid>", 0,
        /// `+`
        Add => "+", 5,
        /// `-`
        Sub => "-", 5,
        /// `*`
        Mul => "*", 6,
        /// `/`
        Div => "/", 6,
        /// `log`
        Log => "log", 7,
        /// `**`
        Pow => "**", 7,
        /// `</`
        Root => "</", 7,
        /// `%`
        Mod => "%", 6,

        /// `&`
        BitAnd => "&", 1,
        /// `|`
        BitOr => "|", 1,
        /// `^`
        BitXor => "^", 2,

        /// `&&`
        LogAnd => "&&", 0,
        /// `||`
        LogOr => "||", 0,
        /// `==`
        Eq => "==", 3,
        /// `!=`
        NotEq => "!=", 3,
        /// `<`
        Lt => "<", 4,
        /// `>`
        Gt => ">", 4,
        /// `<=`
        Le => "<=", 4,
        /// `>=`
        Ge => ">=", 4,

        /// Range operator (`..`).
        Range => "..", 1,
        /// Type check operator (`typeof`).
        TypeOf => "typeof", 1,
        /// Safe-call / chaining operator (`->`).
        Arrow => "->", 9,
    }
}
