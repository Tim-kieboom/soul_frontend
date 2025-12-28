use crate::steps::sementic_analyser::{SemanticInfo, SementicPass, sementic_fault::SementicFault};
use soul_ast::abstract_syntax_tree::AbstractSyntaxTree;
use soul_ast::{
    error::SoulError,
    sementic_models::scope::{NodeId, NodeIdGenerator},
};

pub struct NameResolver<'a> {
    pub ids: NodeIdGenerator,
    pub info: &'a mut SemanticInfo,

    pub current_function: Option<NodeId>,
}

impl<'a> SementicPass<'a> for NameResolver<'a> {
    fn new(info: &'a mut SemanticInfo) -> Self {
        Self {
            info,
            current_function: None,
            ids: NodeIdGenerator::new(),
        }
    }

    fn run(&mut self, ast: &mut AbstractSyntaxTree) {
        self.collect_declarations(&mut ast.root);
        self.resolve_names(&mut ast.root);
    }
}

impl<'a> NameResolver<'a> {
    pub(super) fn log_error(&mut self, err: SoulError) {
        self.info.faults.push(SementicFault::error(err));
    }
}
