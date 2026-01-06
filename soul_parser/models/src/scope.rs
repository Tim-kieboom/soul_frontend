use std::collections::HashMap;

use soul_utils::{Ident, span::Span};

use crate::ast::{Function, GenericDeclare, GenericDeclareKind, Variable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(u32);
impl NodeId {
    pub fn display(&self) -> String {
        format!("{}", self.0)
    }
}

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

    pub fn lookup_variable(&self, ident: &Ident) -> Option<NodeId> {
        for scope in self.scopes.iter().rev() {
            if let Some(ids) = scope.values.get(ident.as_str()) {
                return ids.last().map(|el| el.node_id);
            }
        }

        None
    }

    pub fn lookup_function_candidates(&self, ident: &Ident) -> Vec<NodeId> {
        let mut candidates = Vec::new();

        for scope in self.scopes.iter().rev() {
            if let Some(ids) = scope.values.get(ident.as_str()) {
                for id in ids {
                    if id.kind == ScopeValueEntryKind::Function {
                        candidates.push(id.node_id);
                    }
                }
            }
        }

        candidates
    }
}
impl Default for ScopeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    pub values: HashMap<String, Vec<ScopeValueEntry>>,
    pub types: HashMap<String, ScopeTypeEntry>,
}
impl Scope {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ScopeValueEntry {
    pub node_id: NodeId,
    pub kind: ScopeValueEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScopeValueEntryKind {
    Field,
    Function,
    Variable,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ScopeTypeEntry {
    pub span: Span,
    pub node_id: NodeId,
    pub trait_parent: Option<NodeId>,
    pub kind: ScopeTypeEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScopeTypeEntryKind {
    LifeTime,
    GenericType,
}

pub enum ScopeValueKind<'a> {
    Variable(&'a mut Variable),
    Function(&'a mut Function),
}
impl<'a> ScopeValueKind<'a> {
    pub fn get_id_mut(&mut self) -> &mut Option<NodeId> {
        match self {
            ScopeValueKind::Variable(variable) => &mut variable.node_id,
            ScopeValueKind::Function(function) => &mut function.node_id,
        }
    }

    pub fn get_name(&self) -> &Ident {
        match self {
            ScopeValueKind::Variable(variable) => &variable.name,
            ScopeValueKind::Function(function) => &function.signature.node.name,
        }
    }

    pub fn to_entry_kind(&self) -> ScopeValueEntryKind {
        match self {
            ScopeValueKind::Variable(_) => ScopeValueEntryKind::Variable,
            ScopeValueKind::Function(_) => ScopeValueEntryKind::Function,
        }
    }
}

pub enum ScopeTypeKind<'a> {
    GenricDeclare(&'a mut GenericDeclare),
}
impl<'a> ScopeTypeKind<'a> {
    pub fn get_id_mut(&mut self) -> &mut Option<NodeId> {
        match self {
            ScopeTypeKind::GenricDeclare(ty) => &mut ty.node_id,
        }
    }

    pub fn get_parent_id_mut(&mut self) -> Option<&mut Option<NodeId>> {
        None
    }

    pub fn get_name(&self) -> &Ident {
        match self {
            ScopeTypeKind::GenricDeclare(ty) => match &ty.kind {
                GenericDeclareKind::Type { name, .. } => name,
                GenericDeclareKind::Lifetime(name) => name,
            },
        }
    }

    pub fn to_entry_kind(&self) -> ScopeTypeEntryKind {
        match self {
            ScopeTypeKind::GenricDeclare(ty) => match &ty.kind {
                GenericDeclareKind::Type { .. } => ScopeTypeEntryKind::GenericType,
                GenericDeclareKind::Lifetime(_) => ScopeTypeEntryKind::LifeTime,
            },
        }
    }
}
