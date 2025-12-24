use models::{
    abstract_syntax_tree::{soul_type::NamedTupleType, statment::Ident},
    error::{SoulError, SoulErrorKind, Span},
    sementic_models::scope::{
        NodeId, NodeIdGenerator, Scope, ScopeTypeEntry, ScopeTypeKind, ScopeValueEntry,
        ScopeValueEntryKind, ScopeValueKind,
    },
};

use crate::steps::sementic_analyser::sementic_fault::SementicFault;

pub struct NameResolver {
    pub scopes: Vec<Scope>,
    pub ids: NodeIdGenerator,
    pub errors: Vec<SementicFault>,

    pub current_function: Option<NodeId>,
}

impl NameResolver {
    pub fn new() -> Self {
        let global = Scope::new();
        Self {
            errors: vec![],
            scopes: vec![global],
            current_function: None,
            ids: NodeIdGenerator::new(),
        }
    }

    pub fn consume_faults(self) -> Vec<SementicFault> {
        self.errors
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn lookup_variable(&self, ident: &Ident) -> Option<NodeId> {
        for scope in self.scopes.iter().rev() {
            if let Some(ids) = scope.values.get(ident) {
                return ids.last().map(|el| el.node_id);
            }
        }

        None
    }

    pub(crate) fn lookup_function_candidates(&self, ident: &Ident) -> Vec<NodeId> {
        let mut candidates = Vec::new();

        for scope in self.scopes.iter().rev() {
            if let Some(ids) = scope.values.get(ident) {
                for id in ids {
                    if id.kind == ScopeValueEntryKind::Function {
                        candidates.push(id.node_id);
                    }
                }
            }
        }

        candidates
    }

    pub(super) fn lookup_type(&self, ident: &Ident) -> Option<ScopeTypeEntry> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.types.get(ident) {
                return Some(*entry);
            }
        }
        None
    }

    pub(super) fn declare_parameters(&mut self, parameters: &mut NamedTupleType, span: Span) {
        for (name, ty, node_id) in &mut parameters.types {
            self.resolve_type(ty, span);

            let id = self.new_id();
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

    pub(super) fn declare_value(&mut self, mut value: ScopeValueKind) -> NodeId {
        let id = self.new_id();
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

    pub(super) fn declare_type(&mut self, mut ty: ScopeTypeKind, span: Span) {
        let id = self.new_id();
        *ty.get_id_mut() = Some(id);

        let name = ty.get_name().clone();
        let entry = ScopeTypeEntry {
            node_id: id,
            span,
            kind: ty.to_entry_kind(),
        };

        let same_type = match self.insert_types(name, entry) {
            Some(val) => val,
            None => return,
        };

        self.log_error(SoulError::new(
            format!(
                "more then one of typename '{}' exist in this scope",
                ty.get_name(),
            ),
            SoulErrorKind::ScopeOverride(same_type.span),
            Some(span),
        ));
    }

    pub(super) fn log_error(&mut self, err: SoulError) {
        self.errors.push(SementicFault::error(err));
    }

    fn insert_value(&mut self, name: Ident, entry: ScopeValueEntry) {
        self.current_scope_mut()
            .values
            .entry(name)
            .or_default()
            .push(entry);
    }

    fn insert_types(&mut self, name: Ident, entry: ScopeTypeEntry) -> Option<ScopeTypeEntry> {
        self.current_scope_mut().types.insert(name, entry)
    }

    fn new_id(&mut self) -> NodeId {
        self.ids.new_id()
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("resolver has no scope")
    }
}
