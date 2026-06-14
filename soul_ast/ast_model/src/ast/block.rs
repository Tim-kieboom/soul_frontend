use soul_utils::{TypeModifier, impl_soul_ids, span::Span};

use crate::{NodeId, scope::ScopeId, statements::StatementId};

impl_soul_ids!(BlockId);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub node_id: Option<NodeId>,
    pub scope_id: Option<ScopeId>,

    pub statements: Vec<StatementId>,
    pub modifier: TypeModifier,
    pub span: Span,
}