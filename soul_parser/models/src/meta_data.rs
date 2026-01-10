use soul_utils::{sementic_level::SementicFault, vec_map::AsIndex};

use crate::scope::{NodeId, ScopeBuilder};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AstMetadata {
    pub scopes: ScopeBuilder,
    pub faults: Vec<SementicFault>,
    pub last_node_id: NodeId,
}
impl AstMetadata {
    pub fn new(faults: Vec<SementicFault>) -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults,
            last_node_id: NodeId::new(0),
        }
    }
}
impl Default for AstMetadata {
    fn default() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults: vec![],
            last_node_id: NodeId::new(0),
        }
    }
}