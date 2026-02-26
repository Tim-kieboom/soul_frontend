use hir::{IdAlloc, TypeId};
use soul_utils::soul_error_internal;

use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub fn lower_place(&mut self, place: &hir::Place) -> mir::PlaceId {
        match &place.node {
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

                let base_place = self.lower_place(inner);
                let temp = self.place_to_temp(base_place, ty);
                let operand = mir::Operand::new(mir::OperandKind::Temp(temp));

                self.new_place(mir::Place::Deref(operand))
            }
            hir::PlaceKind::Index { base, index, .. } => {
                let place = self.lower_place(base);
                let operand = self.lower_operand(*index);
                self.new_place(mir::Place::Index(place, operand))
            }
            hir::PlaceKind::Field { base, index, .. } => {
                let place = self.lower_place(base);
                self.new_place(mir::Place::Field(place, *index))
            }
        }
    }

    fn place_to_temp(&mut self, place: mir::PlaceId, ty: TypeId) -> mir::TempId {
        let temp = self.new_temp(ty);
        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place,
            value: mir::Rvalue::new(mir::RvalueKind::Use(mir::Operand::new(
                mir::OperandKind::Temp(temp),
            ))),
        });
        self.push_statement(statement);
        temp
    }
}
