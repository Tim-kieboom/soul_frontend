use soul_utils::SementicFault;

use crate::{abstract_syntax_tree::AbstractSyntaxTree, sementic_models::{scope::ScopeBuilder, trait_impl_store::TraitImplStore}};

pub mod scope;
pub mod trait_impl_store;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AstMetadata {
    pub scopes: ScopeBuilder,
    pub faults: Vec<SementicFault>,
    pub trait_impls: TraitImplStore,
}
impl AstMetadata {
    pub fn new() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults: vec![],
            trait_impls: TraitImplStore::new(),
        }
    }
}

pub trait SementicPass<'a> {
    fn new(info: &'a mut AstMetadata) -> Self;
    fn run(&mut self, ast: &mut AbstractSyntaxTree);
}