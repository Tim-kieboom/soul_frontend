use hir_model::HirTree;
use soul_typed_context::{TypedContext};
use soul_utils::{error::SoulError, sementic_level::SementicFault, vec_map::VecMap};
use thir_model as thir;

pub(crate) mod lower_body;
mod lower_expression;
mod lower_statement;

pub(crate) struct LowerFunctionContext<'a> {
    hir: &'a HirTree,
    typed_context: &'a TypedContext,

    faults: &'a mut Vec<SementicFault>,

    id_generator: thir::IdGenerator,
    pub locals: VecMap<thir::LocalId, thir::Local>,
    pub expressions: VecMap<thir::ExpressionId, thir::TypedExpression>,
}
impl<'a> LowerFunctionContext<'a> {
    pub(crate) fn new(
        hir: &'a HirTree,
        typed: &'a TypedContext,
        faults: &'a mut Vec<SementicFault>,
    ) -> Self {
        Self {
            hir,
            faults,
            typed_context: typed,
            locals: VecMap::new(),
            expressions: VecMap::new(),
            id_generator: thir::IdGenerator::new(),
        }
    }

    pub(crate) fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }
}
