mod id_generator;
use ast::{DeclareStore, ParseResponse};
use hir::{BlockId, FunctionId, HirTree, LocalId};
use id_generator::IdGenerator;
use soul_utils::{Ident, error::SoulError, sementic_level::SementicFault, soul_error_internal};
use std::collections::HashMap;

mod expression;
mod insert;
mod place;
mod resolve_import;
mod statement;
mod r#type;

pub fn hir_lower(response: &ParseResponse, faults: &mut Vec<SementicFault>) -> HirTree {
    let mut context = HirContext::new(&response.store, faults);

    for global in &response.tree.root.statements {
        context.lower_global(global);
    }

    context.hir
}

#[derive(Debug, Default)]
struct Scope {
    locals: HashMap<String, LocalId>,
    functions: HashMap<String, FunctionId>,
}

#[derive(Debug)]
struct HirContext<'a> {
    pub ast_store: &'a DeclareStore,

    pub hir: HirTree,

    pub scopes: Vec<Scope>,
    pub current_body: CurrentBody,

    pub id_generator: IdGenerator,
    pub faults: &'a mut Vec<SementicFault>,
}

#[derive(Debug, Clone, Copy, Default)]
enum CurrentBody {
    #[default]
    Global,
    Block(BlockId),
}

impl<'a> HirContext<'a> {
    fn new(ast_store: &'a DeclareStore, faults: &'a mut Vec<SementicFault>) -> Self {
        let mut id_generator = IdGenerator::new();
        let root_id = id_generator.alloc_module();

        Self {
            faults,
            ast_store,
            id_generator,
            scopes: vec![Scope::default()],
            current_body: CurrentBody::Global,
            hir: HirTree::new(root_id),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn find_local(&mut self, name: &Ident) -> Option<LocalId> {
        for store in self.scopes.iter().rev() {
            if let Some(id) = store.locals.get(name.as_str()).copied() {
                return Some(id);
            }
        }

        None
    }

    fn find_function(&mut self, name: &Ident) -> Option<FunctionId> {
        for store in self.scopes.iter().rev() {
            if let Some(id) = store.functions.get(name.as_str()).copied() {
                return Some(id);
            }
        }

        None
    }

    fn insert_local(&mut self, name: &Ident, local: LocalId) {
        let scope = match self.scopes.last_mut() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "tryed to insert_local in global scope",
                    Some(name.span)
                ));
                return;
            }
        };

        scope.locals.insert(name.to_string(), local);
    }

    fn insert_function(&mut self, name: &Ident, function: FunctionId) {
        let scope = match self.scopes.last_mut() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "tryed to insert_local in global scope",
                    Some(name.span)
                ));
                return;
            }
        };

        scope.functions.insert(name.to_string(), function);
    }

    fn pop_scope(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }
}
