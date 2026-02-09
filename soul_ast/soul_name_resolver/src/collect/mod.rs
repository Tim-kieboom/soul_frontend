use ast::{ast::{Block, NamedTupleType}, scope::{NodeId, Scope, ScopeId, ScopeValueEntry, ScopeValueEntryKind, ScopeValueKind}};

use crate::NameResolver;
mod collect_statement;
mod collect_expression;
mod collect_type;

impl<'a> NameResolver<'a> {
    pub(crate) fn collect_declarations(&mut self, block: &mut Block) {
        self.collect_block(block);
    }

    fn push_scope(&mut self, set_scope_id: &mut Option<ScopeId>) {
        self.info.scopes.push_scope();
        *set_scope_id = Some(self.info.scopes.current_scope_id())
    }

    fn pop_scope(&mut self) {
        self.info.scopes.pop_scope();
    }

    fn alloc_id(&mut self) -> NodeId {
        self.id_generator.alloc()
    }

    fn declare_parameters(&mut self, types: &mut NamedTupleType) {

        for (name, _ty, node_id) in types {
            let id = self.alloc_id();
            *node_id = Some(id);

            self.insert_value(
                name.as_str(),
                ScopeValueEntry { 
                    node_id: id, 
                    kind: ScopeValueEntryKind::Variable, 
                }
            )
        }
    }

    fn declare_value(&mut self, mut value: ScopeValueKind) -> NodeId {
        let id = self.alloc_id();
        *value.get_id_mut() = Some(id);

        self.insert_value(
            value.get_name().as_str(), 
            ScopeValueEntry { 
                node_id: id, 
                kind: value.to_entry_kind(),
            },
        );
        id
    }

    fn insert_value<S: Into<String>>(&mut self, name: S, entry: ScopeValueEntry) {
        self.current_scope_mut()
            .values
            .entry(name.into())
            .or_default()
            .push(entry)
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.info
            .scopes
            .current_scope_mut()
            .expect("resolver has no scope")
    }
}