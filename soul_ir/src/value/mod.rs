use crate::{GenericSubstitute, IrOperand, LlvmBackend, Local, build_error};
use ast::Literal;
use hir::{StructId, TypeId};
use mir_parser::mir::{self, AggregateBody, Operand, PlaceId, Rvalue, RvalueKind};
use soul_utils::{error::SoulResult, soul_error_internal};

mod binary_unary;
mod cast;
pub(crate) mod operand;

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn lower_rvalue(
        &self,
        value: &Rvalue,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        match &value.kind {
            RvalueKind::Field { base, base_type, field_id:_, index } => {
                self.lower_field_access(*base, *base_type, *index, generics)
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
            RvalueKind::Aggregate { struct_type, body } => self.lower_struct_contructor(*struct_type, body, generics),
        }
    }

    fn lower_field_access(
        &self,
        base: PlaceId,
        base_type: TypeId,
        index: usize,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let base_operand = self.lower_place_to_operand(base, generics)?;
        let base_ptr = base_operand.value.into_pointer_value();

        self.expect_type_can_field(base_type)?;
        let base_type = self.lower_type(base_type, generics)?
            .ok_or(soul_error_internal!("none type found as base_type in field", None))?;

        let field = self.builder
            .build_struct_gep(base_type, base_ptr, index as u32, "gep_struct")
            .map_err(build_error)?;
        
        Ok(IrOperand {
            value: field.into(),
            is_signed_interger: false,
        })
    }

    fn expect_type_can_field(
        &self,
        base_type: TypeId,
    ) -> SoulResult<()> {
        
        let hir_type = self.get_type(base_type)?;
        match &hir_type.kind {
            hir::HirTypeKind::Struct(_) => Ok(()),
            _ => Err(soul_error_internal!(format!("trying to access field but base type '{}' is not struct like", hir_type.display(&self.types.types)), None)),
        }
    }

    fn lower_struct_contructor(
        &self, 
        struct_type: StructId, 
        body: &AggregateBody,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        match body {
            AggregateBody::Runtime(operands) => {
                self.lower_aggregate(struct_type, operands, generics)
            }
            AggregateBody::Comptime(literals) => {
                self.lower_const_aggregate(struct_type, literals, generics)
            }
        }
    }

    fn lower_aggregate(
        &self, 
        struct_id: StructId, 
        operands: &Vec<Operand>,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let struct_type = self.get_or_create_struct(struct_id, generics)?;

        let mut fields = Vec::with_capacity(operands.len());
        for operand in operands {
            fields.push(
                self.lower_operand(operand, generics)?.value
            );   
        }

        let aggregate = struct_type.const_named_struct(fields.as_slice());
        Ok(IrOperand{
            value: aggregate.into(),
            is_signed_interger: false,
        })
    }

    fn lower_const_aggregate(
        &self, 
        struct_id: StructId, 
        literals: &Vec<(Literal, TypeId)>,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let struct_type = self.get_or_create_struct(struct_id, generics)?;

        let mut fields = Vec::with_capacity(literals.len());
        for (literal, ty) in literals {
            fields.push(
                self.lower_literal(literal, *ty)?.value
            );   
        }

        let aggregate = struct_type.const_named_struct(fields.as_slice());
        Ok(IrOperand{
            value: aggregate.into(),
            is_signed_interger: false,
        })
    }

    fn lower_place_to_operand(
        &self,
        place: PlaceId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        match &self.mir.tree.places[place] {
            mir::Place::Local(local_id) => {
                let local = self.get_local(*local_id);
                match local {
                    Local::Runtime(ptr) => Ok(IrOperand { value: ptr.into(), is_signed_interger: false }),
                    Local::Comptime(op) => Ok(op.clone()),
                }
            }
            mir::Place::Temp(temp_id) => {
                let temp_op = self.get_temp(*temp_id)?;
                Ok(temp_op.clone())
            }
            mir::Place::Deref(operand) => {
                let ty = self.lower_type(operand.ty, generics)?
                    .unwrap_or(self.context.i8_type().into());
                
                let ptr_op = self.lower_operand(operand, generics)?;
                let ptr = ptr_op.value.into_pointer_value();
                Ok(IrOperand { 
                    value: self.builder.build_load(ty, ptr, "deref").map_err(build_error)?.into(), 
                    is_signed_interger: false 
                })
            }
            mir::Place::Field { base, base_type, field_id:_, index } => {
                self.lower_field_access(*base, *base_type, *index, generics)
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

        let ptr = self
            .builder
            .build_alloca(ir_type, "rvalue")
            .map_err(build_error)?
            .into();
        Ok(IrOperand {
            value: ptr,
            is_signed_interger: false,
        })
    }
}
