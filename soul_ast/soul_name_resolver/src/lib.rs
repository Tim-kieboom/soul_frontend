use ast::{DeclareStore, ParseResponse, meta_data::AstMetadata, scope::{NodeId, NodeIdGenerator, ScopeValue}};
use soul_utils::{Ident, error::SoulError, sementic_level::SementicFault};

mod collect;
mod resolve;

pub fn name_resolve(request: &mut ParseResponse, faults: &mut Vec<SementicFault>) {
    let mut resolver = NameResolver::new(&mut request.meta_data, faults, &mut request.store);
    let root = &mut request.tree.root;
    
    resolver.collect_declarations(root);
    resolver.resolve_names(root);
}

struct NameResolver<'a>  {
    id_generator: NodeIdGenerator,
    info: &'a mut AstMetadata,
    store: &'a mut DeclareStore,
    faults: &'a mut Vec<SementicFault>,
    current_function: Option<NodeId>,
}
impl<'a> NameResolver<'a> {
    fn new(info: &'a mut AstMetadata, faults: &'a mut Vec<SementicFault>, store: &'a mut DeclareStore) -> Self {
        Self {
            info,
            store,
            faults,
            current_function: None,
            id_generator: NodeIdGenerator::new(),
        }
    }

    fn log_error(&mut self, error: SoulError) {
        self.faults.push(SementicFault::error(error));
    }

    fn check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }
}