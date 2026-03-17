#[cfg(debug_assertions)]
use soul_utils::soul_error_internal;

use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub(crate) fn lower_global(&mut self, global: &hir::Global, is_end: &mut bool) {
        match global {
            hir::Global::Function(function, _) => {
                if *function == self.hir.main_function {
                    return; // lower main later
                }
                self.lower_function(*function)
            }
            hir::Global::Variable(variable, _) | hir::Global::InternalVariable(variable, _) => {
                let local = match self.lower_global_variable(variable) {
                    Some(val) => val,
                    None => return,
                };

                let value = match variable.value {
                    Some(val) => val,
                    None => return,
                };

                self.current.function = self.tree.init_global_function;
                self.current.block = Some(self.expect_init_global_block());

                let place = self.new_place(local);
                let value = self.lower_operand(value).pass(is_end);
                self.push_statement(mir::Statement::new(mir::StatementKind::Assign {
                    place,
                    value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                }));
            }

            hir::Global::InternalAssign(assign, _) => {
                self.current.function = self.tree.init_global_function;
                self.current.block = Some(self.expect_init_global_block());

                let place = self.lower_place(&assign.place).pass(is_end);
                let value = self.lower_operand(assign.value).pass(is_end);
                self.push_statement(mir::Statement::new(mir::StatementKind::Assign {
                    place,
                    value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
                }));
            }
        }
    }

    fn lower_global_variable(&mut self, variable: &hir::Variable) -> Option<mir::Place> {
        if variable.is_temp {
            return Some(mir::Place::Temp(
                self.new_temp(self.types.locals[variable.local]),
            ));
        }

        let ty = self.types.locals[variable.local];
        let local = self.new_local_global(variable.local, ty);
        let id = self.id_generators.alloc_global();

        let value_id = match variable.value {
            Some(val) => val,
            None => {
                #[cfg(debug_assertions)]
                self.log_error(soul_error_internal!(
                    "global variables should have Some(_) value",
                    None
                ));
                return None;
            }
        };

        let literal =
            if let hir::ExpressionKind::Literal(literal) = &self.hir.expressions[value_id].kind {
                Some(literal.clone())
            } else {
                None
            };

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

        Some(mir::Place::Local(local))
    }
}