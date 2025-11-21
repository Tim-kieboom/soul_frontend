use crate::{soul_names::TypeModifiers, steps::abstract_syntax_tree::scope::scope::ScopeId};


#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub modifier: TypeModifiers,
    pub statments: Vec<Statment>,
    pub scope_id: ScopeId,
}