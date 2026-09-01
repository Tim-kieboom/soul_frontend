use std::collections::HashMap;

use soul_utils::{
    FunctionId, Ident,
    collections::vec_map::VecMap,
    ids::IdGenerator,
    impl_soul_ids,
    span::{ModuleId, Span},
};

use crate::{
    NodeId,
    statements::{ImportItem, ImportKind, Variable},
};

impl_soul_ids!(ScopeId);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeBuilder {
    scopes: VecMap<ModuleId, ModuleScopes>,
    alloc: IdGenerator<ScopeId>,
}
impl ScopeBuilder {
    pub fn new() -> Self {
        Self {
            scopes: VecMap::new(),
            alloc: IdGenerator::new(),
        }
    }

    pub fn push_scope(&mut self, parent: ScopeId, module: ModuleId) -> Result<(), String> {
        self.scopes
            .get_mut(module)
            .ok_or(format!("ModuleScopes of {:?} not found", module))?
            .push_scope(parent, &mut self.alloc);

        Ok(())
    }

    pub fn pop_scope(&mut self, module: ModuleId) -> Result<(), String> {
        self.scopes
            .get_mut(module)
            .ok_or(format!("ModuleScopes of {:?} not found", module))?
            .pop_scope();

        Ok(())
    }

    pub fn add_module(&mut self, module: ModuleId) -> Option<ModuleScopes> {
        self.scopes
            .insert(module, ModuleScopes::new(&mut self.alloc))
    }

    pub fn go_to(&mut self, scope_id: ScopeId, module: ModuleId) -> Result<(), String> {
        self.scopes
            .get_mut(module)
            .ok_or(format!("ModuleScopes of {:?} not found", module))?
            .go_to(scope_id)
    }

    pub fn get_scope(&self, scope_id: ScopeId, module: ModuleId) -> Option<&Scope> {
        self.scopes.get(module)?.scopes.get(scope_id)
    }

    pub fn current_scope_id(&self, module: ModuleId) -> Option<ScopeId> {
        self.scopes.get(module).map(|scopes| scopes.current)
    }

    pub fn current_scope_mut(&mut self, module: ModuleId) -> Option<&mut Scope> {
        self.scopes.get_mut(module)?.current_scope_mut()
    }

    pub fn lookup_type(&self, ident: &str, module: ModuleId) -> Option<ScopeTypeEntry> {
        self.scopes.get(module)?.lookup_type(ident)
    }

    pub fn lookup_value(&self, ident: &str, kind: ScopeValue, module: ModuleId) -> Option<NodeId> {
        self.scopes.get(module)?.lookup_value(ident, kind)
    }

    pub fn flat_lookup_type(&self, ident: &str, module: ModuleId) -> Option<ScopeTypeEntry> {
        self.scopes.get(module)?.flat_lookup_type(ident)
    }

    pub fn flat_lookup_value(
        &self,
        ident: &str,
        kind: ScopeValue,
        module: ModuleId,
    ) -> Option<NodeId> {
        self.scopes.get(module)?.flat_lookup_value(ident, kind)
    }

    pub fn flat_lookup_function(&self, name: &str, module: ModuleId) -> Option<FunctionId> {
        self.scopes.get(module)?.flat_lookup_function(name)
    }

    pub fn lookup_function(&self, name: &str, module: ModuleId) -> Option<FunctionId> {
        self.scopes.get(module)?.lookup_function(name)
    }

    pub fn lookup_module(&self, name: &str, module: ModuleId) -> Option<ScopeModuleEntry> {
        self.scopes.get(module)?.lookup_module(name)
    }

    pub fn as_vecmap(&self) -> &VecMap<ModuleId, ModuleScopes> {
        &self.scopes
    }

    pub fn iter_modules(
        &self,
        module: ModuleId,
    ) -> Option<impl Iterator<Item = (&String, &ScopeModuleEntry)>> {
        Some(self.scopes.get(module)?.modules())
    }
}
impl Default for ScopeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleScopes {
    scopes: VecMap<ScopeId, Scope>,
    current: ScopeId,
}
impl ModuleScopes {
    fn new(alloc: &mut IdGenerator<ScopeId>) -> Self {
        let current = alloc.alloc();
        Self {
            current,
            scopes: VecMap::from_vec(vec![(current, Scope::new_global(current))]),
        }
    }

    fn push_scope(&mut self, parent: ScopeId, alloc: &mut IdGenerator<ScopeId>) {
        self.current = alloc.alloc();
        self.scopes
            .insert(self.current, Scope::new_child(self.current, parent));
    }

    fn pop_scope(&mut self) {
        debug_assert!(self.scopes[self.current].parent.is_some());
        if let Some(parent) = self.scopes[self.current].parent {
            self.current = parent;
        }
    }

