use std::collections::HashMap;

use crate::abstract_syntax_tree::{expression::Expression, function::Function, soul_type::SoulType, spanned::Spanned, statment::{Ident, Variable}};

/// An identifier for a scope in the scope tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScopeId(usize);

/// A type symbol that can be stored in a scope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TypeSymbol {
    Struct(String),
    Class(String),
    Trait(String),
    Enum(String),
    Union(String),
    /// A type definition alias.
    TypeDef { new_type: String, of_type: String },
}

/// A value symbol that can be stored in a scope (variable or function).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ValueSymbol {
    Variable(Variable),
    /// One or more function definitions with the same name (overloading).
    Functions(Vec<Spanned<Function>>),
}

/// A lexical scope containing symbols (variables, functions, types).
///
/// Scopes form a tree structure where child scopes can access symbols from
/// parent scopes.
#[derive(Debug, Clone)]
pub struct Scope {
    /// The unique identifier for this scope.
    pub id: ScopeId,
    /// The parent scope, if any. `None` for the root scope.
    pub parent: Option<ScopeId>,
    /// List of child scope identifiers.
    pub children: Vec<ScopeId>,

    /// Map of value names to their symbols (variables and functions).
    pub values: HashMap<String, Vec<ValueSymbol>>,
    /// Map of type names to their type symbols.
    pub types: HashMap<String, TypeSymbol>,
}

impl ValueSymbol {
    pub fn new_variable(name: Ident, ty: SoulType, initialize_value: Option<Expression>) -> Self {
        Self::Variable(Variable{name, ty, initialize_value})
    }
}

impl ScopeId {
    pub const GLOBAL: Self = Self::new(0); 

    pub(crate) const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the underlying `usize` value.
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl Scope {
    /// Creates a new root scope with id 0 and no parent.
    pub fn new_root() -> Self {
        Self {
            id: ScopeId(0),
            parent: None,
            children: vec![],
            values: HashMap::new(),
            types: HashMap::new(),
        }
    }

    /// Creates a new child scope with the given id and parent.
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
