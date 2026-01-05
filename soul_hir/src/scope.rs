use std::collections::HashMap;

use crate::{ExpressionId, HirId, LocalDefId, hir_type::HirType};
use soul_ast::abstract_syntax_tree::spanned::Spanned;
use soul_utils::{AsIndex, VecMap};

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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BorrowCheckResult {
    pub borrows: VecMap<ExpressionId, BorrowInfo>,
    pub moves: VecMap<ExpressionId, MoveInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeCheckResult {
    pub expression_types: VecMap<ExpressionId, HirType>,
}

/// Spanned borrow information for precise diagnostics.
pub type BorrowInfo = Spanned<InnerBorrowInfo>;
/// Spanned move information for use-after-move detection.
pub type MoveInfo = Spanned<InnerMoveInfo>;

/// Borrow details (what was borrowed, by whom, mutability).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InnerBorrowInfo {
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
