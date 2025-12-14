use std::collections::HashMap;

use crate::abstract_syntax_tree::{enum_like::{Enum, Union}, function::Function, objects::{Class, Struct, Trait}, statment::{Ident, Variable}};

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

#[derive(Debug)]
pub struct Scope {
    pub values: HashMap<Ident, Vec<ScopeValueEntry>>,
    pub types: HashMap<Ident, ScopeTypeEntry>,
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
    Function,
    Variable,
}

#[derive(Debug, Clone, Copy)]
pub struct ScopeTypeEntry {
    pub node_id: NodeId,
    pub kind: ScopeTypeEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeTypeEntryKind {
    Struct,
    Class,
    Trait,
    Union,
    Enum,
}

pub enum ScopeValueKind<'a> {
    Variable(&'a mut Variable),
    Funtion(&'a mut Function),
}
impl<'a> ScopeValueKind<'a> {
    pub fn get_id_mut(&mut self) -> &mut Option<NodeId> {

        match self {
            ScopeValueKind::Variable(variable) => &mut variable.node_id,
            ScopeValueKind::Funtion(function) => &mut function.node_id,
        } 
    }

    pub fn get_name(&self) -> &String {

        match self {
            ScopeValueKind::Variable(variable) => &variable.name,
            ScopeValueKind::Funtion(function) => &function.signature.name,
        }
    }

    pub fn to_entry_kind(&self) -> ScopeValueEntryKind {

        match self {
            ScopeValueKind::Variable(_) => ScopeValueEntryKind::Variable,
            ScopeValueKind::Funtion(_) => ScopeValueEntryKind::Function,
        }
    }
}

pub enum ScopeTypeKind<'a> {
    Struct(&'a mut Struct),
    Class(&'a mut Class),
    Trait(&'a mut Trait),
    Enum(&'a mut Enum),
    Union(&'a mut Union),
}
impl<'a> ScopeTypeKind<'a> {
    pub fn get_id_mut(&mut self) -> &mut Option<NodeId> {

        match self {
            ScopeTypeKind::Struct(ty) => &mut ty.node_id,
            ScopeTypeKind::Class(ty) => &mut ty.node_id,
            ScopeTypeKind::Trait(ty) => &mut ty.node_id,
            ScopeTypeKind::Enum(ty) => &mut ty.node_id,
            ScopeTypeKind::Union(ty) => &mut ty.node_id,
        } 
    }

    pub fn get_name(&self) -> &String {

        match self {
            ScopeTypeKind::Struct(ty) => &ty.name,
            ScopeTypeKind::Class(ty) => &ty.name,
            ScopeTypeKind::Trait(ty) => &ty.signature.name,
            ScopeTypeKind::Enum(ty) => &ty.name,
            ScopeTypeKind::Union(ty) => &ty.name,
        }
    }

    pub fn to_entry_kind(&self) -> ScopeTypeEntryKind {
        match self {
            ScopeTypeKind::Struct(_) => ScopeTypeEntryKind::Struct,
            ScopeTypeKind::Class(_) => ScopeTypeEntryKind::Class,
            ScopeTypeKind::Trait(_) => ScopeTypeEntryKind::Trait,
            ScopeTypeKind::Union(_) => ScopeTypeEntryKind::Union,
            ScopeTypeKind::Enum(_) => ScopeTypeEntryKind::Enum,
        }
    }
}
