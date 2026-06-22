pub use crate::ast::*;
use crate::{
    ast::{
        block::{Block, BlockId},
        expression::{Expression, ExpressionId},
        statements::{Enum, ExternalFunction, Function, FunctionSignature, Statement, StatementId, Struct, Trait},
    },
    declare_store::DeclareStore,
    scope::ScopeBuilder,
};
use soul_utils::{
    CrateContext, FunctionId, Ident, collections::{
        vec_map::{VecMap, VecMapIndex},
        vec_set::VecSet,
    }, fault::{Fault, FaultCollector}, ids::IdGenerator, span::{ModuleId, Spanned}
};
use std::collections::HashMap;

mod ast;
pub mod declare_store;
pub mod scope;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AstTree {
    pub root: ModuleId,
    pub store: AstStore,
    pub context: CrateContext,
    pub modules: AstModuleStore,
    pub scope_info: ScopeInfo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeInfo {
    pub scopes: ScopeBuilder,
    pub last_node_id: NodeId,
}
impl ScopeInfo {
    pub fn new(module: ModuleId) -> Self {
        Self {
            scopes: ScopeBuilder::new(module),
            last_node_id: NodeId::new_index(0),
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AstModuleStore {
    modules: VecMap<ModuleId, Module>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FunctionKind {
    Normal(Function),
    External(ExternalFunction),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub struct_type: Option<EntryKind<CustomType>>,
    pub function: Option<EntryKind<FunctionId>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CustomType {
    Struct(Struct),
    Enum(Enum),
    Trait(Trait),
}
impl CustomType {

    pub fn name(&self) -> &Ident {
        match self {
            CustomType::Struct(obj) => &obj.name,
            CustomType::Enum(obj) => &obj.name,
            CustomType::Trait(obj) => &obj.name,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct EntryKind<T> {
    pub value: T,
    pub is_public: bool,
}

impl AstTree {
    pub fn new(root: ModuleId) -> Self {
        Self {
            root,
            store: AstStore::new(),
            context: CrateContext::default(),
            modules: AstModuleStore::default(),
            scope_info: ScopeInfo::new(root),
        }
    }

    pub fn faults(&self) -> &FaultCollector {
        &self.context.faults
    }

    pub fn log_fault(&mut self, fault: Fault) {
        self.context.faults.push(fault);
    }
}

impl AstModuleStore {
    pub fn insert(&mut self, id: ModuleId, module: Module) -> Option<Module> {
        self.modules.insert(id, module)
    }

    pub fn as_vecmap(&self) -> &VecMap<ModuleId, Module> {
        &self.modules
    }

    pub fn as_vecmap_mut(&mut self) -> &mut VecMap<ModuleId, Module> {
        &mut self.modules
    }

    pub fn contains(&self, id: ModuleId) -> bool {
        self.modules.get(id).is_some()
    }

    pub fn get(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(id)
    }

    pub fn get_mut(&mut self, id: ModuleId) -> Option<&mut Module> {
        self.modules.get_mut(id)
    }
}

impl FunctionKind {
    pub fn signature(&self) -> &Spanned<FunctionSignature> {
        match self {
            FunctionKind::Normal(function) => &function.signature,
            FunctionKind::External(external_function) => &external_function.signature,
        }
    }

    pub fn signature_mut(&mut self) -> &mut Spanned<FunctionSignature> {
        match self {
            FunctionKind::Normal(function) => &mut function.signature,
            FunctionKind::External(external_function) => &mut external_function.signature,
        }
    }
}

impl AstStore {
    pub fn new() -> Self {
        Self {
            blocks: Default::default(),
            declares: Default::default(),
            functions: Default::default(),
            statements: Default::default(),
            expressions: Default::default(),
            block_generator: Default::default(),
            function_generator: Default::default(),
            statement_generator: Default::default(),
            expression_generator: Default::default(),
        }
    }

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
