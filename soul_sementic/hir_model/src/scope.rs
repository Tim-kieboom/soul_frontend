use std::collections::HashMap;

use soul_utils::vec_map::AsIndex;

use crate::LocalDefId;

/// Unique identifier for scopes in the HIR scope tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ScopeId(usize);
impl ScopeId {
    pub fn increment(&mut self) {
        self.0 += 1
    }
}
impl AsIndex for ScopeId {
    fn new(value: usize) -> Self {
        Self(value)
    }

    fn index(&self) -> usize {
        self.0
    }
}

/// Scope information for borrow checking, lifetime analysis, and name resolution.
///
/// Tracks local variables, active borrows, moves, and type parameters within a lexical scope.
/// Forms a tree structure via `parent` pointers for nested scope analysis.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub locals: HashMap<String, LocalDefId>,
}
