use models::{
    abstract_syntax_tree::{
        AbstractSyntaxTree,
    },
    sementic_models::scope::ScopeBuilder,
};
use crate::{SementicFault, steps::sementic_analyser::trait_impl_store::TraitImplStore};

pub mod sementic_fault;
pub mod name_resolution;
pub mod trait_impl_store;

pub(crate) struct SemanticInfo {
    scopes: ScopeBuilder,
    faults: Vec<SementicFault>,
    trait_impls: TraitImplStore,
}
impl SemanticInfo {
    pub fn new() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults: vec![],
            trait_impls: TraitImplStore::new(),
        }
    }

    pub fn consume_faults(self) -> Vec<SementicFault> {
        self.faults
    }
}

pub(crate) trait SementicPass<'a> {
    fn new(info: &'a mut SemanticInfo) -> Self;
    fn run(&mut self, ast: &mut AbstractSyntaxTree);
}
