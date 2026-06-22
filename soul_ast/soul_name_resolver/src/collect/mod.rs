
use ast_model::{block::BlockId, scope::{Scope, ScopeTypeEntry, ScopeTypeEntryKind}, statements::{Enum, StatementId, Struct, Trait}};
use soul_utils::{fault::Fault, soul_error_internal, span::ModuleId};

use crate::NameResolver;

mod statement;
mod import;

impl<'a> NameResolver<'a> {
    pub(crate) fn collect_module(&mut self, id: ModuleId) {
        let global = match self.ast_modules.get(id) {
            Some(module) => module.global,
            None => {
                self.log_fault(soul_error_internal!(format!("{id:?} not found"), None));
                return
            }
        };

        let prev = self.current.module;
        self.current.module = id;
        self.current.in_global = true;
        self.collect_block(global);
        self.current.module = prev;
    }

    pub(super) fn collect_block(&mut self, id: BlockId) {
        self.push_scope(id);
        self.collect_scopeless_block(id);
        self.pop_scope();
    }

    pub(crate) fn collect_scopeless_block(&mut self, id: BlockId) {
        let node_id = self.alloc_node();
        self.nodes.blocks.insert(id, node_id);
        
        for statement in &self.store.blocks[id].statements {
            self.collect_statement(*statement);
        }
    }

    fn push_scope(&mut self, id: BlockId) {
        let parent = match self.scope_info.scopes.current_scope_id(self.current.module) {
            Some(val) => val,
            None => {
                self.log_fault(soul_error_internal!(
                    format!(
                        "push_scope current_scope_id is None {:?}",
                        self.current.module
                    ),
                    None
                ));
                return;
            }
        };
        self.scope_info
            .scopes
            .push_scope(parent, self.current.module)
            .expect("no err");
        
        let Some(scope_id) = self.scope_info.scopes.current_scope_id(self.current.module) else {
            return
        };
        self.scope_ids.insert(id, scope_id);
    }

    fn pop_scope(&mut self) {
        self.scope_info
            .scopes
            .pop_scope(self.current.module)
            .expect("no err");
    }

    fn declare_enum(&mut self, id: StatementId, enum_: &Enum) {
        let node_id = self.alloc_node();
        self.nodes.statements.insert(id, node_id);

        let name = &enum_.name;
        let entry = ScopeTypeEntry {
            node_id,
            span: name.span(),
            trait_parent: None,
            kind: ScopeTypeEntryKind::Enum,
        };

        let old_entry = self.current_scope_mut()
            .insert_types(name.as_str(), entry);

        if old_entry.is_some() {
            self.log_fault(Fault::error(
                format!("type of name {} already exists in scope", name.as_str()),
                Some(name.span()),
            ));
        }
    }

    fn declare_trait(&mut self, id: StatementId, trait_: &Trait) {
        let node_id = self.alloc_node();
        self.nodes.statements.insert(id, node_id);

        let name = &trait_.name;
        let scope_type = ScopeTypeEntry {
            node_id,
            trait_parent: None,
            span: name.span(),
            kind: ScopeTypeEntryKind::Trait,
        };

        let old_entry = self
            .current_scope_mut()
            .insert_types(name.as_str(), scope_type);

        if old_entry.is_some() {
            self.log_fault(Fault::error(
                format!("type of name {} already exists in scope", name.as_str()),
                Some(name.span()),
            ));
        }
    }

    fn declare_struct(&mut self, id: StatementId, struct_: &Struct) {
        let node_id = self.alloc_node();
        self.nodes.statements.insert(id, node_id);

        let name = &struct_.name;
        let scope_type = ScopeTypeEntry {
            node_id,
            trait_parent: None,
            span: name.span(),
            kind: ScopeTypeEntryKind::Struct,
        };

        let old_entry = self
            .current_scope_mut()
            .insert_types(name.as_str(), scope_type);

        if old_entry.is_some() {
            self.log_fault(Fault::error(
                format!("type of name {} already exists in scope", name.as_str()),
                Some(name.span()),
            ));
        }
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scope_info
            .scopes
            .current_scope_mut(self.current.module)
            .expect("resolver has no scope")
    }
}