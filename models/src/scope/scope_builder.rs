use crate::{abstract_syntax_tree::soul_type::SoulType, scope::scope::{Scope, ScopeId, TypeSymbol, ValueSymbol}, soul_names::TypeModifier};

/// A builder for managing a tree of scopes.
///
/// Allows pushing and popping scopes, and inserting/resolving symbols
/// within the current scope or parent scopes.
#[derive(Debug)]
pub struct ScopeBuilder {
    /// All scopes in the tree, indexed by their `ScopeId`.
    pub scopes: Vec<Scope>,
    /// The currently active scope identifier.
    pub current: ScopeId,
}

impl ScopeBuilder {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new_root()],
            current: ScopeId::new(0),
        }
    }

    /// Push a new child scope and make it current
    pub fn push_scope(&mut self, modifier: TypeModifier, use_block: Option<SoulType>) -> ScopeId {
        let new_id = ScopeId::new(self.scopes.len());
        let parent = self.current;
        self.current_scope_mut().children.push(new_id);
        self.scopes.push(Scope::new_child(new_id, parent, modifier, use_block));
        self.current = new_id;
        new_id
    }

    /// Pop the current scope and move to parent
    pub fn pop_scope(&mut self) {
        let parent = self.current_scope().parent.expect("Cannot pop root scope");
        self.current = parent;
    }

    /// Insert a type into current scope
    pub fn insert_type(&mut self, name: String, symbol: TypeSymbol) -> Result<(), String> {
        let scope = self.current_scope_mut();
        if scope.types.contains_key(&name) {
            return Err(format!("Type '{}' already exists in current scope", name));
        }
        scope.types.insert(name, symbol);
        Ok(())
    }

    /// Insert a value (variable or function) into current scope
    pub fn insert_value(&mut self, name: String, symbol: ValueSymbol) {
        let scope = self.current_scope_mut();
        scope.values.entry(name).or_default().push(symbol);
    }

    pub fn get_current_use_block_type(&self) -> Option<&SoulType> {
        self.scopes
            .get(self.current.as_usize())?
            .use_block
            .as_ref()
    }

    pub fn get_current_modifier(&self) -> TypeModifier {
        self.scopes
            .get(self.current.as_usize())
            .map(|scope| scope.modifier)
            .unwrap_or(TypeModifier::Mut)
    }

    /// Resolves a type symbol by walking up the scope tree.
    ///
    /// Starts from the current scope and searches parent scopes until
    /// the symbol is found or the root is reached.
    pub fn resolve_type(&self, name: &str) -> Option<&TypeSymbol> {
        let mut scope_id = self.current;
        loop {
            let sc = &self.scopes[scope_id.as_usize()];
            if let Some(sym) = sc.types.get(name) {
                return Some(sym);
            }
            match sc.parent {
                Some(parent) => scope_id = parent,
                None => return None,
            }
        }
    }

    /// Resolves a value symbol by walking up the scope tree.
    ///
    /// Starts from the current scope and searches parent scopes until
    /// the symbol is found or the root is reached.
    pub fn resolve_value(&self, name: &str) -> Option<&Vec<ValueSymbol>> {
        let mut scope_id = self.current;
        loop {
            let sc = &self.scopes[scope_id.as_usize()];
            if let Some(list) = sc.values.get(name) {
                return Some(list);
            }
            match sc.parent {
                Some(parent) => scope_id = parent,
                None => return None,
            }
        }
    }

    /// Returns an immutable reference to the current scope.
    pub fn current_scope(&self) -> &Scope {
        &self.scopes[self.current.as_usize()]
    }

    /// Returns a mutable reference to the current scope.
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        &mut self.scopes[self.current.as_usize()]
    }
}