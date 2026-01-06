use soul_utils::sementic_level::SementicFault;

use crate::scope::ScopeBuilder;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AstMetadata {
    pub scopes: ScopeBuilder,
    pub faults: Vec<SementicFault>,
}
impl AstMetadata {
    pub fn new() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults: vec![],
        }
    }
}
impl Default for AstMetadata {
    fn default() -> Self {
        Self::new()
    }
}