use crate::{GenericSubstitute, IrOperand, LlvmBackend, Local, OperandInfo};
use hir::{ComplexLiteral, StructId, TypeId};
use inkwell::values::BasicValueEnum;
use mir_parser::mir::{self, AggregateBody, Operand, PlaceId, Rvalue, RvalueKind};
use soul_utils::{error::SoulResult, soul_error_internal};
use typed_hir::{FieldInfo, ThirTypeKind, display_thir::DisplayThirType};

mod binary_unary;
mod cast;
pub(crate) mod operand;

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn lower_rvalue(
        &self,
        value: &Rvalue,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        match &value.kind {
            RvalueKind::Field { base, field_id } => {
                let field_info = &self.types.types_table.fields[*field_id];
                self.lower_field_access(*base, field_info, generics)
            }
            RvalueKind::CastUse { value, cast_to } => self.lower_cast(value, *cast_to, generics),
            RvalueKind::Use(operand) => self.lower_operand(operand, generics),
            RvalueKind::Binary {
                left,
                operator,
                right,
            } => self.lower_binary(left, operator, right, generics),
            RvalueKind::Unary { operator, value } => self.lower_unary(value, operator, generics),
            RvalueKind::StackAlloc(ty) => self.lower_stack_alloc(*ty, generics),
            RvalueKind::Aggregate { struct_type, body } => {
                self.lower_struct_contructor(*struct_type, ty, body, generics)
            }
        }
    }

    pub(crate) fn new_loaded_operand(
        &self,
        value: BasicValueEnum<'a>,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ir_type = self
            .lower_type(ty, generics)?
            .unwrap_or(self.context.i8_type().into());

        Ok(IrOperand {
            value,
            info: OperandInfo::new_loaded(ty, ir_type),
        })
    }

    pub(crate) fn new_unloaded_operand(
        &self,
        value: BasicValueEnum<'a>,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ir_type = self
            .lower_type(ty, generics)?
            .unwrap_or(self.context.i8_type().into());

        Ok(IrOperand {
            value,
            info: OperandInfo::new_unloaded(ty, ir_type),
        })
    }

    fn lower_field_access(
        &self,
        base: PlaceId,
        field_info: &FieldInfo,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let base_operand = self.lower_place_to_operand(base, generics)?;
        let base_ptr = base_operand.get_or_convert_pointer(&self.builder)?;

        self.expect_type_can_field(field_info.base_type)?;
        let base_type =
            self.lower_type(field_info.base_type, generics)?
                .ok_or(soul_error_internal!(
                    "none type found as base_type in field",
                    None
                ))?;

        let field_type = self
            .lower_type(field_info.field_type, generics)?
            .ok_or(soul_error_internal!("type should be Some", None))?;

        let field = self
            .builder
            .build_field_access(base_type, field_type, base_ptr, field_info)?;

        self.new_unloaded_operand(field.into(), field_info.field_type, generics)
    }

    pub(crate) fn expect_type_can_field(&self, base_type: TypeId) -> SoulResult<()> {
        let hir_type = self.get_type(base_type)?;
        match &hir_type.kind {
            ThirTypeKind::Struct(_) => Ok(()),
            _ => Err(soul_error_internal!(
                format!(
                    "trying to access field but base type '{}' is not struct like",
                    hir_type.display(&self.types.types_map)
                ),
                None
            )),
        }
    }

    fn lower_struct_contructor(
        &self,
        struct_id: StructId,
        ty: TypeId,
        body: &AggregateBody,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        match body {
            AggregateBody::Runtime(operands) => {
                self.lower_aggregate(struct_id, ty, operands, generics)
            }
            AggregateBody::Comptime(literals) => {
                self.lower_const_aggregate(struct_id, ty, literals, generics)
            }
        }
    }

    fn lower_aggregate(
        &self,
        struct_id: StructId,
        ty: TypeId,
        operands: &Vec<Operand>,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let struct_ir = self.get_or_create_struct(struct_id, generics)?;

        let mut fields = Vec::with_capacity(operands.len());
        for operand in operands {
            fields.push(self.lower_operand(operand, generics)?.value);
        }

        let ptr = self.builder.build_alloca(struct_ir, "tmp_struct")?;

        for (i, field) in fields.into_iter().enumerate() {
            self.builder.store_field(struct_ir, ptr, field, i)?;
        }

        self.new_unloaded_operand(ptr.into(), ty, generics)
    }

    pub(crate) fn lower_const_aggregate(
        &self,
        struct_id: StructId,
        ty: TypeId,
        literals: &Vec<(ComplexLiteral, TypeId)>,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let struct_ir = self.get_or_create_struct(struct_id, generics)?;

        let mut fields = Vec::with_capacity(literals.len());
        for (literal, ty) in literals {
            fields.push(self.lower_literal(literal, *ty, generics)?.value);
        }

        let aggregate = struct_ir.const_named_struct(fields.as_slice());
        self.new_loaded_operand(aggregate.into(), ty, generics)
    }

    pub(crate) fn lower_place_to_operand(
        &self,
        place: PlaceId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ty = self.mir.tree.places[place].ty;
        match &self.mir.tree.places[place].kind {
            mir::PlaceKind::Local(local_id) => {
                let local = self.get_local(*local_id);
                match local {
                    Local::Runtime(ptr) => self.new_loaded_operand(ptr.into(), ty, generics),
                    Local::Comptime(op) => Ok(op.clone()),
                }
            }
            mir::PlaceKind::Temp(temp_id) => {
                let temp_op = self.get_temp(*temp_id)?;
                Ok(temp_op.clone())
            }
            mir::PlaceKind::Deref(operand) => {
                let ty = self
                    .lower_type(operand.ty, generics)?
                    .unwrap_or(self.context.i8_type().into());

                let ptr_op = self.lower_operand(operand, generics)?;
                let ptr = ptr_op.value.into_pointer_value();
                let value = self.builder.build_load(ty, ptr, "load")?.into();
                self.new_loaded_operand(value, operand.ty, generics)
            }
            mir::PlaceKind::Field { struct_type:_, base, field_id } => {
                let field_info = &self.types.types_table.fields[*field_id];
                self.lower_field_access(*base, field_info, generics)
            }
        }
    }

    fn lower_stack_alloc(
        &self,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ir_type = self
            .lower_type(ty, generics)?
            .ok_or(soul_error_internal!("stackalloc type should be Some", None))?;

        let ptr = self.builder.build_alloca(ir_type, "rvalue")?.into();

        self.new_loaded_operand(ptr, ty, generics)
    }
}
