use soul_utils::{impl_soul_ids, span::Span};

use crate::statements::StatementId;

impl_soul_ids!(BlockId);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub statements: Vec<StatementId>,
    pub is_const: bool,
    pub span: Span,
}
