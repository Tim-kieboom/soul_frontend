use crate::{Block, ExpressionId, FunctionId, LocalId, Place, TypeId};
use ast::ast::{BinaryOperator, Literal, UnaryOperator};

/// A typed HIR expression.
///
/// Every expression has a unique ID and an associated type.
/// Source spans are stored externally in `SpanMap`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Expression {
    pub id: ExpressionId,
    pub ty: TypeId,
    pub kind: ExpressionKind,
}

/// The different kinds of HIR expressions.
///
/// Expressions in HIR are fully typed, name-resolved, and free of
/// syntactic sugar. All expressions are identified by an `ExpressionId`,
/// and their source locations are stored externally in the `SpanMap`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    // --- Values ---
    /// A literal value (integer, float, string, etc.).
    Literal(Literal),

    /// A reference to a local variable.
    Local(LocalId),

    /// A reference to a function item.
    Function(FunctionId),

    // --- Memory operations ---
    /// Loads the value from a place.
    ///
    /// This represents reading from a variable, dereference, or indexed location.
    Load(Place),

    /// Creates a reference to a place.
    ///
    /// The `mutable` flag indicates whether this is a mutable (`&`)
    /// or immutable (`@`) reference.
    Ref { place: Place, mutable: bool },

    /// Dereferences a pointer or reference expression.
    DeRef(ExpressionId),

    // --- Operators ---
    /// A unary operation.
    Unary {
        operator: UnaryOperator,
        expression: ExpressionId,
    },

    /// A binary operation.
    Binary {
        left: ExpressionId,
        operator: BinaryOperator,
        right: ExpressionId,
    },

    // --- Control flow ---
    /// An `if` expression.
    ///
    /// The expression evaluates the condition and executes either
    /// the `then_block` or the optional `else_block`.
    If {
        cond: ExpressionId,
        then_block: Block,
        else_block: Option<Block>,
    },

    /// A `while` loop expression.
    ///
    /// If `cond` is `None`, the loop is infinite.
    While {
        cond: Option<ExpressionId>,
        body: Block,
    },

    // --- Calls ---
    /// A function or method call.
    ///
    /// If `callee` is present, this represents a method-style call.
    Call {
        function: FunctionId,
        callee: Option<ExpressionId>,
        arguments: Vec<ExpressionId>,
    },

    // --- Type operations ---
    /// An explicit type cast.
    Cast {
        value: ExpressionId,
        cast_to: TypeId,
    },
}
