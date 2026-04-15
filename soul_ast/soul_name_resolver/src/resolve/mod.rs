use ast::{Block, scope::ScopeId};
use soul_utils::{Ident, ids::FunctionId, soul_error_internal, span::ModuleId};

use crate::NameResolver;
mod resolve_expression;
mod resolve_statement;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_modules(&mut self, module_id: ModuleId) {
        use std::mem::swap;
        
        let mut global = Block::dummy();
        match self.modules.get_mut(module_id) {
            Some(module) => swap(&mut module.global, &mut global),
            None => {
                self.log_error(soul_error_internal!(format!("{:?} not found", module_id), None));
                return
            }
        }

        self.resolve_block(&mut global);
        swap(
            &mut global,
            &mut self.modules.get_mut(module_id).expect("just checked").global,
        );
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
