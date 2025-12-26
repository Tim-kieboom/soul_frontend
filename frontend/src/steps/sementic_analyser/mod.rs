use models::{abstract_syntax_tree::AbstractSyntaxTree, sementic_models::scope::{ScopeBuilder}};

use crate::SementicFault;

pub mod name_resolution;
pub mod sementic_fault;
pub mod type_resolution;

pub(crate) struct SementicInfo {
    scopes: ScopeBuilder,
    faults: Vec<SementicFault>,
}
impl SementicInfo {
    pub fn new() -> Self {
        Self {
            scopes: ScopeBuilder::new(),
            faults: vec![],
        }
    }

    pub fn consume_faults(self) -> Vec<SementicFault> {
        self.faults
    }
}

pub(crate) trait SementicPass<'a> {
    fn new(info: &'a mut SementicInfo) -> Self;
    fn run(&mut self, ast: &mut AbstractSyntaxTree);
}
