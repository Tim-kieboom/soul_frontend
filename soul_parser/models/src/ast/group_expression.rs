use crate::{ast::{Expression, SoulType}, scope::NodeId};

/// An array literal, e.g., `[1, 2, 3]`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Array {
    pub id: Option<NodeId>,
    /// Optional explicit collection type.
    pub collection_type: Option<SoulType>,
    /// Optional explicit element type.
    pub element_type: Option<SoulType>,
    /// The array element expressions.
    pub values: Vec<Expression>,
}
