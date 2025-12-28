use soul_ast::{
    abstract_syntax_tree::{block::Block, soul_type::SoulType, statment::Ident},
    sementic_models::scope::{NodeId, ScopeId},
};

use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;

mod name_resolve_expression;
mod name_resolve_statment;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_names(&mut self, block: &mut Block) {
        self.resolve_block(block);
    }

    fn lookup_function_candidates(&mut self, ident: &Ident) -> Vec<NodeId> {
        self.info.scopes.lookup_function_candidates(ident)
    }

    fn try_go_to(&mut self, scope_id: Option<ScopeId>) {
        debug_assert!(scope_id.is_some());
        if let Some(index) = scope_id {
            self.info.scopes.go_to(index);
        }
    }

    fn insert_trait_impl(&mut self, trait_impl: SoulType, of_type: SoulType) {
        if let Err(msg) = self.info.trait_impls.insert(trait_impl, of_type) {
            self.log_error(msg);
        }
    }
}
