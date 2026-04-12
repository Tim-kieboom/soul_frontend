use hir::{BlockId, ExpressionId, FieldId, LocalId, PlaceId, StatementId};
use soul_utils::ids::{FunctionId, IdGenerator};

#[derive(Debug, Clone, Default)]
pub(crate) struct IdAllocalor {
    pub(crate) place: IdGenerator<PlaceId>,
    pub(crate) block: IdGenerator<BlockId>,
    pub(crate) local: IdGenerator<LocalId>,
    pub(crate) field: IdGenerator<FieldId>,
    pub(crate) function: IdGenerator<FunctionId>,
    pub(crate) statement: IdGenerator<StatementId>,
    pub(crate) expression: IdGenerator<ExpressionId>,
}
impl IdAllocalor {
    pub fn new(function: IdGenerator<FunctionId>) -> Self {
        Self {
            function,
            ..Default::default()
        }
    }

    pub fn alloc_field(&mut self) -> FieldId {
        self.field.alloc()
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

    pub fn alloc_function(&mut self) -> FunctionId {
        self.function.alloc()
    }

    pub fn alloc_body(&mut self) -> BlockId {
        self.block.alloc()
    }
}
