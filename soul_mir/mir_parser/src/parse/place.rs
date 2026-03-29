use hir::TypeId;
use soul_utils::{ids::IdAlloc, soul_error_internal, span::Span};
use typed_hir::FieldInfo;

use crate::{
    EndBlock, MirContext,
    mir::{self, Rvalue},
};

impl<'a> MirContext<'a> {

    pub fn lower_place(&mut self, place_id: hir::PlaceId) -> EndBlock<mir::PlaceId> {
        let is_end = &mut false;
        let span = self.place_span(place_id);
        let mir_place = match &self.hir_response.hir.nodes.places[place_id].kind {
            hir::PlaceKind::Local(local_id) => {
                let local = match self.local_remap.get(*local_id) {
                    Some(val) => *val,
                    None => {
                        self.log_error(soul_error_internal!(
                            format!("{:?} not found in remap", local_id),
                            Some(span)
                        ));
                        mir::LocalId::error()
                    }
                };

                self.new_place(mir::Place::Local(local))
            }
            hir::PlaceKind::Temp(local_id) => {
                let ty = self.place_type(place_id);
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
            hir::PlaceKind::Deref(inner) => {
                let ty = self.place_type(place_id);

                let base_place = self.lower_place(*inner).pass(is_end);
                let operand = self.place_to_operand(base_place, ty);
                self.new_place(mir::Place::Deref(operand))
            }
            hir::PlaceKind::Index { .. } => todo!("mir desugar index"),
            hir::PlaceKind::Field { base, .. } => {
                let base = self.lower_place(*base).pass(is_end);
                let field_id = self.hir_response.typed.types_table.place_fields[place_id];
                let FieldInfo{ base_type, field_type:_, field_index } = self.hir_response.typed.types_table.fields[field_id];
                
                self.new_place(mir::Place::Field{base, base_type, field_id, index: field_index})
            },
        };

        let ty = self.place_type(place_id);
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
            mir::Place::Field{base, base_type:_, field_id, index} => {
                let field_id = *field_id;
                let index = *index;
                let base = *base;
                let field_temp = self.new_temp(ty);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::Temp(field_temp)),
                    value: Rvalue::new(mir::RvalueKind::Field { base, index, base_type: ty, field_id }),
                });
                self.push_statement(statement);
                mir::Operand::new(ty, mir::OperandKind::Temp(field_temp))
            }
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

    fn place_span(&self, place_id: hir::PlaceId) -> Span {
        self.hir_response.hir.nodes.places[place_id].span
    }
}
