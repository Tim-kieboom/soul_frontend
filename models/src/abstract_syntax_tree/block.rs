use crate::{abstract_syntax_tree::statment::Statement, scope::scope::ScopeId, soul_names::TypeModifier};

/// A block of statements with an associated scope.
///
/// Blocks can have type modifiers (like `const` or `mut`) and contain
/// a sequence of statements that execute in order.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Block {
    /// The type modifier applied to this block.
    pub modifier: TypeModifier,
    /// The statements contained in this block.
    pub statments: Vec<Statement>,
    /// The scope identifier for this block's lexical scope.
    pub scope_id: ScopeId,
}