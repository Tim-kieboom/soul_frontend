use soul_utils::{span::ModuleId, vec_map::VecMapIndex};

use crate::scope::{NodeId, ScopeBuilder};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AstMetadata {
    pub scopes: ScopeBuilder,
    pub last_node_id: NodeId,
}
impl AstMetadata {
    pub fn new(module: ModuleId) -> Self {
        Self {
            scopes: ScopeBuilder::new(module),
            last_node_id: NodeId::new_index(0),
        }
    }
}
