use soul_utils::vec_map::VecMapIndex;

macro_rules! impl_id {
    ($($ty:ty), *) => {$(

        impl VecMapIndex for $ty {
            fn new_index(value: usize) -> Self {
                Self(value)
            }

            fn index(&self) -> usize {
                self.0
            }
        }

        impl $ty {
            /// gives current value and increments self
            fn alloc(&mut self) -> Self {
                let new = *self;
                self.0 += 1;
                new
            }
        }
    )*}
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct ExpressionId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct StatementId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct LocalId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct ItemId(usize);

impl_id!(ExpressionId, StatementId, LocalId, ItemId);

pub struct ItemIdGenerator {
    item: ItemId,
}
impl ItemIdGenerator {
    pub fn new() -> Self {
        Self {
            item: ItemId(0),
        }
    }

    pub fn alloc(&mut self) -> ItemId {
        self.item.alloc()
    }
}

pub struct IdGenerator {
    expression: ExpressionId,
    statement: StatementId,
    local: LocalId,
}
impl IdGenerator {
    pub fn new() -> Self {
        Self {
            expression: ExpressionId(0),
            statement: StatementId(0),
            local: LocalId(0),
        }
    }

    pub fn alloc_expression(&mut self) -> ExpressionId {
        self.expression.alloc()
    }

    pub fn alloc_statement(&mut self) -> StatementId {
        self.statement.alloc()
    } 

    pub fn alloc_local(&mut self) -> LocalId {
        self.local.alloc()
    }
}