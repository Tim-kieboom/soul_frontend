use hir::{IdAlloc, TypeId};
use soul_utils::soul_error_internal;

use crate::{
    EndBlock, MirContext, mir::{self, Rvalue}
};

impl<'a> MirContext<'a> {
    pub fn lower_place(&mut self, place: &hir::Place) -> EndBlock<mir::PlaceId> {
        let is_end = &mut false;
        let place = match &place.node {
            hir::PlaceKind::Local(local_id, _) => {
                let local = match self.local_remap.get(*local_id) {
                    Some(val) => *val,
                    None => {
                        self.log_error(soul_error_internal!(
                            "local {:?} not found in remap",
                            Some(place.span)
                        ));
                        mir::LocalId::error()
                    }
                };

                self.new_place(mir::Place::Local(local))
            }
            hir::PlaceKind::Deref(inner, _) => {
                let id = inner.node.get_id();
                let ty = self.types.places[id];

                let base_place = self.lower_place(inner).pass(is_end);
                let temp = self.place_to_temp(base_place, ty);
                let operand = mir::Operand::new(ty, mir::OperandKind::Temp(temp));

                self.new_place(mir::Place::Deref(operand))
            }
            hir::PlaceKind::Index { base, index, .. } => {
                let place = self.lower_place(base).pass(is_end);
                let operand = self.lower_operand(*index).pass(is_end);
                self.new_place(mir::Place::Index(place, operand))
            }
            hir::PlaceKind::Field { base, index, .. } => {
                let place = self.lower_place(base).pass(is_end);
                self.new_place(mir::Place::Field(place, *index))
            }
        };

        EndBlock::new(place, is_end)
    }

    pub(crate) fn place_to_temp(&mut self, place_id: mir::PlaceId, ty: TypeId) -> mir::TempId {
        if let mir::Place::Temp(temp) = &self.tree.places[place_id] {
            return *temp;
        }

        let temp = self.new_temp(ty);
        let value = match &self.tree.places[place_id] {
            mir::Place::Local(local_id) => {
                mir::Operand::new(ty, mir::OperandKind::Local(*local_id))
            }

            mir::Place::Deref(operand) => {
                let operand = operand.clone();
                self.deref_to_operand(operand)
            }
            mir::Place::Index(_, _) => todo!(),
            mir::Place::Field(_, _) => todo!(),

            mir::Place::Temp(_) => unreachable!(),
        };

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::Temp(temp)),
            value: Rvalue::new(mir::RvalueKind::Use(value)),
        });
        self.push_statement(statement);
        temp
    }

    fn deref_to_operand(&mut self, operand: mir::Operand) -> mir::Operand {
        let ty = operand.ty;
        let deref_temp = self.new_temp(ty);

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::Temp(deref_temp)),
            value: Rvalue::new(mir::RvalueKind::Use(operand)),
        });
        self.push_statement(statement);

        mir::Operand::new(ty, mir::OperandKind::Temp(deref_temp))
    }
}
