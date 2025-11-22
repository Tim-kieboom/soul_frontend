use std::collections::{BTreeMap};
use crate::{abstract_syntax_tree::{expression::{BoxExpression, Expression}, soul_type::SoulType, statment::Ident}, scope::scope::ScopeId};

/// A grouped expression type, such as tuple, array, or named tuple.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExpressionGroup {
    /// A tuple, e.g., `(1, 2, 3)`.
    Tuple(Tuple),
    /// An array literal, e.g., `[1, 2, 3]`.
    Array(Array),
    /// A named tuple, e.g., `{x: 1, y: 2}`.
    NamedTuple(NamedTuple),
    /// An array filler expression, e.g., `[5 => 1] //makes [1,1,1,1,1]`.
    ArrayFiller(ArrayFiller),
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

/// An array filler expression, e.g., `[for i in 5 => i] //makes [0,1,2,3,4]`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArrayFiller {
    /// Optional explicit collection type.
    pub collection_type: Option<SoulType>,
    /// Optional explicit element type.
    pub element_type: Option<SoulType>,
    /// Expression that evaluates to the number of elements to create.
    pub amount: BoxExpression,
    /// Optional identifier for the index variable in the fill expression.
    pub index: Option<Ident>,
    /// Expression to evaluate for each element.
    pub fill_expr: BoxExpression,
    /// The scope identifier for this array filler.
    pub scope_id: ScopeId,
}

/// A named tuple, e.g., `{x: 1, y: 2}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedTuple {
    /// Map of field names to their expression values.
    pub values: BTreeMap<Ident, Expression>,
    
    /// Whether to insert default values for missing fields.
    ///
    /// When `true`, `Foo{field: 1, ..}` means all other fields use their default values.
    pub insert_defaults: bool,
}

/// A tuple, e.g., `(1, 2, 3)`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tuple {
    /// The tuple element expressions.
    pub values: Vec<Expression>
}