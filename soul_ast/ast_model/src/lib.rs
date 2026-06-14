pub use crate::ast::*;
use std::collections::HashMap;
use soul_utils::{FunctionId, collections::{vec_map::VecMap, vec_set::VecSet}, ids::IdGenerator, span::{ModuleId, Spanned}};
use crate::{ast::{block::{Block, BlockId}, expression::{Expression, ExpressionId}, statements::{ExternalFunction, Function, FunctionSignature, Statement, StatementId}}, declare_store::DeclareStore};

mod ast;
pub mod scope;
pub mod declare_store;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleStore {
    pub modules: VecMap<ModuleId, Module>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FunctionKind {
    Normal(Function),
    External(ExternalFunction),
}
impl FunctionKind {
    pub fn signature(&self) -> &Spanned<FunctionSignature> {
        match self {
            FunctionKind::Normal(function) => &function.signature,
            FunctionKind::External(external_function) => &external_function.signature,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AstStore { 
    pub declares: DeclareStore,
    pub blocks: VecMap<BlockId, Block>,
    pub statements: VecMap<StatementId, Statement>,
    pub functions: VecMap<FunctionId, FunctionKind>,
    pub expressions: VecMap<ExpressionId, Expression>,
    block_generator: IdGenerator<BlockId>,
    function_generator: IdGenerator<FunctionId>,
    statement_generator: IdGenerator<StatementId>,
    expression_generator: IdGenerator<ExpressionId>,
}
impl AstStore {
    pub fn insert_block(&mut self, block: Block) -> BlockId {
        let id = self.block_generator.alloc();
        self.blocks.insert(id, block);
        id
    }

    pub fn alloc_function(&mut self) -> FunctionId {
        self.function_generator.alloc()
    }

    pub fn insert_function(&mut self, function: FunctionKind) -> FunctionId {
        let id = function.signature().value.id;
        self.functions.insert(id, function);
        id
    }

    pub fn insert_expression(&mut self, value: Expression) -> ExpressionId {
        let id = self.expression_generator.alloc();
        self.expressions.insert(id, value);
        id
    }

    pub fn insert_statement(&mut self, statement: Statement) -> StatementId {
        let id = self.statement_generator.alloc();
        self.statements.insert(id, statement);
        id
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub global: BlockId,
    pub parent: Option<ModuleId>,
    pub modules: VecSet<ModuleId>,
    pub header: HashMap<String, HeaderEntry>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HeaderEntry {
    pub variable: Option<EntryKind<NodeId>>,
    pub function: Option<EntryKind<FunctionId>>,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct EntryKind<T> {
    pub value: T,
    pub is_public: bool,
}