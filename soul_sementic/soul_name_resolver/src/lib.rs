use parser_models::{AbstractSyntaxTree, ParseResponse, meta_data::AstMetadata, scope::{NodeId, NodeIdGenerator}};
use soul_utils::{error::SoulError, sementic_level::SementicFault};

mod collect;
mod resolve;

pub fn name_resolve(request: &mut ParseResponse) {
    let mut resolver = NameResolver::new(&mut request.meta_data);
    resolver.run(&mut request.tree);
}

struct NameResolver<'a>  {
    id_generator: NodeIdGenerator,
    info: &'a mut AstMetadata,
    current_function: Option<NodeId>,
}
impl<'a> NameResolver<'a> {
    fn new(info: &'a mut AstMetadata) -> Self {
        Self {
            info,
            current_function: None,
            id_generator: NodeIdGenerator::new(),
        }
    }

    fn run(&mut self, ast: &mut AbstractSyntaxTree) {
        self.collect_declarations(&mut ast.root);
        self.resolve_names(&mut ast.root);
    }

    fn log_error(&mut self, error: SoulError) {
        self.info.faults.push(SementicFault::error(error));
    }
}