use crate::{GenericSubstitute, IrOperand, LlvmBackend, build_error};
use hir::TypeId;
use mir_parser::mir::{Rvalue, RvalueKind};
use soul_utils::{error::SoulResult, soul_error_internal};

mod binary_unary;
mod cast;
pub(crate) mod operand;

impl<'a> LlvmBackend<'a> {
    pub(crate) fn lower_rvalue(
        &self,
        value: &Rvalue,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        match &value.kind {
            RvalueKind::CastUse { value, cast_to } => self.lower_cast(value, *cast_to, generics),
            RvalueKind::Use(operand) => self.lower_operand(operand, generics),
            RvalueKind::Binary {
                left,
                operator,
                right,
            } => self.lower_binary(left, operator, right, generics),
            RvalueKind::Unary { operator, value } => self.lower_unary(value, operator, generics),
            RvalueKind::StackAlloc(ty) => self.lower_stack_alloc(*ty, generics),
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
