use std::collections::HashMap;

use soul_utils::vec_map::VecMapIndex;

use crate::LocalDefId;

/// Unique identifier for scopes in the HIR scope tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ScopeId(usize);
impl ScopeId {
    pub fn increment(&mut self) {
        self.0 += 1
    }
    pub fn write(&self, sb: &mut String) {
        use std::fmt::Write;
        write!(sb, "{}", self.0).expect("should not have write")
    }
}
impl VecMapIndex for ScopeId {
    fn new_index(value: usize) -> Self {
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
impl Scope {
    pub fn new_global() -> Self {
        Self {
            parent: None,
            locals: HashMap::default(),
        }
    }

    pub fn new_child(parent: ScopeId) -> Self {
        Self {
            parent: Some(parent),
            locals: HashMap::default(),
        }
    }
}
