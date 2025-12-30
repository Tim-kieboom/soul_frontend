<<<<<<< Updated upstream
=======
use crate::steps::sementic_analyser::{AstMetaData, SementicPass, sementic_fault::SementicFault};
>>>>>>> Stashed changes
use soul_ast::abstract_syntax_tree::AbstractSyntaxTree;
use soul_ast::sementic_models::{ASTSemanticInfo, SementicPass};
use soul_ast::sementic_models::sementic_fault::SementicFault;
use soul_ast::{
    error::SoulError,
    sementic_models::scope::{NodeId, NodeIdGenerator},
};

pub struct NameResolver<'a> {
    pub ids: NodeIdGenerator,
<<<<<<< Updated upstream
    pub info: &'a mut ASTSemanticInfo,
=======
    pub info: &'a mut AstMetaData,
>>>>>>> Stashed changes

    pub current_function: Option<NodeId>,
}

impl<'a> SementicPass<'a> for NameResolver<'a> {
<<<<<<< Updated upstream
    fn new(info: &'a mut ASTSemanticInfo) -> Self {
=======
    fn new(info: &'a mut AstMetaData) -> Self {
>>>>>>> Stashed changes
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
