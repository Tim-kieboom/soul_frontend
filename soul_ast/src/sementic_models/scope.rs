use std::collections::HashMap;

use soul_utils::Span;

use crate::{
    abstract_syntax_tree::{
        enum_like::{Enum, Union, UnionVariant},
        function::Function,
        objects::{Class, Field, Struct, Trait},
        soul_type::{GenericDeclare, GenericDeclareKind},
        statment::{Ident, Variable},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(u32, NodeTag);
impl NodeId {
    pub fn display(&self) -> String {
        format!("{}", self.0)
    }

    pub fn tag(&self) -> NodeTag {
        self.1
    }
}
impl Ord for NodeId{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl PartialOrd for NodeId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

pub struct NodeIdGenerator {
    next: u32,
}
impl NodeIdGenerator {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn new_id(&mut self, tag: NodeTag) -> NodeId {
        let id = self.next;
        self.next += 1;
        NodeId(id, tag)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum NodeTag {
    Enum,
    Field,
    Class,
    Trait,
    Union,
    Struct,
    Variable,
    Function,
    UnionVariant,
    GenricDeclare,
}
impl NodeTag {
    pub fn is_traitable_type(&self) -> bool {
        match self {
            NodeTag::Enum 
            | NodeTag::Class 
            | NodeTag::Trait 
            | NodeTag::Union 
            | NodeTag::Struct => true,
            
            NodeTag::Field 
            | NodeTag::Variable
            | NodeTag::Function
            | NodeTag::UnionVariant
            | NodeTag::GenricDeclare => false,
        }
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    pub values: HashMap<String, Vec<ScopeValueEntry>>,
    pub types: HashMap<String, ScopeTypeEntry>,
}
impl Scope {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            types: HashMap::new(),
        }
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
    Enum,
    Class,
    Trait,
    Union,
    Struct,
    LifeTime,
    GenericType,
    UnionVariant,
    GenericExpression,
}

pub enum ScopeValueKind<'a> {
    Field(&'a mut Field),
    Variable(&'a mut Variable),
    Function(&'a mut Function),
}
impl<'a> ScopeValueKind<'a> {
    pub fn get_id_mut(&mut self) -> &mut Option<NodeId> {
        match self {
            ScopeValueKind::Field(field) => &mut field.node_id,
            ScopeValueKind::Variable(variable) => &mut variable.node_id,
            ScopeValueKind::Function(function) => &mut function.node_id,
        }
    }

    pub fn get_name(&self) -> &Ident {
        match self {
            ScopeValueKind::Field(field) => &field.name,
            ScopeValueKind::Variable(variable) => &variable.name,
            ScopeValueKind::Function(function) => &function.signature.node.name,
        }
    }

    pub fn to_entry_kind(&self) -> ScopeValueEntryKind {
        match self {
            ScopeValueKind::Field(_) => ScopeValueEntryKind::Field,
            ScopeValueKind::Variable(_) => ScopeValueEntryKind::Variable,
            ScopeValueKind::Function(_) => ScopeValueEntryKind::Function,
        }
    }

    pub fn to_node_tag(&self) -> NodeTag {
        match self {
            ScopeValueKind::Field(_) => NodeTag::Field,
            ScopeValueKind::Variable(_) => NodeTag::Variable,
            ScopeValueKind::Function(_) => NodeTag::Function,
        }
    }
}

pub enum ScopeTypeKind<'a> {
    Struct(&'a mut Struct),
    Class(&'a mut Class),
    Trait(&'a mut Trait),
    Enum(&'a mut Enum),
    Union(&'a mut Union),
    UnionVariant{ty: &'a mut UnionVariant, trait_id: NodeId},
    GenricDeclare(&'a mut GenericDeclare),
}
impl<'a> ScopeTypeKind<'a> {
    pub fn get_id_mut(&mut self) -> &mut Option<NodeId> {
        match self {
            ScopeTypeKind::GenricDeclare(ty) => &mut ty.node_id,
            ScopeTypeKind::UnionVariant{ty, ..} => &mut ty.node_id,
            ScopeTypeKind::Struct(ty) => &mut ty.node_id,
            ScopeTypeKind::Class(ty) => &mut ty.node_id,
            ScopeTypeKind::Trait(ty) => &mut ty.node_id,
            ScopeTypeKind::Enum(ty) => &mut ty.node_id,
            ScopeTypeKind::Union(ty) => &mut ty.node_id,
        }
    }

    pub fn get_parent_id_mut(&mut self) -> Option<&mut Option<NodeId>> {
        match self {
            ScopeTypeKind::UnionVariant{ty, ..} => Some(&mut ty.node_id),
            
            ScopeTypeKind::GenricDeclare(_) 
            | ScopeTypeKind::Trait(_) 
            | ScopeTypeKind::Struct(_) 
            | ScopeTypeKind::Class(_) 
            | ScopeTypeKind::Union(_) 
            | ScopeTypeKind::Enum(_) => None,
        }
    }

    pub fn get_name(&self) -> &Ident {
        match self {
            ScopeTypeKind::GenricDeclare(ty) => match &ty.kind {
                GenericDeclareKind::Type { name, .. } => &name,
                GenericDeclareKind::Lifetime(ident) => &ident,
                GenericDeclareKind::Expression { name, .. } => &name,
            },
            ScopeTypeKind::UnionVariant{ty, ..} => &ty.name,
            ScopeTypeKind::Struct(ty) => &ty.name,
            ScopeTypeKind::Class(ty) => &ty.name,
            ScopeTypeKind::Trait(ty) => &ty.signature.name,
            ScopeTypeKind::Enum(ty) => &ty.name,
            ScopeTypeKind::Union(ty) => &ty.name,
        }
    }

    pub fn to_entry_kind(&self) -> ScopeTypeEntryKind {
        match self {
            ScopeTypeKind::GenricDeclare(ty) => match &ty.kind {
                GenericDeclareKind::Type { .. } => ScopeTypeEntryKind::GenericType,
                GenericDeclareKind::Lifetime(_) => ScopeTypeEntryKind::LifeTime,
                GenericDeclareKind::Expression { .. } => ScopeTypeEntryKind::GenericExpression,
            },
            ScopeTypeKind::UnionVariant{..} => ScopeTypeEntryKind::UnionVariant,
            ScopeTypeKind::Struct(_) => ScopeTypeEntryKind::Struct,
            ScopeTypeKind::Class(_) => ScopeTypeEntryKind::Class,
            ScopeTypeKind::Trait(_) => ScopeTypeEntryKind::Trait,
            ScopeTypeKind::Union(_) => ScopeTypeEntryKind::Union,
            ScopeTypeKind::Enum(_) => ScopeTypeEntryKind::Enum,
        }
    }

    pub fn to_node_tag(&self) -> NodeTag {
        match self {
            ScopeTypeKind::Enum(_) => NodeTag::Enum,
            ScopeTypeKind::Trait(_) => NodeTag::Trait,
            ScopeTypeKind::Class(_) => NodeTag::Class,
            ScopeTypeKind::Union(_) => NodeTag::Union,
            ScopeTypeKind::Struct(_) => NodeTag::Struct,
            ScopeTypeKind::UnionVariant{..} => NodeTag::UnionVariant,
            ScopeTypeKind::GenricDeclare(_) => NodeTag::GenricDeclare,
        }
    }
}
