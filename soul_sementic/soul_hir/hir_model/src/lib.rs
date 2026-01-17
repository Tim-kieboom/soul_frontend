use parser_models::scope::NodeId;
use soul_utils::{sementic_level::SementicFault, span::Span, vec_map::VecMap};

mod expression;
mod hir_type;
mod item;
mod scope;
mod statement;

pub use expression::*;
pub use hir_type::*;
pub use item::*;
pub use scope::*;
pub use statement::*;

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
    /// Next available HIR scope identifier for allocation.
    pub next_scope_id: ScopeId,
    pub functions: VecMap<NodeId, FunctionSignature>,
    /// Top-level items.
    pub items: VecMap<NodeId, Item>,
    /// Executable code bodies.
    pub bodies: VecMap<NodeId, Body>,
    /// Scope hierarchy for borrow checking and lifetime analysis.
    pub scopes: VecMap<ScopeId, Scope>,
    /// All expressions in the module, indexed by stable IDs.
    pub expressions: VecMap<ExpressionId, Expression>,
}

/// Type alias for HIR body identifiers.
pub type BodyId = NodeId;
/// Type alias for HIR block identifiers.
pub type BlockId = NodeId;
/// Type alias for HIR statement identifiers.
pub type StatementId = NodeId;
/// Type alias for HIR expression identifiers.
pub type ExpressionId = NodeId;

/// Stable identifier for local definitions within a specific HIR owner (item).
///
/// Combines an owner HIR item with a local counter for unambiguous local references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LocalDefId {
    /// HIR item that owns this local definition.
    pub owner: NodeId,
    /// Local counter within the owner.
    pub local_id: u32,
}
impl LocalDefId {
    pub fn write(&self, sb: &mut String) {
        sb.push_str("LocalDefId{");
        sb.push_str("owner: ");
        self.owner.write(sb);
        sb.push_str(&format!(", local_id: {}", self.local_id));
        sb.push('}');
    } 
}

/// Executable code body in HIR, either a block of statements or a single expression.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Body {
    Block(Block, Span),
    Expression(ExpressionId, Span),
}
impl Body {
    pub fn span(&self) -> Span {
        match self {
            Body::Block(_, span) => *span,
            Body::Expression(_, span) => *span,
        } 
    }
}