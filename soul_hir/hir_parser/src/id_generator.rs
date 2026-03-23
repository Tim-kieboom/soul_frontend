use hir::{BlockId, ExpressionId, GenericId, LocalId, ModuleId, PlaceId, StatementId};

use soul_utils::ids::{FunctionId, IdGenerator as Generator};

#[derive(Debug, Clone)]
pub(crate) struct IdAllocalors {
    pub(crate) place: Generator<PlaceId>,
    pub(crate) block: Generator<BlockId>,
    pub(crate) local: Generator<LocalId>,
    pub(crate) module: Generator<ModuleId>,
    pub(crate) generic: Generator<GenericId>,
    pub(crate) function: Generator<FunctionId>,
    pub(crate) statement: Generator<StatementId>,
    pub(crate) expression: Generator<ExpressionId>,
}
impl IdAllocalors {
    pub fn new(function: Generator<FunctionId>) -> Self {
        Self {
            function,
            place: Generator::new(),
            block: Generator::new(),
            local: Generator::new(),
            module: Generator::new(),
            generic: Generator::new(),
            statement: Generator::new(),
            expression: Generator::new(),
        }
    }

    pub fn alloc_generic(&mut self) -> GenericId {
        self.generic.alloc()
    }

    pub fn alloc_place(&mut self) -> PlaceId {
        self.place.alloc()
    }

    pub fn alloc_expression(&mut self) -> ExpressionId {
        self.expression.alloc()
    }

    pub fn alloc_local(&mut self) -> LocalId {
        self.local.alloc()
    }

    pub fn alloc_statement(&mut self) -> StatementId {
        self.statement.alloc()
    }

    pub fn alloc_module(&mut self) -> ModuleId {
        self.module.alloc()
    }

    pub fn alloc_function(&mut self) -> FunctionId {
        self.function.alloc()
    }

    pub fn alloc_body(&mut self) -> BlockId {
        self.block.alloc()
    }
}
