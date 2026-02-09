use soul_utils::vec_map::VecMapIndex;

/// used for internal idGenerator
pub trait IdAlloc {
    /// Returns the initial value for the ID generator.
    fn begin() -> Self;

    /// Allocates a new unique ID and advances the generator.
    fn alloc(&mut self) -> Self;
}

/// Generates strongly-typed ID newtypes and implements allocation logic.
///
/// Each generated ID type is backed by a `usize` and can be used
/// as an index into `VecMap`.
macro_rules! impl_ids {
    ($($ty:ident),*) => {
        $(
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            pub struct $ty(usize);

            impl VecMapIndex for $ty {
                fn new_index(value: usize) -> Self {
                    Self(value)
                }

                fn index(&self) -> usize {
                    self.0
                }
            }

            impl IdAlloc for $ty {
                fn begin() -> Self {
                    Self(0)
                }

                fn alloc(&mut self) -> Self {
                    let new = self.clone();
                    self.0 += 1;
                    new
                }
            }
        )*
    };
}

impl_ids!(
    ModuleId,
    FunctionId,
    FieldId,
    TypeId,
    LocalId,
    StatementId,
    ExpressionId
);

/// Generates unique IDs for a given IR context.
///
/// Each generator instance produces a monotonically increasing
/// sequence of IDs of a single type.
pub struct IdGenerator<Id: IdAlloc> {
    current: Id,
}
impl<Id: IdAlloc> IdGenerator<Id> {
    /// Creates a new ID generator.
    pub fn new() -> Self {
        Self {
            current: Id::begin(),
        }
    }

    /// Allocates and returns a fresh ID.
    pub fn alloc(&mut self) -> Id {
        self.current.alloc()
    }
}
