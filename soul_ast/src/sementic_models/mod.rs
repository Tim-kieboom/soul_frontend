use crate::{abstract_syntax_tree::AbstractSyntaxTree, sementic_models::{scope::ScopeBuilder, sementic_fault::SementicFault, trait_impl_store::TraitImplStore}};

pub mod scope;
pub mod sementic_fault;
pub mod trait_impl_store;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ASTSemanticInfo {
    pub scopes: ScopeBuilder,
    pub faults: Vec<SementicFault>,
    pub trait_impls: TraitImplStore,
}
impl ASTSemanticInfo {
    pub fn new() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults: vec![],
            trait_impls: TraitImplStore::new(),
        }
    }
}

pub trait SementicPass<'a> {
    fn new(info: &'a mut ASTSemanticInfo) -> Self;
    fn run(&mut self, ast: &mut AbstractSyntaxTree);
}