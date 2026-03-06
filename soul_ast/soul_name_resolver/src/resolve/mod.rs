use ast::{
    Block,
    scope::ScopeId,
};
use soul_utils::{Ident, ids::FunctionId};

use crate::NameResolver;
mod resolve_expression;
mod resolve_statement;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_names(&mut self, block: &mut Block) {
        self.resolve_block(block);
    }

    fn try_go_to(&mut self, scope_id: Option<ScopeId>) {
        debug_assert!(scope_id.is_some());
        if let Some(index) = scope_id {
            self.info.scopes.go_to(index);
        }
    }

    fn lookup_function(&mut self, name: &Ident) -> Option<FunctionId> {
        self.info.scopes.lookup_function(name)
    }
}
