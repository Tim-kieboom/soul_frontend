use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScopeId(usize);

#[derive(Debug, Clone)]
pub enum TypeSymbol {
    Struct(String),
    Class(String),
    Trait(String),
    Enum(String),
    Union(String),
    TypeDef { new_type: String, of_type: String },
}

#[derive(Debug, Clone)]
pub enum ValueSymbol {
    Variable(todo!()),
    Function(todo!()),
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub children: Vec<ScopeId>,

    pub values: HashMap<String, Vec<ValueSymbol>>,
    pub types: HashMap<String, TypeSymbol>,
}

impl ScopeId {
    pub(crate) fn new(value: usize) -> Self {
        Self(value)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl Scope {
    pub fn new_root() -> Self {
        Self {
            id: ScopeId(0),
            parent: None,
            children: vec![],
            values: HashMap::new(),
            types: HashMap::new(),
        }
    }

    pub fn new_child(id: ScopeId, parent: ScopeId) -> Self {
        Self {
            id,
            parent: Some(parent),
            children: vec![],
            values: HashMap::new(),
            types: HashMap::new(),
        }
    }
}
