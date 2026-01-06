use soul_utils::Ident;

use crate::{Expression, SoulType};

/// A grouped expression type, such as tuple, array, or named tuple.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExpressionGroup {
    /// A tuple, e.g., `(1, 2, 3)`.
    Tuple(Tuple),
    /// An array literal, e.g., `[1, 2, 3]`.
    Array(Box<Array>),
    /// A named tuple, e.g., `{x: 1, y: 2}`.
    NamedTuple(NamedTuple),
}

/// An array literal, e.g., `[1, 2, 3]`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Array {
    /// Optional explicit collection type.
    pub collection_type: Option<SoulType>,
    /// Optional explicit element type.
    pub element_type: Option<SoulType>,
    /// The array element expressions.
    pub values: Vec<Expression>,
}

/// A named tuple, e.g., `{x: 1, y: 2}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedTuple {
    /// Map of field names to their expression values.
    pub values: Vec<(Ident, Expression)>,

    /// Whether to insert default values for missing fields.
    ///
    /// When `true`, `Foo{field: 1, ..}` means all other fields use their default values.
    pub insert_defaults: bool,
}

pub type Tuple = Vec<Expression>;
