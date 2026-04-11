use std::collections::HashMap;

use soul_utils::{Ident, ids::FunctionId, impl_soul_ids, span::Span, vec_map::VecMapIndex};

use crate::ast::Variable;

impl_soul_ids!(NodeId, ScopeId);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeBuilder {
    scopes: Vec<Scope>,
    current: ScopeId,
    next: ScopeId,
}
impl ScopeBuilder {
    pub fn new() -> Self {
        let root = ScopeId::new_index(0);
        let next = ScopeId::new_index(1);
        Self {
            scopes: vec![Scope::new_global(root)],
            current: root,
            next,
        }
    }

    pub fn push_scope(&mut self, parent: ScopeId) {
        self.current = self.next;
        self.next.0 += 1;
        self.scopes
            .insert(self.current.0, Scope::new_child(self.current, parent));
    }

    pub fn pop_scope(&mut self) {
        self.current = self.scopes[self.current.0].parent.unwrap_or(self.current);
    }

    pub fn go_to(&mut self, scope_id: ScopeId) {
        self.current = scope_id;
    }

    pub fn get_scope(&self, scope_id: ScopeId) -> Option<&Scope> {
        self.scopes.get(scope_id.0)
    }

    pub fn current_scope_id(&self) -> ScopeId {
        self.current
    }

    pub fn current_scope_mut(&mut self) -> Option<&mut Scope> {
        self.scopes.get_mut(self.current.0)
    }

    pub fn lookup_type(&self, ident: &Ident) -> Option<ScopeTypeEntry> {
        for scope in self.scope_iter() {
            if let Some(ScopeEntry { types, .. }) = scope.entries.get(ident.as_str()) {
                return *types;
            }
        }
        None
    }

    pub fn lookup_value(&self, ident: &Ident, kind: ScopeValue) -> Option<NodeId> {
        for scope in self.scope_iter() {
            let ids = match scope.entries.get(ident.as_str()) {
                Some(ScopeEntry {
                    values: Some(val), ..
                }) => val,
                _ => continue,
            };

            if let Some(id) = ids.get(kind) {
                return Some(id);
            }
        }

        None
    }

    pub fn flat_lookup_value(&self, ident: &Ident, kind: ScopeValue) -> Option<NodeId> {
        let scope = self.scopes.get(self.current.0)?;
        let ids = scope.entries.get(ident.as_str())?.values.as_ref()?;

        ids.get(kind)
    }

    pub fn lookup_function(&self, ident: &Ident) -> Option<FunctionId> {
        for scope in self.scope_iter() {
            if let Some(ScopeEntry { function, .. }) = scope.entries.get(ident.as_str()) {
                return *function;
            };
        }

        None
    }

    fn scope_iter<'a>(&'a self) -> ScopeIterator<'a> {
        ScopeIterator::new(&self.scopes, self.current)
    }
}

impl Default for ScopeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

struct ScopeIterator<'a> {
    scopes: &'a Vec<Scope>,
    current: Option<ScopeId>,
}
impl<'a> ScopeIterator<'a> {
    fn new(scopes: &'a Vec<Scope>, current: ScopeId) -> Self {
        Self {
            scopes,
            current: Some(current),
        }
    }
}
impl<'a> Iterator for ScopeIterator<'a> {
    type Item = &'a Scope;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.current?.0;
        let scope = self.scopes.get(index)?;
        self.current = scope.parent;
        Some(scope)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScopeEntry {
    function: Option<FunctionId>,
    values: Option<ScopeValueEntry>,
    types: Option<ScopeTypeEntry>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    entries: HashMap<String, ScopeEntry>,
}
impl Scope {
    pub fn new_global(id: ScopeId) -> Self {
        Self {
            id,
            parent: None,
            entries: HashMap::new(),
        }
    }

    pub fn new_child(id: ScopeId, parent: ScopeId) -> Self {
        let mut this = Self {
            id,
            parent: Some(parent),
            entries: HashMap::new(),
        };
        this.parent = Some(parent);
        this
    }

    pub fn get_entries(&self) -> &HashMap<String, ScopeEntry> {
        &self.entries
    }

    pub fn insert_function(&mut self, name: &str, id: FunctionId) -> Option<FunctionId> {
        self.get_mut_entry(name)?.function.replace(id)
    }

    pub fn insert_types(&mut self, name: &str, id: ScopeTypeEntry) -> Option<ScopeTypeEntry> {
        self.get_mut_entry(name)?.types.replace(id)
    }

    pub fn insert_value(&mut self, name: &str, kind: ScopeValue, id: NodeId) -> Option<NodeId> {
        if self.get_mut_entry(name)?.values.is_none() {
            self.get_mut_entry(name)?.values = Some(ScopeValueEntry::default());
        }

        let values = &mut self.get_mut_entry(name)?.values.as_mut().unwrap();
        values.insert(kind, id)
    }

    fn get_mut_entry(&mut self, name: &str) -> Option<&mut ScopeEntry> {
        if !self.entries.contains_key(name) {
            self.entries.insert(name.to_string(), ScopeEntry::default());
        }

        self.entries.get_mut(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScopeValue {
    Field = 0,
    Variable = 1,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScopeValueEntry {
    kinds: [Option<NodeId>; 2],
}
impl ScopeValueEntry {
    pub fn insert(&mut self, kind: ScopeValue, id: NodeId) -> Option<NodeId> {
        let index = kind as usize;
        debug_assert!(
            index < self.kinds.len(),
            "should probebly increase stack array len to amount of variants of ScopeValueEntryKind",
        );
        self.kinds[index].replace(id)
    }

    pub fn get(&self, kind: ScopeValue) -> Option<NodeId> {
        let index = kind as usize;
        debug_assert!(
            index < self.kinds.len(),
            "should probebly increase stack array len to amount of variants of ScopeValueEntryKind",
        );
        self.kinds[index]
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ScopeTypeEntry {
    pub span: Span,
    pub node_id: NodeId,
    pub kind: ScopeTypeEntryKind,
    pub trait_parent: Option<NodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScopeTypeEntryKind {
    Struct,
    LifeTime,
    GenericType,
}

pub enum ScopeValueKind<'a> {
    Variable(&'a mut Variable),
}
impl<'a> ScopeValueKind<'a> {
    pub fn get_id_mut(&mut self) -> &mut Option<NodeId> {
        match self {
            ScopeValueKind::Variable(variable) => &mut variable.node_id,
        }
    }

    pub fn get_ident(&self) -> &Ident {
        match self {
            ScopeValueKind::Variable(variable) => &variable.name,
        }
    }

    pub fn to_entry_kind(&self) -> ScopeValue {
        match self {
            ScopeValueKind::Variable(_) => ScopeValue::Variable,
        }
    }
}
