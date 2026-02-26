use hir::{BlockId, ExpressionId, FunctionId, LocalId, ModuleId, PlaceId, StatementId};

#[derive(Debug, Clone)]
pub(crate) struct IdGenerator {
    pub(crate) place: hir::IdGenerator<PlaceId>,
    pub(crate) block: hir::IdGenerator<BlockId>,
    pub(crate) local: hir::IdGenerator<LocalId>,
    pub(crate) module: hir::IdGenerator<ModuleId>,
    pub(crate) function: hir::IdGenerator<FunctionId>,
    pub(crate) statement: hir::IdGenerator<StatementId>,
    pub(crate) expression: hir::IdGenerator<ExpressionId>,
}
impl IdGenerator {
    pub fn new() -> Self {
        Self {
            place: hir::IdGenerator::new(),
            block: hir::IdGenerator::new(),
            local: hir::IdGenerator::new(),
            module: hir::IdGenerator::new(),
            function: hir::IdGenerator::new(),
            statement: hir::IdGenerator::new(),
            expression: hir::IdGenerator::new(),
        }
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
