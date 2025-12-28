use soul_ast::{
    abstract_syntax_tree::{
        AbstractSyntaxTree,
    },
    sementic_models::scope::ScopeBuilder,
};
use crate::{SementicFault, steps::sementic_analyser::trait_impl_store::TraitImplStore};

pub mod sementic_fault;
pub mod name_resolution;
pub mod trait_impl_store;

pub struct SemanticInfo {
    pub scopes: ScopeBuilder,
    pub faults: Vec<SementicFault>,
    pub trait_impls: TraitImplStore,
}
impl SemanticInfo {
    pub fn new() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults: vec![],
            trait_impls: TraitImplStore::new(),
        }
    }
}

pub(crate) trait SementicPass<'a> {
    fn new(info: &'a mut SemanticInfo) -> Self;
    fn run(&mut self, ast: &mut AbstractSyntaxTree);
}
