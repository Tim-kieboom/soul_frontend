use hir::TypeId;
use soul_utils::{ids::IdAlloc, soul_error_internal};

use crate::{
    EndBlock, MirContext,
    mir::{self, Rvalue},
};

impl<'a> MirContext<'a> {
    pub fn lower_place(&mut self, place: &hir::Place) -> EndBlock<mir::PlaceId> {
        let is_end = &mut false;
        let mir_place = match &place.node {
            hir::PlaceKind::Local(local_id, _) => {
                let local = match self.local_remap.get(*local_id) {
                    Some(val) => *val,
                    None => {
                        self.log_error(soul_error_internal!(
                            format!("{:?} not found in remap", local_id),
                            Some(place.span)
                        ));
                        mir::LocalId::error()
                    }
                };

                self.new_place(mir::Place::Local(local))
            }
            hir::PlaceKind::Temp(local_id, id) => {
                let ty = self.types.places[*id];
                let temp = match self.temp_remap.get(*local_id) {
                    Some(val) => *val,
                    None => {
                        let temp = self.new_temp(ty);
                        self.temp_remap.insert(*local_id, temp);
                        temp
                    }
                };

                self.new_place(mir::Place::Temp(temp))
            }
            hir::PlaceKind::Deref(inner, _) => {
                let id = inner.node.get_id();
                let ty = self.types.places[id];

                let base_place = self.lower_place(inner).pass(is_end);
                let operand = self.place_to_operand(base_place, ty);
                self.new_place(mir::Place::Deref(operand))
            }
            hir::PlaceKind::Index { .. } => todo!("mir desugar index"),
            hir::PlaceKind::Field { .. } => todo!("mir desugar field"),
        };

        let ty = self.types.places[place.node.get_id()];
        self.place_typed.insert(mir_place, ty);
        EndBlock::new(mir_place, is_end)
    }

    pub(crate) fn place_to_temp(&mut self, place_id: mir::PlaceId, ty: TypeId) -> mir::TempId {
        if let mir::Place::Temp(temp) = &self.tree.places[place_id] {
            return *temp;
        }

        let temp = self.new_temp(ty);
        let value = self.place_to_operand(place_id, ty);
        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::Temp(temp)),
            value: Rvalue::new(mir::RvalueKind::Use(value)),
        });
        self.push_statement(statement);
        temp
    }

    pub(crate) fn place_to_operand(&mut self, place_id: mir::PlaceId, ty: TypeId) -> mir::Operand {
        match &self.tree.places[place_id] {
            mir::Place::Local(local_id) => {
                mir::Operand::new(ty, mir::OperandKind::Local(*local_id))
            }
            mir::Place::Temp(temp) => mir::Operand::new(ty, mir::OperandKind::Temp(*temp)),
            mir::Place::Deref(operand) => {
                let operand = operand.clone();
                self.deref_to_operand(operand)
            }
        }
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
