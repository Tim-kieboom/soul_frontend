use parser_models::{ast::Block, scope::{NodeId, ScopeId, ScopeValueEntryKind}};
use soul_utils::Ident;

use crate::NameResolver;
mod resolve_statement;
mod resolve_expression;

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

    fn lookup_function(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValueEntryKind::Function)
    }
}