use parser_models::{AbstractSyntaxTree, ParseResponse, meta_data::AstMetadata, scope::{NodeId, NodeIdGenerator, ScopeValueEntryKind}};
use soul_utils::{Ident, error::SoulError, sementic_level::SementicFault};

mod collect;
mod resolve;

pub fn name_resolve(request: &mut ParseResponse) {
    let mut resolver = NameResolver::new(&mut request.meta_data, &mut request.faults);
    resolver.run(&mut request.tree);
}

struct NameResolver<'a>  {
    id_generator: NodeIdGenerator,
    info: &'a mut AstMetadata,
    faults: &'a mut Vec<SementicFault>,
    current_function: Option<NodeId>,
}
impl<'a> NameResolver<'a> {
    fn new(info: &'a mut AstMetadata, faults: &'a mut Vec<SementicFault>) -> Self {
        Self {
            info,
            faults,
            current_function: None,
            id_generator: NodeIdGenerator::new(),
        }
    }

    fn run(&mut self, ast: &mut AbstractSyntaxTree) {
        self.collect_declarations(&mut ast.root);
        self.resolve_names(&mut ast.root);
    }

    fn log_error(&mut self, error: SoulError) {
        self.faults.push(SementicFault::error(error));
    }

    fn check_variable(&mut self, name: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_value(name, ScopeValueEntryKind::Variable)
    }
}