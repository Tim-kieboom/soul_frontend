use std::collections::HashMap;
use soul_ast::sementic_models::sementic_fault::SementicFault;

pub mod expression;
pub mod hir_type;
pub mod item;
pub mod scope;
pub mod statement;
pub use expression::*;
pub use hir_type::*;
pub use item::*;
pub use scope::*;
pub use statement::*;

type Todo = u8;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirResponse {
    pub hir: HirTree,
    pub faults: Vec<SementicFault>,
}

/// High-Level Intermediate Representation (HIR) for Soul programs.
///
/// Decouples semantic analysis from surface syntax for borrow checking,
/// type checking, and optimization. Uses stable `HirId` for analysis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirTree {
    pub root: Module,
}

/// Compilation unit containing all HIR items, bodies, scopes, and expressions.
///
/// Items represent top-level declarations (functions, structs, etc.).
/// Bodies represent executable code (blocks or expressions).
/// Scopes track ownership and borrow state for borrow checking.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Module {
    /// Next available HIR identifier for allocation.
    pub next_id: HirId,
    /// Next available HIR scope identifier for allocation.
    pub next_scope_id: ScopeId,
    /// Top-level items.
    pub items: HashMap<HirId, Item>,
    /// Executable code bodies.
    pub bodies: HashMap<HirId, Body>,
    /// Scope hierarchy for borrow checking and lifetime analysis.
    pub scopes: HashMap<ScopeId, Scope>,
    /// All expressions in the module, indexed by stable IDs.
    pub expressions: HashMap<ExpressionId, Expression>,
}

/// Type alias for HIR body identifiers.
pub type HirBodyId = HirId;
/// Type alias for HIR block identifiers.
pub type HirBlockId = HirId;
/// Type alias for HIR statement identifiers.
pub type StatementId = HirId;
/// Type alias for HIR expression identifiers.
pub type ExpressionId = HirId;

/// Stable identifier for all HIR nodes, stable across compilation passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HirId(u32);
impl HirId {
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

/// Stable identifier for local definitions within a specific HIR owner (item).
///
/// Combines an owner HIR item with a local counter for unambiguous local references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LocalDefId {
    /// HIR item that owns this local definition.
    pub owner: HirId,
    /// Local counter within the owner.
    pub local_id: u32,
}

/// Executable code body in HIR, either a block of statements or a single expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Body {
    Block(HirBlockId),
    Expression(ExpressionId),
}
