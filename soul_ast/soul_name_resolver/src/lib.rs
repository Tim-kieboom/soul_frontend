use ast::{
    AstResponse, DeclareStore,
    meta_data::AstMetadata,
    scope::{NodeId, ScopeValue},
};
use soul_utils::{
    Ident,
    error::SoulError,
    ids::{FunctionId, IdGenerator},
    sementic_level::SementicFault,
};

mod collect;
mod resolve;

pub fn name_resolve(request: &mut AstResponse, faults: &mut Vec<SementicFault>) {
    let mut resolver = NameResolver::new(
        &mut request.meta_data,
        faults,
        &mut request.store,
        &mut request.function_generators,
    );
    let root = &mut request.tree.root;

    resolver.collect_declarations(root);
    resolver.resolve_names(root);
}

struct NameResolver<'a> {
    node_generator: IdGenerator<NodeId>,
    function_generator: &'a mut IdGenerator<FunctionId>,
    info: &'a mut AstMetadata,
    store: &'a mut DeclareStore,
    faults: &'a mut Vec<SementicFault>,
    current_function: Option<FunctionId>,
}
impl<'a> NameResolver<'a> {
    fn new(
        info: &'a mut AstMetadata,
        faults: &'a mut Vec<SementicFault>,
        store: &'a mut DeclareStore,
        function_generator: &'a mut IdGenerator<FunctionId>,
    ) -> Self {
        Self {
            info,
            store,
            faults,
            function_generator,
            current_function: None,
            node_generator: IdGenerator::new(),
        }
    }

    fn log_error(&mut self, error: SoulError) {
        self.faults.push(SementicFault::error(error));
    }

    fn check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValue::Variable)
    }
}
