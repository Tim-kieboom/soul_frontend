mod expression;
mod function;
mod hir_maps;
mod hir_type;
mod ids;
mod place;
mod statement;
use ast::FunctionKind;
pub use expression::*;
pub use function::*;
pub use hir_maps::*;
pub use hir_type::*;
pub use ids::*;
pub use place::*;
use soul_utils::{
    Ident,
    ids::{FunctionId, IdAlloc},
    soul_names::INIT_GLOBALS_FUNCTION_NAME,
    span::{ModuleId, Span},
};
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
    pub root: ModuleId,
    pub info: InfoMaps,
    pub nodes: NodeMaps,

    pub main: FunctionId,
    pub init_globals: FunctionId,
}
impl HirTree {
    pub fn new(ast_root: &ast::Module, main: FunctionId, init_globals: FunctionId) -> Self {
        let init_global_function = Function {
            id: init_globals,
            generics: vec![],
            parameters: vec![],
            owner_type: TypeId::error(),
            return_type: TypeId::error(),
            kind: FunctionKind::Static,
            body: FunctionBody::Internal(BlockId::error()),
            name: Ident::new(
                INIT_GLOBALS_FUNCTION_NAME.to_string(),
                Span::default(ast_root.id),
            ),
        };

        let mut nodes = NodeMaps::new(init_global_function);
        let root = Module {
            id: ast_root.id,
            globals: vec![],
            modules: ast_root.modules.clone(),
        };
        let root_id = root.id;
        nodes.modules.insert(root_id, root);

        Self {
            main,
            nodes,
            init_globals,
            root: root_id,
            info: InfoMaps::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub modules: Vec<ModuleId>,
    pub globals: Vec<Global>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Global {
    pub kind: GlobalKind,
    pub id: StatementId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GlobalKind {
    Function(FunctionId),
    Variable(Variable),

    /// (allows mutable) only allowed to be used by hir lowerer not user
    InternalVariable(Variable),
    /// only allowed to be used by hir lowerer not user
    InternalAssign(Assign),
}
impl Global {
    pub fn new(kind: GlobalKind, id: StatementId) -> Self {
        Self { kind, id }
    }

    pub fn get_span(&self, spans: &SpanMap) -> Span {
        let span = match &self.kind {
            GlobalKind::Variable(variable) => spans.locals.get(variable.local),
            GlobalKind::Function(function_id) => spans.functions.get(*function_id),
            GlobalKind::InternalAssign(assign) => spans.expressions.get(assign.value),
            GlobalKind::InternalVariable(variable) => spans.locals.get(variable.local),
        };

        match span {
            Some(val) => *val,
            None => {
                #[cfg(debug_assertions)]
                panic!("span not found of {:?}", self.kind);
                #[cfg(not(debug_assertions))]
                Span::default_const()
            }
        }
    }

    pub const fn should_be_inmutable(&self) -> bool {
        match self.kind {
            GlobalKind::Function(_) | GlobalKind::InternalVariable(_) => false,
            GlobalKind::Variable(_) | GlobalKind::InternalAssign(_) => true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub local: LocalId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub statements: Vec<Statement>,
    pub terminator: Option<Terminator>,
}
impl Block {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            terminator: None,
            statements: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Terminator {
    Return(ExpressionId),
    Expression(ExpressionId),
}
impl Terminator {
    pub fn get_expression_id(&self) -> ExpressionId {
        match self {
            Terminator::Return(expression_id) => *expression_id,
            Terminator::Expression(expression_id) => *expression_id,
        }
    }
}
