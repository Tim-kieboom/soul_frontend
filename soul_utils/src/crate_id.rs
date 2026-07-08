use crate::ids::{IdAlloc, IdGenerator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CrateId(pub u32);

impl CrateId {
    pub const MAIN: CrateId = CrateId(1);
    pub const INVALID: CrateId = CrateId(0);
}

impl IdAlloc for CrateId {
    fn error() -> Self {
        Self::INVALID
    }

    fn begin() -> Self {
        Self(2)
    }

    fn alloc(&mut self) -> Self {
        let new = Self(self.0);
        self.0 += 1;
        new
    }

    fn last(&self) -> Self {
        Self(self.0)
    }
}

impl Default for CrateId {
    fn default() -> Self {
        Self::MAIN
    }
}

pub type CrateIdGenerator = IdGenerator<CrateId>;
