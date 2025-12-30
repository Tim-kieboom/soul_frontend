pub mod name_resolution;
<<<<<<< Updated upstream
=======
pub mod sementic_fault;
pub mod trait_impl_store;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AstMetaData {
    pub scopes: ScopeBuilder,
    pub faults: Vec<SementicFault>,
    pub trait_impls: TraitImplStore,
}
impl AstMetaData {
    pub fn new() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults: vec![],
            trait_impls: TraitImplStore::new(),
        }
    }
}

pub(crate) trait SementicPass<'a> {
    fn new(info: &'a mut AstMetaData) -> Self;
    fn run(&mut self, ast: &mut AbstractSyntaxTree);
}
>>>>>>> Stashed changes
