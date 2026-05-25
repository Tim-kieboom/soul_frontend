use ast::Literal;
use hir::{ComplexLiteral, ExpressionId, TypeId};
use soul_utils::{IdAlloc, Span, soul_error_internal};
use typed_hir::ThirTypeKind;

use crate::{MirContext, mir};

impl<'a> MirContext<'a> {
    pub(super) fn lower_new_single(
        &mut self,
        value_id: hir::ExpressionId,
        heap_ptr_type: TypeId,
        is_end: &mut bool,
    ) -> mir::Operand {
        let inner = self.lower_operand(value_id).pass(is_end);
        let inner_ty = self.expression_type(value_id);
        let ptr_temp = self.new_temp(heap_ptr_type);

        let heap_stmt = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(
                mir::PlaceKind::Temp(ptr_temp),
                heap_ptr_type,
            )),
            value: mir::Rvalue::new(mir::RvalueKind::HeapAlloc {
                ty: inner_ty,
                count: 1,
            }),
        });
        self.push_statement(heap_stmt);

        let ptr_operand = mir::Operand::new(heap_ptr_type, mir::OperandKind::Temp(ptr_temp));
        let store_place = self.new_place(mir::Place::new(
            mir::PlaceKind::Deref(ptr_operand),
            inner_ty,
        ));
        let store_stmt = mir::Statement::new(mir::StatementKind::Assign {
            place: store_place,
            value: mir::Rvalue::new(mir::RvalueKind::Operand(inner)),
        });
        self.push_statement(store_stmt);

        mir::Operand::new(heap_ptr_type, mir::OperandKind::Temp(ptr_temp))
    }

    pub(super) fn lower_new_array(
        &mut self,
        values: &Vec<ExpressionId>,
        ptr_type: TypeId,
        array_type: TypeId,
        span: Span,
        is_end: &mut bool,
    ) -> mir::Operand {
        let count = values.len() as u64;

        let element_type = match self.id_to_type(array_type).kind {
            ThirTypeKind::Array { element, .. } => element,
            _ => {
                self.log_error(soul_error_internal!(
                    "array type should be ThirTypeKind::Array",
                    Some(span)
                ));
                TypeId::error()
            }
        };

        let ptr_temp = self.new_temp(ptr_type);

        let heap_stmt = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(ptr_temp), ptr_type)),
            value: mir::Rvalue::new(mir::RvalueKind::HeapAlloc {
                ty: element_type,
                count,
            }),
        });
        self.push_statement(heap_stmt);

        for (i, val_id) in values.iter().enumerate() {
            let val = self.lower_operand(*val_id).pass(is_end);
            let offset_type = self.hir_response.typed.types_table.u32_type;
            let offset_op = mir::Operand::new(
                offset_type,
                mir::OperandKind::Comptime(ComplexLiteral::Basic(Literal::Uint(i as u128))),
            );
            let ptr_operand = mir::Operand::new(ptr_type, mir::OperandKind::Temp(ptr_temp));
            let elem_ptr_temp = self.new_temp(ptr_type);

            let offset_stmt = mir::Statement::new(mir::StatementKind::Assign {
                place: self.new_place(mir::Place::new(
                    mir::PlaceKind::Temp(elem_ptr_temp),
                    ptr_type,
                )),
                value: mir::Rvalue::new(mir::RvalueKind::PtrOffset {
                    pointer: ptr_operand,
                    offset: offset_op,
                }),
            });
            self.push_statement(offset_stmt);

            let elem_ptr = mir::Operand::new(ptr_type, mir::OperandKind::Temp(elem_ptr_temp));
            let store_place = self.new_place(mir::Place::new(
                mir::PlaceKind::Deref(elem_ptr),
                element_type,
            ));
            let store_stmt = mir::Statement::new(mir::StatementKind::Assign {
                place: store_place,
                value: mir::Rvalue::new(mir::RvalueKind::Operand(val)),
            });
            self.push_statement(store_stmt);
        }

        let array_struct = self.hir_response.typed.types_map.array_struct;
        let result_temp = self.new_temp(array_type);
        let len_type = self.hir_response.typed.types_table.u32_type;
        let const_len = mir::Operand::new(
            len_type,
            mir::OperandKind::Comptime(ComplexLiteral::Basic(Literal::Uint(count as u128))),
        );

        let aggregate = mir::Rvalue::new(mir::RvalueKind::Aggregate {
            struct_type: array_struct,
            body: mir::AggregateBody::Runtime(vec![
                mir::Operand::new(ptr_type, mir::OperandKind::Temp(ptr_temp)),
                const_len,
            ]),
        });

        let result_stmt = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(
                mir::PlaceKind::Temp(result_temp),
                array_type,
            )),
            value: aggregate,
        });
        self.push_statement(result_stmt);

        mir::Operand::new(array_type, mir::OperandKind::Temp(result_temp))
    }

    pub(super) fn lower_new_heap_array(
        &mut self,
        ptr_id: hir::ExpressionId,
        len_id: hir::ExpressionId,
        array_type: TypeId,
        _span: Span,
        is_end: &mut bool,
    ) -> mir::Operand {
        let ptr = self.lower_operand(ptr_id).pass(is_end);
        let len = self.lower_operand(len_id).pass(is_end);

        let array_struct = self.hir_response.typed.types_map.array_struct;
        let result_temp = self.new_temp(array_type);

        let aggregate = mir::Rvalue::new(mir::RvalueKind::Aggregate {
            struct_type: array_struct,
            body: mir::AggregateBody::Runtime(vec![ptr, len]),
        });

        let result_stmt = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(
                mir::PlaceKind::Temp(result_temp),
                array_type,
            )),
            value: aggregate,
        });
        self.push_statement(result_stmt);

        mir::Operand::new(array_type, mir::OperandKind::Temp(result_temp))
    }

    pub(super) fn lower_alloc(
        &mut self,
        size_id: hir::ExpressionId,
        value_type: TypeId,
        span: Span,
        is_end: &mut bool,
    ) -> mir::Operand {
        let _ = span;
        let size = self.lower_operand(size_id).pass(is_end);
        let temp = self.new_temp(value_type);

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), value_type)),
            value: mir::Rvalue::new(mir::RvalueKind::Alloc { size }),
        });

        self.push_statement(statement);
        mir::Operand::new(value_type, mir::OperandKind::Temp(temp))
    }

    pub(super) fn lower_dealloc(
        &mut self,
        ptr_id: hir::ExpressionId,
        is_end: &mut bool,
    ) -> mir::Operand {
        let ptr = self.lower_operand(ptr_id).pass(is_end);

        let statement = mir::Statement::new(mir::StatementKind::Dealloc { ptr });
        self.push_statement(statement);

        self.new_none_operand()
    }

    pub(super) fn lower_realloc(
        &mut self,
        ptr_id: hir::ExpressionId,
        size_id: hir::ExpressionId,
        value_type: TypeId,
        span: Span,
        is_end: &mut bool,
    ) -> mir::Operand {
        let _ = span;
        let ptr = self.lower_operand(ptr_id).pass(is_end);
        let size = self.lower_operand(size_id).pass(is_end);
        let temp = self.new_temp(value_type);

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), value_type)),
            value: mir::Rvalue::new(mir::RvalueKind::Realloc { ptr, size }),
        });

        self.push_statement(statement);
        mir::Operand::new(value_type, mir::OperandKind::Temp(temp))
    }

    pub(super) fn lower_drop(
        &mut self,
        value_id: hir::ExpressionId,
        value_type: TypeId,
        span: Span,
        is_end: &mut bool,
    ) -> mir::Operand {
        let value = self.lower_operand(value_id).pass(is_end);

        let temp_id = self.new_temp(value_type);
        let drop_stmt = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp_id), value_type)),
            value: mir::Rvalue::new(mir::RvalueKind::Drop { value, span }),
        });
        self.push_statement(drop_stmt);

        let none_type = self.hir_response.typed.types_table.none_type;
        mir::Operand::new(none_type, mir::OperandKind::None)
    }
}
