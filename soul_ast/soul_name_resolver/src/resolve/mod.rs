use ast_model::block::BlockId;
use soul_utils::{soul_error_internal, span::ModuleId};

use crate::NameResolver;

mod expression;
mod function_call;
mod return_type;
mod statement;

#[cfg(test)]
mod binary_expression_tests;
#[cfg(test)]
mod function_call_argument_tests;
#[cfg(test)]
mod function_call_tests;
#[cfg(test)]
mod return_type_tests;

impl<'a> NameResolver<'a> {
    pub(crate) fn resolve_module(&mut self, id: ModuleId) {
        let global = match self.ast_modules.get(id) {
            Some(module) => module.global,
            None => {
                self.log_fault(soul_error_internal!(format!("{id:?} not found"), None));
                return;
            }
        };

        let prev = self.current.module;
        self.current.module = id;
        self.current.in_global = true;
        self.resolve_block(global);
        self.current.module = prev;
    }

    fn resolve_block(&mut self, block_id: BlockId) {
        self.try_go_to(block_id);
        let Some(block) = self.store.blocks.get(block_id) else {
            self.log_fault(soul_error_internal!(
                format!("{block_id:?} not found"),
                None
            ));
            return;
        };

        for statement in &block.statements {
            self.resolve_statement(*statement)
        }
    }

    fn try_go_to(&mut self, block_id: BlockId) {
        let scope_id = self.scope_ids.get(block_id).copied();

        if !scope_id.is_some() {
            println!("breakpoint at: {}:{}", file!(), line!());
        }
        debug_assert!(scope_id.is_some());
        if let Some(index) = scope_id {
            self.scope_info
                .scopes
                .go_to(index, self.current.module)
                .expect("no err");
        }
    }
}
