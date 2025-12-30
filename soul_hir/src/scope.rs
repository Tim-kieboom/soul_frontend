use std::collections::HashMap;

use soul_ast::abstract_syntax_tree::{spanned::Spanned, statment::Ident};

use crate::{ExpressionId, HirId, hir_type::HirType};

/// Unique identifier for scopes in the HIR scope tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ScopeId(u32);
impl ScopeId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }
    pub fn increment(&mut self) {
        self.0+=1
    }
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// Scope information for borrow checking, lifetime analysis, and name resolution.
///
/// Tracks local variables, active borrows, moves, and type parameters within a lexical scope.
/// Forms a tree structure via `parent` pointers for nested scope analysis.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    /// Nesting depth for debugging and analysis.
    pub depth: u32,
    /// All active borrows in this scope (tracked by borrow checker).
    pub borrows: Vec<Borrow>,
    /// Parent scope for lifetime chaining and lookup fallback.
    pub parent: Option<ScopeId>,
    /// Mutability rules for this scope (affects borrow validity).
    pub mutability: ScopeMutability,
    /// Local variable bindings (name -> HIR ID).
    pub locals: HashMap<String, HirId>,
    /// Variables moved out of this scope (prevents use-after-move).
    pub moves: HashMap<HirId, MoveInfo>,
    /// Active borrows indexed by borrowed item for fast conflict detection.
    pub active_borrows: HashMap<HirId, Vec<Borrow>>,

    /// Generic type parameters declared in this scope.
    pub type_parameters: Vec<Ident>,
    /// Concrete type bindings for generic parameters.
    pub type_bindings: HashMap<Ident, HirType>,
}

/// Spanned borrow information for precise diagnostics.
pub type Borrow = Spanned<InnerBorrow>;
/// Spanned move information for use-after-move detection.
pub type MoveInfo = Spanned<InnerMoveInfo>;

/// Borrow details (what was borrowed, by whom, mutability).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InnerBorrow {
    /// HIR ID of the borrowed value/place.
    pub owner_id: HirId,
    /// Whether this is a mutable borrow.
    pub is_mutable: bool,
    /// HIR ID of the borrower expression/statement.
    pub borrower_id: HirId,
    /// Scope where the borrow began (for lifetime checking).
    pub scope_start: ScopeId,
}

/// Move details (where a value was moved from/to).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InnerMoveInfo {
    /// Scope where the value was originally defined.
    pub from_scope: ScopeId,
    /// Expression that consumed/moved the value.
    pub to_expression: ExpressionId,
}

/// Scope mutability rules for borrow checking.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScopeMutability {
    /// No borrows permitted in this scope.
    NoBorrows,
    /// Immutable borrows only.
    Const,
    /// Mutable and Immutable borrows permitted.
    Mut,
}
