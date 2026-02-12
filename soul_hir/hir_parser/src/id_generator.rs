use hir::{BlockId, ExpressionId, FunctionId, InferVarId, LocalId, ModuleId, StatementId, TypeId};

#[derive(Debug, Clone)]
pub(crate) struct IdGenerator {
    pub(crate) ty: hir::IdGenerator<TypeId>,
    pub(crate) block: hir::IdGenerator<BlockId>,
    pub(crate) local: hir::IdGenerator<LocalId>,
    pub(crate) module: hir::IdGenerator<ModuleId>,
    pub(crate) infer: hir::IdGenerator<InferVarId>,
    pub(crate) function: hir::IdGenerator<FunctionId>,
    pub(crate) statement: hir::IdGenerator<StatementId>,
    pub(crate) expression: hir::IdGenerator<ExpressionId>,
}
impl IdGenerator {
    pub fn new() -> Self {
        Self {
            ty: hir::IdGenerator::new(),
            block: hir::IdGenerator::new(),
            local: hir::IdGenerator::new(),
            infer: hir::IdGenerator::new(),
            module: hir::IdGenerator::new(),
            function: hir::IdGenerator::new(),
            statement: hir::IdGenerator::new(),
            expression: hir::IdGenerator::new(),
        }
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

    pub fn alloc_infer(&mut self) -> InferVarId {
        self.infer.alloc()
    }

    pub fn alloc_body(&mut self) -> BlockId {
        self.block.alloc()
    }
}
