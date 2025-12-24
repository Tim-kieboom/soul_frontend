use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;
use models::abstract_syntax_tree::AbstractSyntaxTree;

pub mod name_resolve_expression;
pub mod name_resolve_statment;
pub mod name_resolve_type;
pub mod name_resolver;

impl NameResolver {
    pub fn resolve_ast(&mut self, ast: &mut AbstractSyntaxTree) {
        self.resolve_block(&mut ast.root);
    }
}