    fn go_to(&mut self, scope_id: ScopeId) -> Result<(), String> {
        if !self.scopes.contains(scope_id) {
            return Err(format!("ScopeId {:?} not found in module", scope_id));
        }

        self.current = scope_id;
        Ok(())
    }

    fn current_scope_mut(&mut self) -> Option<&mut Scope> {
        self.scopes.get_mut(self.current)
    }

    pub fn lookup_type(&self, ident: &str) -> Option<ScopeTypeEntry> {
        for scope in self.scope_iter() {
            if let Some(ScopeEntry { types, .. }) = scope.entries.get(ident) {
                return *types;
            }
        }
        None
    }

    pub fn lookup_value(&self, ident: &str, kind: ScopeValue) -> Option<NodeId> {
        for scope in self.scope_iter() {
            let ids = match scope.entries.get(ident) {
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

    fn flat_lookup_type(&self, ident: &str) -> Option<ScopeTypeEntry> {
        let scope = self.scopes.get(self.current)?;
        scope.entries.get(ident)?.types
    }

    fn flat_lookup_value(&self, ident: &str, kind: ScopeValue) -> Option<NodeId> {
        let scope = self.scopes.get(self.current)?;
        let ids = scope.entries.get(ident)?.values.as_ref()?;

        ids.get(kind)
    }

    fn flat_lookup_function(&self, name: &str) -> Option<FunctionId> {
        let scope = self.scopes.get(self.current)?;
        scope.entries.get(name)?.function
    }

    fn lookup_function(&self, name: &str) -> Option<FunctionId> {
        for scope in self.scope_iter() {
            if let Some(ScopeEntry { function, .. }) = scope.entries.get(name) {
                return *function;
            };
        }

        None
    }

    fn lookup_module(&self, name: &str) -> Option<ScopeModuleEntry> {
        for scope in self.scope_iter() {
            if let Some(ScopeEntry { module, .. }) = scope.entries.get(name) {
                return module.clone();
            };
        }

        None
    }

    fn modules(&self) -> impl Iterator<Item = (&std::string::String, &ScopeModuleEntry)> {
        self.scope_iter()
            .flat_map(|scope| {
                scope
                    .entries
                    .iter()
                    .filter_map(|(name, entry)| 
                        entry.module.as_ref().map(|module| (name, module))
                    )
            })
    }

    fn scope_iter<'a>(&'a self) -> ScopeIterator<'a> {
        ScopeIterator::new(&self.scopes, self.current)
    }
}

struct ScopeIterator<'a> {
    scopes: &'a VecMap<ScopeId, Scope>,
    current: Option<ScopeId>,
}
impl<'a> ScopeIterator<'a> {
    fn new(scopes: &'a VecMap<ScopeId, Scope>, current: ScopeId) -> Self {
        Self {
            scopes,
            current: Some(current),
        }
    }
}
impl<'a> Iterator for ScopeIterator<'a> {
    type Item = &'a Scope;

    fn next(&mut self) -> Option<Self::Item> {
        let scope = self.scopes.get(self.current?)?;
        self.current = scope.parent;
        Some(scope)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScopeEntry {
    function: Option<FunctionId>,
    values: Option<ScopeValueEntry>,
    types: Option<ScopeTypeEntry>,
    module: Option<ScopeModuleEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeModuleEntry {
    pub module_name: String,
    pub module_id: ModuleId,
    /// For external crate imports
    pub crate_name: Option<String>,
    pub import_kind: ImportKind,
    pub imported_items: Vec<ImportItem>,
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

    pub fn insert_module(
        &mut self,
        name: &str,
        entry: ScopeModuleEntry,
    ) -> Option<ScopeModuleEntry> {
        self.get_mut_entry(name)?.module.replace(entry)
    }

    pub fn get_module(&self, name: &str) -> Option<&ScopeModuleEntry> {
        self.entries.get(name).and_then(|e| e.module.as_ref())
    }

    pub fn get_module_entry(&self, name: &str) -> Option<&ScopeModuleEntry> {
        self.get_module(name)
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
    Enum,
    Struct,
    Union,
    LifeTime,
    GenericType,
    Trait,
}

pub enum ScopeValueKind<'a> {
    Variable(&'a Variable),
}
impl<'a> ScopeValueKind<'a> {
    pub fn get_id(&self) -> NodeId {
        match self {
            ScopeValueKind::Variable(variable) => variable.id,
        }
    }

    pub fn get_ident(&self) -> Result<&Ident, &str> {
        match self {
            ScopeValueKind::Variable(variable) => {
                variable.name().ok_or("expected simple variable pattern")
            }
        }
    }

    pub fn to_entry_kind(&self) -> ScopeValue {
        match self {
            ScopeValueKind::Variable(_) => ScopeValue::Variable,
        }
    }
}
