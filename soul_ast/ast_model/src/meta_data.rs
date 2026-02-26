use soul_utils::vec_map::VecMapIndex;

use crate::scope::{NodeId, ScopeBuilder};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AstMetadata {
    pub scopes: ScopeBuilder,
    pub last_node_id: NodeId,
}
impl AstMetadata {
    pub fn new() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            last_node_id: NodeId::new_index(0),
        }
    }
}
impl Default for AstMetadata {
    fn default() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            last_node_id: NodeId::new_index(0),
        }
    }
}
