use std::collections::HashMap;

use soul_utils::{Ident, ids::FunctionId, impl_soul_ids, span::Span};

use crate::ast::Variable;

impl_soul_ids!(NodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ScopeId {
    index: usize,
    at_len: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeBuilder {
    scopes: Vec<Scope>,
    current: usize,
}
impl ScopeBuilder {
    pub fn new() -> Self {
        let global = Scope::new();
        Self {
            scopes: vec![global],
            current: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.current += 1;
        self.scopes.insert(self.current, Scope::new());
    }

    pub fn pop_scope(&mut self) {
        self.current = self.current.saturating_sub(1);
    }

    pub fn go_to(&mut self, scope_id: ScopeId) {
        let ScopeId { index, at_len } = scope_id;

        let delta_len = self.scopes.len() as i64 - at_len as i64;
        let index = (index as i64 + delta_len) as usize;
        self.current = index;
    }

    pub fn get_scope(&self, scope_id: ScopeId) -> Option<&Scope> {
        self.scopes.get(scope_id.index)
    }

    pub fn current_scope_id(&self) -> ScopeId {
        ScopeId {
            index: self.current,
            at_len: self.scopes.len(),
        }
    }

    pub fn current_scope_mut(&mut self) -> Option<&mut Scope> {
        self.scopes.get_mut(self.current)
    }

    pub fn lookup_type(&self, ident: &Ident) -> Option<ScopeTypeEntry> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.types.get(ident.as_str()) {
                return Some(*entry);
            }
        }
        None
    }

    pub fn lookup_value(&self, ident: &Ident, kind: ScopeValue) -> Option<NodeId> {
        for scope in self.scopes.iter().rev() {
            let ids = match scope.values.get(ident.as_str()) {
                Some(val) => val,
                None => continue,
            };

            if let Some(id) = ids.get(kind) {
                return Some(id);
            }
        }

        None
    }

    pub fn lookup_function(&self, ident: &Ident) -> Option<FunctionId> {
        for scope in self.scopes.iter().rev() {
            match scope.functions.get(ident.as_str()) {
                Some(val) => return Some(*val),
                None => (),
            };
        }

        None
    }
}
impl Default for ScopeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    pub values: HashMap<String, ScopeValueEntry>,
    pub types: HashMap<String, ScopeTypeEntry>,
    pub functions: HashMap<String, FunctionId>,
}
impl Scope {
    pub fn new() -> Self {
        Self::default()
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

    pub fn get_name(&self) -> &Ident {
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
