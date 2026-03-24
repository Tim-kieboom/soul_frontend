use soul_utils::Ident;

use crate::{
    ast::{Expression, SoulType},
    scope::NodeId,
};

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

/// An struct literal, e.g., `Struct{field: 1, field2: 2}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StructConstructor {
    pub id: Option<NodeId>,
    pub struct_type: SoulType,
    pub values: Vec<(Ident, Expression)>,
    pub defaults: bool,
}
