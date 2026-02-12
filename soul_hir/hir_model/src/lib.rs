mod expression;
mod function;
mod ids;
mod meta_data_maps;
mod place;
mod hir_type;
mod statement;
pub use hir_type::*;
pub use expression::*;
pub use function::*;
pub use ids::*;
pub use meta_data_maps::*;
pub use place::*;
use soul_utils::{vec_map::VecMap};
pub use statement::*;

/// High-level Intermediate Representation (HIR) tree.
///
/// The HIR contains the fully resolved and typed structure of a module.
/// It is free of syntax-only information such as tokens or spans.
///
/// Source locations and auxiliary semantic information are stored separately
/// in `MetaDataMap` and are indexed by IR node IDs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirTree {
    /// Root module of the HIR.
    pub root: Module,
    
    /// Side-table containing all types
    /// for HIR nodes.
    pub types: TypedContext,
    
    /// Side-table containing source spans
    /// for HIR nodes.
    pub spans: SpanMap,
    
    /// Side-table containing auxiliary metadata
    /// for HIR nodes.
    pub meta_data: MetaDataMap,
    
    pub imports: ImportMap,
    pub blocks: VecMap<BlockId, Block>,
    pub expressions: VecMap<ExpressionId, Expression>,
}


impl HirTree {
    pub fn new(root_id: ModuleId) -> Self {
        Self {
            imports: ImportMap::new(),
            blocks: VecMap::default(),
            spans: SpanMap::default(),
            types: TypedContext::new(),
            expressions: VecMap::default(),
            meta_data: MetaDataMap::default(),
            root: Module { id: root_id, imports: vec![], globals: vec![] },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub imports: Vec<Import>,
    pub globals: Vec<Global>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Import {
    pub module: ModuleId,
    pub kind: ast::ImportKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Global {
    Function(Function, StatementId),
    Variable(Variable, StatementId),

    /// only allowed to be used by hir lowerer not user
    InternalAssign(Assign, StatementId),
}
impl Global {
    pub fn get_id(&self) -> StatementId {
        match self {
            Global::Function(_, statement_id) => *statement_id,
            Global::Variable(_, statement_id) => *statement_id,
            Global::InternalAssign(_, statement_id) => *statement_id,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub ty: TypeId,
    pub local: LocalId,
    pub value: Option<ExpressionId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub imports: Vec<Import>,
    pub statements: Vec<Statement>,
    pub terminator: Option<ExpressionId>,
}
impl Block {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            imports: vec![],
            terminator: None,
            statements: vec![],
        }
    }
}
