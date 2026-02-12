use std::collections::HashMap;

use soul_utils::{Ident, span::Span, vec_map::VecMapIndex};

use crate::ast::{Function, Variable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(u32);
impl NodeId {
    pub fn internal_new(value: u32) -> Self {
        Self(value)
    }
    pub fn display(&self) -> String {
        format!("{}", self.0)
    }
    pub fn write(&self, sb: &mut String) {
        use std::fmt::Write;
        write!(sb, "{}", self.0).expect("should not give write error")
    }
}
impl VecMapIndex for NodeId {
    fn new_index(value: usize) -> Self {
        Self(value as u32)
    }

    fn index(&self) -> usize {
        self.0 as usize
    }
}

pub struct NodeIdGenerator(u32);
impl NodeIdGenerator {
    pub fn from_last(last: NodeId) -> Self {
        Self(last.0+1)
    }

    pub fn new() -> Self {
        Self(0)
    }

    pub fn alloc(&mut self) -> NodeId {
        let node = NodeId::internal_new(self.0);
        self.0 += 1;
        node
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

    pub fn lookup_value(&self, ident: &Ident, kind: ScopeValueEntryKind) -> Option<NodeId> {
        
        for scope in self.scopes.iter().rev() {
            
            let ids = match scope.values.get(ident.as_str()) {
                Some(val) => val,
                None => continue,
            };

            for id in ids {
                if id.kind == kind {
                    return Some(id.node_id);
                }
            }
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
