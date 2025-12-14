use models::{
    abstract_syntax_tree::{soul_type::NamedTupleType, statment::Ident},
    error::{SoulError, SoulErrorKind, Span},
    sementic_models::scope::{NodeId, NodeIdGenerator, Scope, ScopeTypeEntry, ScopeTypeKind, ScopeValueEntry, ScopeValueEntryKind, ScopeValueKind},
};

use crate::steps::sementic_analyser::sementic_fault::SementicFault;

pub struct NameResolver {
    pub scopes: Vec<Scope>,
    pub ids: NodeIdGenerator,
    pub errors: Vec<SementicFault>,

    pub current_function: Option<NodeId>,
    pub loop_depth: usize,
}

impl NameResolver {
    pub fn new() -> Self {
        let global = Scope::new();
        Self {
            loop_depth: 0,
            errors: vec![],
            scopes: vec![global],
            current_function: None,
            ids: NodeIdGenerator::new(),
        }
    }

    pub fn consume_faults(self) -> Vec<SementicFault> {
        self.errors
    }

    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub(crate) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn lookup_variable(&self, ident: &Ident) -> Option<NodeId> {
        for scope in self.scopes.iter().rev() {
            if let Some(ids) = scope.values.get(ident) {
                return ids.last().map(|el| el.node_id.clone());
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

    pub(crate) fn lookup_type(&self, ident: &Ident) -> Option<ScopeTypeEntry> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.types.get(ident) {
                return Some(*entry);
            }
        }
        None
    }

    pub(crate) fn declare_parameters(&mut self, parameters: &mut NamedTupleType) {
        for (name, _type, node_id) in &mut parameters.types {
            let id = self.new_id();
            *node_id = Some(id);
            self.insert_value(name.clone(), ScopeValueEntry{node_id: id, kind: ScopeValueEntryKind::Variable});
        }
    }

    pub(crate) fn declare_value(&mut self, mut value: ScopeValueKind) -> NodeId {
        let id = self.new_id();
        *value.get_id_mut() = Some(id);

        self.insert_value(value.get_name().clone(), ScopeValueEntry{node_id: id, kind: value.to_entry_kind()});
        id
    }

    pub(crate) fn declare_type(&mut self, mut ty: ScopeTypeKind, span: Span) {
        const IS_UNIQUE: bool = true;
        let id = self.new_id();
        *ty.get_id_mut() = Some(id);

        if self.insert_types(ty.get_name().clone(), ScopeTypeEntry { node_id: id, kind: ty.to_entry_kind() }) != IS_UNIQUE {
            self.log_error(SoulError::new(
                format!(
                    "more then one of typename '{}' exist in this scope",
                    ty.get_name()
                ),
                SoulErrorKind::ScopeError,
                Some(span),
            ));
        }
    }

    pub(crate) fn log_error(&mut self, err: SoulError) {
        self.errors.push(SementicFault::error(err));
    }

    fn insert_value(&mut self, name: Ident, entry: ScopeValueEntry) {
        self.current_scope_mut()
            .values
            .entry(name)
            .or_default()
            .push(entry);
    }

    fn insert_types(&mut self, name: Ident, entry: ScopeTypeEntry) -> bool {
        let is_non_overriding_insert = self.current_scope_mut()
            .types
            .insert(name, entry)
            .is_none();

        is_non_overriding_insert
    }

    fn new_id(&mut self) -> NodeId {
        self.ids.new_id()
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("resolver has no scope")
    }
}
