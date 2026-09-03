use ast_model::{
    NodeId,
    block::BlockId,
    scope::{Scope, ScopeBuilder, ScopeTypeEntry, ScopeTypeEntryKind},
    statements::{Enum, Struct, Trait},
};
use soul_utils::{
    Ident,
    fault::Fault,
    soul_error_internal,
    span::{ModuleId, Span},
};

use crate::NameResolver;

mod expression;
mod import;
mod soul_type;
mod statement;

#[cfg(test)]
mod statement_tests;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_module(&mut self, id: ModuleId) {
        let global = match self.ast_modules.get(id) {
            Some(module) => module.global,
            None => {
                self.log_fault(soul_error_internal!(format!("{id:?} not found"), None));
                return;
            }
        };

        self.scope_info.add_module(id);

        let prev = self.current.module;
        self.current.module = id;
        self.current.in_global = true;
        self.push_scope(global);
        self.collect_scopeless_block(global);
        self.pop_scope();
        self.current.module = prev;
    }

    fn collect_block(&mut self, id: BlockId) {
        self.push_scope(id);
        let prev = self.current.in_global;
        self.current.in_global = false;
        self.collect_scopeless_block(id);
        self.current.in_global = prev;
        self.pop_scope();
    }

    fn collect_scopeless_block(&mut self, id: BlockId) {
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
            return;
        };
        self.scope_ids.insert(id, scope_id);
    }

    fn pop_scope(&mut self) {
        self.scope_info
            .scopes
            .pop_scope(self.current.module)
            .expect("no err");
    }

    fn declare_enum(&mut self, enum_: &Enum) {
        let name = &enum_.name;
        let entry = ScopeTypeEntry {
            span: name.span(),
            trait_parent: None,
            kind: ScopeTypeEntryKind::Enum,
            node_id: enum_.id,
        };

        self.declares
            .try_insert_enum(enum_.id, enum_, self.current.module);
        let old_entry = self.current_scope_mut().insert_types(name.as_str(), entry);

        if old_entry.is_some() {
            self.log_fault(Fault::error(
                format!("type of name {} already exists in scope", name.as_str()),
                Some(name.span()),
            ));
        }
    }

    fn declare_trait(&mut self, trait_: &Trait) {
        let name = &trait_.name;
        let scope_type = ScopeTypeEntry {
            node_id: trait_.id,
            trait_parent: None,
            span: name.span(),
            kind: ScopeTypeEntryKind::Trait,
        };

        self.declares
            .try_insert_trait(trait_.id, trait_, self.current.module);
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

    fn declare_struct(&mut self, struct_: &Struct) {
        let name = &struct_.name;
        let scope_type = ScopeTypeEntry {
            node_id: struct_.id,
            trait_parent: None,
            span: name.span(),
            kind: ScopeTypeEntryKind::Struct,
        };

        self.declares
            .try_insert_struct(struct_.id, struct_, self.current.module);
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

    fn insert_struct_alias(
        scopes: &mut ScopeBuilder,
        name: &Ident,
        span: Span,
        id: NodeId,
        module: ModuleId,
    ) -> bool {
        if scopes.flat_lookup_type(name.as_str(), module).is_some() {
            return false;
        }

        Self::static_current_scope_mut(scopes, module).insert_types(
            name.as_str(),
            ScopeTypeEntry {
                span,
                node_id: id,
                trait_parent: None,
                kind: ScopeTypeEntryKind::Struct,
            },
        );

        true
    }

    pub fn current_scope_mut(&mut self) -> &mut Scope {
        self.scope_info
            .scopes
            .current_scope_mut(self.current.module)
            .unwrap_or_else(|| panic!("{:?} has no scope", self.current.module))
    }

    /// Pushes an anonymous child scope for collecting struct members so that
    /// fields and methods of distinct structs do not collide in the module scope.
    pub(super) fn push_struct_scope(&mut self) {
        let parent = match self.scope_info.scopes.current_scope_id(self.current.module) {
            Some(val) => val,
            None => return,
        };
        if let Err(err) = self
            .scope_info
            .scopes
            .push_scope(parent, self.current.module)
        {
            self.log_fault(soul_error_internal!(err, None));
        }
    }

    /// Pops the scope pushed by [`Self::push_struct_scope`].
    pub(super) fn pop_struct_scope(&mut self) {
        if let Err(err) = self.scope_info.scopes.pop_scope(self.current.module) {
            self.log_fault(soul_error_internal!(err, None));
        }
    }
}
