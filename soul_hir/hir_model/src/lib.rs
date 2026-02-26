mod expression;
mod function;
mod hir_type;
mod ids;
mod meta_data_maps;
mod place;
mod statement;
pub use expression::*;
pub use function::*;
pub use hir_type::*;
pub use ids::*;
pub use meta_data_maps::*;
pub use place::*;
use soul_utils::{span::Span, vec_map::VecMap};
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
    pub types: TypesMap,

    /// Side-table containing source spans
    /// for HIR nodes.
    pub spans: SpanMap,

    /// Side-table containing auxiliary metadata
    /// for HIR nodes.
    pub meta_data: MetaDataMap,

    pub imports: ImportMap,
    pub blocks: VecMap<BlockId, Block>,
    pub locals: VecMap<LocalId, TypeId>,
    pub functions: VecMap<FunctionId, Function>,
    pub expressions: VecMap<ExpressionId, Expression>,
}

impl HirTree {
    pub fn new(root_id: ModuleId) -> Self {
        Self {
            types: TypesMap::new(),
            spans: SpanMap::default(),
            blocks: VecMap::default(),
            locals: VecMap::default(),
            imports: ImportMap::new(),
            functions: VecMap::default(),
            expressions: VecMap::default(),
            meta_data: MetaDataMap::default(),
            root: Module {
                id: root_id,
                imports: vec![],
                globals: vec![],
            },
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
    Function(FunctionId, StatementId),
    Variable(Variable, StatementId),

    /// (allows mutable) only allowed to be used by hir lowerer not user
    InternalVariable(Variable, StatementId),
    /// only allowed to be used by hir lowerer not user
    InternalAssign(Assign, StatementId),
}
impl Global {
    pub fn get_id(&self) -> StatementId {
        match self {
            Global::Function(_, statement_id) => *statement_id,
            Global::Variable(_, statement_id) => *statement_id,
            Global::InternalAssign(_, statement_id) => *statement_id,
            Global::InternalVariable(_, statement_id) => *statement_id,
        }
    }

    pub fn get_span(&self, spans: &SpanMap) -> Span {
        let span = match self {
            Global::Variable(variable, _) => spans.locals.get(variable.local),
            Global::Function(function_id, _) => spans.functions.get(*function_id),
            Global::InternalAssign(assign, _) => spans.expressions.get(assign.value),
            Global::InternalVariable(variable, _) => spans.locals.get(variable.local),
        };

        match span {
            Some(val) => *val,
            None => {
                #[cfg(debug_assertions)]
                panic!("span not found of {:?}", self);
                #[cfg(not(debug_assertions))]
                Span::default_const()
            }
        }
    }

    pub const fn should_be_inmutable(&self) -> bool {
        match self {
            Global::Function(_, _) | Global::InternalVariable(_, _) => false,

            Global::Variable(_, _) | Global::InternalAssign(_, _) => true,
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
