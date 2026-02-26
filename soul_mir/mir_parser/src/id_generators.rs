use crate::mir;
use hir::IdGenerator;

pub(crate) struct IdGenerators {
    temp: IdGenerator<mir::TempId>,
    block: IdGenerator<mir::BlockId>,
    local: IdGenerator<mir::LocalId>,
    place: IdGenerator<mir::PlaceId>,
    statement: IdGenerator<mir::StatementId>,
}
impl IdGenerators {
    pub(crate) fn new() -> Self {
        Self {
            temp: IdGenerator::new(),
            block: IdGenerator::new(),
            local: IdGenerator::new(),
            place: IdGenerator::new(),
            statement: IdGenerator::new(),
        }
    }

    pub(crate) fn alloc_temp(&mut self) -> mir::TempId {
        self.temp.alloc()
    }

    pub(crate) fn alloc_place(&mut self) -> mir::PlaceId {
        self.place.alloc()
    }

    pub(crate) fn alloc_block(&mut self) -> mir::BlockId {
        self.block.alloc()
    }

    pub(crate) fn alloc_local(&mut self) -> mir::LocalId {
        self.local.alloc()
    }

    pub(crate) fn alloc_statement(&mut self) -> mir::StatementId {
        self.statement.alloc()
    }
}
