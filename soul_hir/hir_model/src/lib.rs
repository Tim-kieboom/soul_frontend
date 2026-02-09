mod expression;
mod function;
mod ids;
mod meta_data_map;
mod place;
mod statement;
pub use expression::*;
pub use function::*;
pub use ids::*;
pub use meta_data_map::*;
pub use place::*;
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

    /// Side-table containing source spans
    /// for HIR nodes.
    pub spans: SpanMap,

    /// Side-table containing auxiliary metadata
    /// for HIR nodes.
    pub meta_data: MetaDataMap,
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Global {
    Function(Function),
    Variable(Variable),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub local: LocalId,
    pub ty: TypeId,
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub terminator: Option<ExpressionId>,
}
