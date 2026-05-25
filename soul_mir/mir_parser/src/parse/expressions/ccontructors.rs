use ast::Literal;
use hir::{ComplexLiteral, ExpressionId, StructId, TypeId, UnionId};
use soul_utils::{IdAlloc, Ident, soul_error_internal};

use crate::{
    EndBlock, MirContext,
    mir::{self, Operand},
};

impl<'a> MirContext<'a> {
    pub(super) fn lower_struct_constructor(
        &mut self,
        values: &Vec<(Ident, ExpressionId)>,
        struct_id: StructId,
        struct_type: TypeId,
    ) -> EndBlock<Operand> {
        let r#struct = self
            .hir_response
            .typed
            .types_map
            .id_to_struct(struct_id)
            .expect("should have struct");

        let dummy = Operand::new(TypeId::error(), mir::OperandKind::None);
        let is_end = &mut false;

        let mut runtime = false;
        let mut fields = Vec::new();

        fields.resize(r#struct.fields.len(), dummy);

        for (name, value) in values {
            let i = match self.find_field_index(r#struct, name.as_str()) {
                Some(val) => val,
                None => continue,
            };

            let value_type = self.expression_type(*value);
            let operand = match self.get_expression_literal(*value) {
                Some(literal) => {
                    Operand::new(value_type, mir::OperandKind::Comptime(literal.clone()))
                }
                None => {
                    runtime = true;
                    self.lower_operand(*value).pass(is_end)
                }
            };
            fields[i] = operand;
        }

        let all_comptime = fields
            .iter()
            .all(|op| matches!(op.kind, mir::OperandKind::Comptime(_)));

        let body = if runtime || !all_comptime {
            if !runtime && !all_comptime {
                self.log_error(soul_error_internal!(
                    "expected all fields to be compile-time known in struct constructor",
                    None
                ));
            }
            mir::AggregateBody::Runtime(fields)
        } else {
            let literals = fields
                .into_iter()
                .enumerate()
                .map(|(i, op)| {
                    let ty = r#struct.fields[i].ty;
                    match op.kind {
                        mir::OperandKind::Comptime(literal) => (literal, ty),
                        _ => unreachable!(),
                    }
                })
                .collect();

            mir::AggregateBody::Comptime(literals)
        };

        let ctor = mir::RvalueKind::Aggregate {
            struct_type: struct_id,
            body,
        };
        let temp = self.new_temp(struct_type);

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), struct_type)),
            value: mir::Rvalue::new(ctor),
        });
        self.push_statement(statement);
        let operand = mir::Operand::new(struct_type, mir::OperandKind::Temp(temp));
        EndBlock::new(operand, is_end)
    }

    pub(super) fn lower_union_constructor(
        &mut self,
        union_id: UnionId,
        variant_index: usize,
        value: ExpressionId,
        union_type: TypeId,
    ) -> EndBlock<Operand> {
        let union = self
            .hir_response
            .hir
            .info
            .types
            .id_to_union(union_id)
            .expect("should have union");

        let internal_struct_id = union.internal_struct;

        let internal_struct = self
            .hir_response
            .typed
            .types_map
            .id_to_struct(internal_struct_id)
            .expect("should have internal union struct");

        let is_end = &mut false;
        let mut fields = Vec::with_capacity(2);

        let tag_type = internal_struct.fields[0].ty;
        let tag_operand = Operand::new(
            tag_type,
            mir::OperandKind::Comptime(ComplexLiteral::Basic(Literal::Int(variant_index as i128))),
        );
        fields.push(tag_operand);

        let operand = self.lower_operand(value).pass(is_end);
        fields.push(operand);

        let is_comptime = fields
            .iter()
            .all(|op| matches!(op.kind, mir::OperandKind::Comptime(_)));

        let body = if !is_comptime {
            mir::AggregateBody::Runtime(fields)
        } else {
            let literals = fields
                .into_iter()
                .enumerate()
                .map(|(i, op)| {
                    let ty = if i == 0 {
                        internal_struct.fields[0].ty
                    } else {
                        match &op.kind {
                            mir::OperandKind::Comptime(_) => op.ty,
                            _ => unreachable!(),
                        }
                    };
                    match op.kind {
                        mir::OperandKind::Comptime(literal) => (literal, ty),
                        _ => unreachable!(),
                    }
                })
                .collect();
            mir::AggregateBody::Comptime(literals)
        };

        let ctor = mir::RvalueKind::Aggregate {
            struct_type: internal_struct_id,
            body,
        };
        let temp = self.new_temp(union_type);
        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), union_type)),
            value: mir::Rvalue::new(ctor),
        });
        self.push_statement(statement);
        let operand = mir::Operand::new(union_type, mir::OperandKind::Temp(temp));
        EndBlock::new(operand, is_end)
    }
}
