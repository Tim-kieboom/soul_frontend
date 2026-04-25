use hir::LocalKind;
#[cfg(debug_assertions)]
use soul_utils::soul_error_internal;

use crate::{
    MirContext,
    mir::{self, ModuleNodeId},
};

impl<'a> MirContext<'a> {
    pub(crate) fn lower_global(
        &mut self,
        global: &hir::Global,
        is_end: &mut bool,
    ) -> Option<ModuleNodeId> {
        match &global.kind {
            hir::GlobalKind::Function(function) => {
                if self.hir_response.hir.main == Some(*function) {
                    return Some(ModuleNodeId::FunctionId(*function)); // lower main later
                }
                self.lower_function(*function);
                Some(ModuleNodeId::FunctionId(*function))
            }
            hir::GlobalKind::Variable(variable) | hir::GlobalKind::InternalVariable(variable) => {
                let local = match self.lower_global_variable(variable) {
                    Some(val) => val,
                    None => return None,
                };

                let local_info = &self.hir_response.hir.nodes.locals[variable.local];

                let value = match &local_info.kind {
                    LocalKind::Variable(Some(val)) => *val,
                    _ => return None,
                };

                self.current.function = self.tree.init_global_function;
                self.current.block = Some(self.expect_init_global_block());

                let place = self.new_place(local);
                let value = self.lower_operand(value).pass(is_end);
                let id = self.push_statement(mir::Statement::new(mir::StatementKind::Assign {
                    place,
                    value: mir::Rvalue::new(mir::RvalueKind::Operand(value)),
                }));
                Some(ModuleNodeId::StatementId(id))
            }

            hir::GlobalKind::InternalAssign(assign) => {
                self.current.function = self.tree.init_global_function;
                self.current.block = Some(self.expect_init_global_block());

                let place = self.lower_place(assign.place).pass(is_end);
                let value = self.lower_operand(assign.value).pass(is_end);
                let id = self.push_statement(mir::Statement::new(mir::StatementKind::Assign {
                    place,
                    value: mir::Rvalue::new(mir::RvalueKind::Operand(value)),
                }));
                Some(ModuleNodeId::StatementId(id))
            }
        }
    }

    fn lower_global_variable(&mut self, variable: &hir::Variable) -> Option<mir::Place> {
        let local_info = &self.hir_response.hir.nodes.locals[variable.local];
        let ty = self.local_type(variable.local);
        if local_info.is_temp() {
            let temp = self.new_temp(self.local_type(variable.local));
            return Some(mir::Place::new(mir::PlaceKind::Temp(temp), ty));
        }

        let local = self.new_local_global(variable.local, ty);
        let id = self.id_generators.alloc_global();

        let span = local_info.span;
        let value_id = match local_info.kind {
            LocalKind::Variable(Some(val)) => val,
            _ => {
                #[cfg(debug_assertions)]
                self.log_error(soul_error_internal!(
                    "global variables should have Some(_) value",
                    span
                ));
                return None;
            }
        };

        let literal = self.get_expression_literal(value_id);
        let is_literal = literal.is_some();
        let global = mir::Global {
            id,
            ty,
            local,
            literal,
        };
        self.tree.globals.insert(id, global);
        if is_literal {
            return None;
        }

        Some(mir::Place::new(mir::PlaceKind::Local(local), ty))
    }
}
