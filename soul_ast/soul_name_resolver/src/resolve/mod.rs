use ast::{Block, scope::ScopeId};
use soul_utils::{ids::FunctionId, soul_error_internal, span::ModuleId};

use crate::NameResolver;
mod resolve_expression;
mod resolve_function_call;
mod resolve_statement;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_modules(&mut self, module_id: ModuleId) {
        use std::mem::swap;

        let mut global = Block::dummy();
        match self.modules.get_mut(module_id) {
            Some(module) => swap(&mut module.global, &mut global),
            None => {
                self.log_error(soul_error_internal!(
                    format!("{:?} not found", module_id),
                    None
                ));
                return;
            }
        }

        let prev = self.current.module;
        self.current.module = module_id;
        self.resolve_block(&mut global);
        self.current.module = prev;

        swap(
            &mut global,
            &mut self
                .modules
                .get_mut(module_id)
                .expect("just checked")
                .global,
        );
    }

    fn try_go_to(&mut self, scope_id: Option<ScopeId>) {
        debug_assert!(scope_id.is_some());
        if let Some(index) = scope_id {
            self.info
                .scopes
                .go_to(index, self.current.module)
                .expect("no err");
        }
    }

    pub(super) fn lookup_function(&mut self, name: &str) -> Option<FunctionId> {
        self.info.scopes.lookup_function(name, self.current.module)
    }
}
