use soul_ast::{
    abstract_syntax_tree::{block::Block, soul_type::NamedTupleType, statment::Ident},
    error::{SoulError, SoulErrorKind, Span},
    sementic_models::scope::{
        NodeId, NodeTag, Scope, ScopeId, ScopeTypeEntry, ScopeTypeKind, ScopeValueEntry,
        ScopeValueEntryKind, ScopeValueKind,
    },
};

use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;

mod name_collect_expression;
mod name_collect_statment;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_declarations(&mut self, block: &mut Block) {
        self.collect_block(block);
    }

    fn push_scope(&mut self, set_scope_id: &mut Option<ScopeId>) {
        self.info.scopes.push_scope();
        *set_scope_id = Some(self.info.scopes.current_scope_id());
    }

    fn pop_scope(&mut self) {
        self.info.scopes.pop_scope();
    }

    fn new_id(&mut self, tag: NodeTag) -> NodeId {
        self.ids.new_id(tag)
    }

    fn declare_parameters(&mut self, parameters: &mut NamedTupleType) {
        for (name, _ty, node_id) in &mut parameters.types {
            let id = self.new_id(NodeTag::Variable);
            *node_id = Some(id);

            self.insert_value(
                name.clone(),
                ScopeValueEntry {
                    node_id: id,
                    kind: ScopeValueEntryKind::Variable,
                },
            );
        }
    }

    fn declare_value(&mut self, mut value: ScopeValueKind) -> NodeId {
        let id = self.new_id(value.to_node_tag());
        *value.get_id_mut() = Some(id);

        self.insert_value(
            value.get_name().clone(),
            ScopeValueEntry {
                node_id: id,
                kind: value.to_entry_kind(),
            },
        );

        id
    }

    fn declare_type(&mut self, mut ty: ScopeTypeKind, span: Span) {
        let id = self.new_id(ty.to_node_tag());
        *ty.get_id_mut() = Some(id);

        let trait_parent = match ty.get_parent_id_mut() {
            Some(parent) => *parent,
            None => None,
        };

        let name = ty.get_name().clone();
        let entry = ScopeTypeEntry {
            span,
            node_id: id,
            trait_parent,
            kind: ty.to_entry_kind(),
        };

        let same_type = match self.insert_types(name, entry) {
            Some(val) => val,
            None => return,
        };

        self.log_error(SoulError::new(
            format!(
                "more then one of typename '{}' exist in this scope",
                ty.get_name().as_str(),
            ),
            SoulErrorKind::ScopeOverride(same_type.span),
            Some(span),
        ));
    }

    fn insert_value(&mut self, name: Ident, entry: ScopeValueEntry) {
        self.current_scope_mut()
            .values
            .entry(name.node)
            .or_default()
            .push(entry);
    }

    fn insert_types(&mut self, name: Ident, entry: ScopeTypeEntry) -> Option<ScopeTypeEntry> {
        self.current_scope_mut().types.insert(name.node, entry)
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.info
            .scopes
            .current_scope_mut()
            .expect("resolver has no scope")
    }
}
