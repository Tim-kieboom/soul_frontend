use std::collections::{HashMap};

use crate::{abstract_syntax_tree::{enum_like::{Enum, Union}, function::Function, objects::{Class, Field, Struct, Trait}, soul_type::{GenericDeclare, GenericDeclareKind}, statment::{Ident, Variable}}, error::Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(u32);
impl NodeId {
    pub fn display(&self) -> String {
        format!("{}", self.0)
    }
}

pub struct NodeIdGenerator {
    next: u32,
}
impl NodeIdGenerator {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn new_id(&mut self) -> NodeId {
        let id = self.next;
        self.next += 1;
        NodeId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ScopeId{index: usize, at_len: usize}

#[derive(Debug)]
pub struct ScopeBuilder {
    scopes: Vec<Scope>,
    current: usize,
}
impl ScopeBuilder {
    pub fn new() -> Self {
        let global = Scope::new();
        Self { scopes: vec![global], current: 0 }    
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.insert(self.current+1, Scope::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.current -= 1
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

#[derive(Debug)]
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

#[derive(Debug, Clone, Copy)]
pub struct ScopeValueEntry {
    pub node_id: NodeId,
    pub kind: ScopeValueEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeValueEntryKind {
    Field,
    Function,
    Variable,
}

#[derive(Debug, Clone, Copy)]
pub struct ScopeTypeEntry { 
    pub span: Span,
    pub node_id: NodeId,
    pub kind: ScopeTypeEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeTypeEntryKind {
    Enum,
    Class,
    Trait,
    Union,
    Struct,
    LifeTime,
    GenericType,
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
}

pub enum ScopeTypeKind<'a> {
    Struct(&'a mut Struct),
    Class(&'a mut Class),
    Trait(&'a mut Trait),
    Enum(&'a mut Enum),
    Union(&'a mut Union),
    GenricDeclare(&'a mut GenericDeclare)
}
impl<'a> ScopeTypeKind<'a> {
    pub fn get_id_mut(&mut self) -> &mut Option<NodeId> {

        match self {
            ScopeTypeKind::GenricDeclare(ty) => &mut ty.node_id,
            ScopeTypeKind::Struct(ty) => &mut ty.node_id,
            ScopeTypeKind::Class(ty) => &mut ty.node_id,
            ScopeTypeKind::Trait(ty) => &mut ty.node_id,
            ScopeTypeKind::Enum(ty) => &mut ty.node_id,
            ScopeTypeKind::Union(ty) => &mut ty.node_id,
        } 
    }

    pub fn get_name(&self) -> &Ident {

        match self {
            ScopeTypeKind::GenricDeclare(ty) => match &ty.kind {
                GenericDeclareKind::Type{name, ..} => &name,
                GenericDeclareKind::Lifetime(ident) => &ident,
                GenericDeclareKind::Expression{name, ..} => &name,
            },
            ScopeTypeKind::Struct(ty) => &ty.name,
            ScopeTypeKind::Class(ty) => &ty.name,
            ScopeTypeKind::Trait(ty) => &ty.signature.name,
            ScopeTypeKind::Enum(ty) => &ty.name,
            ScopeTypeKind::Union(ty) => &ty.name,
        }
    }

    pub fn to_entry_kind(&self) -> ScopeTypeEntryKind {
        match self {
            ScopeTypeKind::GenricDeclare(ty) => {
                match &ty.kind {
                    GenericDeclareKind::Type{..} => ScopeTypeEntryKind::GenericType,
                    GenericDeclareKind::Lifetime(_) => ScopeTypeEntryKind::LifeTime,
                    GenericDeclareKind::Expression{..} => ScopeTypeEntryKind::GenericExpression,
                }
            },
            ScopeTypeKind::Struct(_) => ScopeTypeEntryKind::Struct,
            ScopeTypeKind::Class(_) => ScopeTypeEntryKind::Class,
            ScopeTypeKind::Trait(_) => ScopeTypeEntryKind::Trait,
            ScopeTypeKind::Union(_) => ScopeTypeEntryKind::Union,
            ScopeTypeKind::Enum(_) => ScopeTypeEntryKind::Enum,
        }
    }
}
