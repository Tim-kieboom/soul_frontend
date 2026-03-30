use inkwell::{FloatPredicate, IntPredicate, basic_block::BasicBlock, builder::{Builder, BuilderError}, context::Context, types::{BasicType, FloatMathType, IntMathType, PointerMathType}, values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, CallSiteValue, FloatMathValue, FunctionValue, InstructionValue, IntMathValue, IntValue, PointerMathValue, PointerValue}};
use soul_utils::error::{SoulError, SoulResult};
use typed_hir::FieldInfo;

use crate::{IrOperand};

pub struct IrBuilder<'ctx> {
    inkwell: Builder<'ctx>,
}
impl<'ctx> IrBuilder<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { inkwell: context.create_builder() }
    }

    pub fn build_unconditional_branch(&self, block: BasicBlock<'ctx>) -> SoulResult<()> {
        self
            .inkwell
            .build_unconditional_branch(block)
            .map_err(build_error)?;
        
        Ok(())
    }

    pub fn build_call(&self, function: FunctionValue<'ctx>, args: &[BasicMetadataValueEnum<'ctx>]) -> SoulResult<CallSiteValue<'ctx>> {
        self.inkwell
            .build_call(function, args, "exit_call")
            .map_err(build_error)
    }

    pub fn build_unreachable(&self) -> SoulResult<InstructionValue<'ctx>> {
        self.inkwell.build_unreachable().map_err(build_error)
    }

    pub fn build_return(&self, value: Option<&dyn BasicValue<'ctx>>) -> SoulResult<InstructionValue<'ctx>> {
        self.inkwell.build_return(value).map_err(build_error)
    }

    pub fn build_conditional_branch(&self, comparison: IntValue<'ctx>, then_block: BasicBlock<'ctx>, else_block: BasicBlock<'ctx>) -> SoulResult<InstructionValue<'ctx>> {
        self.inkwell.build_conditional_branch(comparison, then_block, else_block).map_err(build_error)
    }

    pub fn position_at_end(&self, block: BasicBlock<'ctx>) {
        self.inkwell.position_at_end(block);
    }

    pub fn store_parameter<V>(&self, ptr: PointerValue<'ctx>, value: V) -> SoulResult<InstructionValue<'ctx>> 
    where
        V: BasicValue<'ctx>
    {
        self.inkwell.build_store(ptr, value).map_err(build_error)
    }

    pub fn build_alloca<T>(&self, ty: T, name: &str) -> SoulResult<PointerValue<'ctx>> 
    where
        T: BasicType<'ctx>
    {
        self.inkwell.build_alloca(ty, name).map_err(build_error)
    }

    pub fn store_operand(&self, destination_ptr: PointerValue<'ctx>, operand: IrOperand<'ctx>) -> SoulResult<InstructionValue<'ctx>> {
        
        let value = if operand.info.is_unloaded {
            self.inkwell
                .build_load(operand.info.ir_type, operand.value.into_pointer_value(), "source_value")
                .map_err(build_error)?
        } else {
            operand.value
        };
        
        self.inkwell
            .build_store(destination_ptr, value)
            .map_err(build_error)
    }

    pub fn build_load<T>(&self, pointee_ty: T, ptr: PointerValue<'ctx>, name: &str) -> SoulResult<BasicValueEnum<'ctx>>
    where
        T: BasicType<'ctx>, 
    {
        self.inkwell.build_load(pointee_ty, ptr, name).map_err(build_error)
    }

    pub fn build_int_add<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_add(left, right, "add_int")
            .map_err(build_error)
    }

    pub fn build_float_add<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: FloatMathValue<'ctx>,
    {
        self.inkwell.build_float_add(left, right, "add_float")
            .map_err(build_error)
    }

    pub fn build_int_sub<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_sub(left, right, "sub_int")
            .map_err(build_error)
    }

    pub fn build_float_sub<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: FloatMathValue<'ctx>,
    {
        self.inkwell.build_float_sub(left, right, "sub_float")
            .map_err(build_error)
    }

    pub fn build_int_mul<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_mul(left, right, "mul_int")
            .map_err(build_error)
    }

    pub fn build_float_mul<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: FloatMathValue<'ctx>,
    {
        self.inkwell.build_float_mul(left, right, "mul_float")
            .map_err(build_error)
    }

    pub fn build_int_signed_div<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_signed_div(left, right, "div_int")
            .map_err(build_error)
    }

    pub fn build_int_unsigned_div<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_unsigned_div(left, right, "div_uint")
            .map_err(build_error)
    }

    pub fn build_float_div<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: FloatMathValue<'ctx>,
    {
        self.inkwell.build_float_div(left, right, "div_float")
            .map_err(build_error)
    }

    pub fn build_int_signed_rem<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_signed_rem(left, right, "rem_int")
            .map_err(build_error)
    }

    pub fn build_int_unsigned_rem<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_unsigned_rem(left, right, "rem_uint")
            .map_err(build_error)
    }

    pub fn build_and<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_and(left, right, "and_int")
            .map_err(build_error)
    }

    pub fn build_or<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_or(left, right, "or_int")
            .map_err(build_error)
    }

    pub fn build_xor<T>(&self, left: T, right: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_xor(left, right, "xor_int")
            .map_err(build_error)
    }

    pub fn build_int_compare<T>(&self, op: IntPredicate, left: T, right: T) -> SoulResult<<T::BaseType as IntMathType<'ctx>>::ValueType> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_compare(op, left, right, "cmp_int")
            .map_err(build_error)
    }

    pub fn build_float_compare<T>(&self, op: FloatPredicate, left: T, right: T) -> SoulResult<<<T::BaseType as FloatMathType<'ctx>>::MathConvType as IntMathType<'ctx>>::ValueType> 
    where
        T: FloatMathValue<'ctx>,
    {
        self.inkwell.build_float_compare(op, left, right, "cmp_float")
            .map_err(build_error)
    }

    pub fn build_int_neg<T>(&self, left: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_neg(left, "neg_int")
            .map_err(build_error)
    }

    pub fn build_float_neg<T>(&self, left: T) -> SoulResult<T> 
    where
        T: FloatMathValue<'ctx>,
    {
        self.inkwell.build_float_neg(left, "neg_float")
            .map_err(build_error)
    }

    pub fn build_not<T>(&self, left: T) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_not(left, "not_int")
            .map_err(build_error)
    }

    pub fn build_int_to_ptr<T>(&self, int: T, ptr_type: <T::BaseType as IntMathType<'ctx>>::PtrConvType) -> SoulResult<<<T::BaseType as IntMathType<'ctx>>::PtrConvType as PointerMathType<'ctx>>::ValueType>
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_to_ptr(int, ptr_type, "cast_int_to_ptr").map_err(build_error)
    }

    pub fn build_ptr_to_int<T>(&self, ptr: T, int_type: <T::BaseType as PointerMathType<'ctx>>::PtrConvType) -> SoulResult<<<T::BaseType as PointerMathType<'ctx>>::PtrConvType as IntMathType<'ctx>>::ValueType>
    where
        T: PointerMathValue<'ctx>,
    {
        self.inkwell.build_ptr_to_int(ptr, int_type, "cast_ptr_to_int").map_err(build_error)
    }

    pub fn build_int_s_extend<T>(&self, int_value: T, int_type: T::BaseType) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_s_extend(int_value, int_type, "cast_int_ext").map_err(build_error)
    }

    pub fn build_int_z_extend<T>(&self, int_value: T, int_type: T::BaseType) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_z_extend(int_value, int_type, "cast_uint_ext").map_err(build_error)
    }

    pub fn build_int_truncate<T>(&self, int_value: T, int_type: T::BaseType) -> SoulResult<T> 
    where
        T: IntMathValue<'ctx>,
    {
        self.inkwell.build_int_truncate(int_value, int_type, "cast_uint_turnc").map_err(build_error)
    }

    pub fn build_float_to_signed_int<T>(&self, float: T, int_type: <T::BaseType as FloatMathType<'ctx>>::MathConvType) -> SoulResult<<<T::BaseType as FloatMathType<'ctx>>::MathConvType as IntMathType<'ctx>>::ValueType>
    where
        T: FloatMathValue<'ctx>,
    {
        self.inkwell.build_float_to_signed_int(float, int_type, "cast_float_to_int").map_err(build_error)
    }

    pub fn build_float_to_unsigned_int<T>(&self, float: T, int_type: <T::BaseType as FloatMathType<'ctx>>::MathConvType) -> SoulResult<<<T::BaseType as FloatMathType<'ctx>>::MathConvType as IntMathType<'ctx>>::ValueType>
    where
        T: FloatMathValue<'ctx>,
    {
        self.inkwell.build_float_to_unsigned_int(float, int_type, "cast_float_to_uint").map_err(build_error)
    }

    pub fn build_signed_int_to_float<T>(&self, int: T, float_type: <T::BaseType as IntMathType<'ctx>>::MathConvType) -> SoulResult<<<T::BaseType as IntMathType<'ctx>>::MathConvType as FloatMathType<'ctx>>::ValueType>
    where
        T: IntMathValue<'ctx> 
    {
        self.inkwell.build_signed_int_to_float(int, float_type, "cast_int_to_float").map_err(build_error)
    }

    pub fn build_unsigned_int_to_float<T>(&self, int: T, float_type: <T::BaseType as IntMathType<'ctx>>::MathConvType) -> SoulResult<<<T::BaseType as IntMathType<'ctx>>::MathConvType as FloatMathType<'ctx>>::ValueType>
    where
        T: IntMathValue<'ctx> 
    {
        self.inkwell.build_unsigned_int_to_float(int, float_type, "cast_uint_to_float").map_err(build_error)
    }

    pub fn build_bit_cast<T, V>(&self, val: V, ty: T) -> SoulResult<BasicValueEnum<'ctx>>
    where
        T: BasicType<'ctx>,
        V: BasicValue<'ctx>, 
    {
        self.inkwell.build_bit_cast(val, ty, "bit_cast").map_err(build_error)
    }

    pub fn build_float_ext<T>(&self, float: T, float_type: T::BaseType) -> SoulResult<T>
    where
        T: FloatMathValue<'ctx> 
    {
        self.inkwell.build_float_ext(float, float_type, "cast_float_ext").map_err(build_error)
    }

    pub fn build_float_trunc<T>(&self, float: T, float_type: T::BaseType) -> SoulResult<T>
    where
        T: FloatMathValue<'ctx> 
    {
        self.inkwell.build_float_trunc(float, float_type, "cast_float_trunc").map_err(build_error)
    }

    pub fn build_field_access<T, F>(&self, base_type: T, field_type: F, base_ptr: PointerValue<'ctx>, field_info: &FieldInfo) -> SoulResult<PointerValue<'ctx>>
    where 
        T: BasicType<'ctx>,
        F: BasicType<'ctx> + Copy,
    {
        let field_ptr = self.inkwell
            .build_struct_gep(base_type, base_ptr, field_info.field_index as u32, "gep_struct")
            .map_err(build_error)?;

        let loaded_value = self.inkwell
            .build_load(field_type, field_ptr, "load_field")
            .map_err(build_error)?;

        let field_alloca = self.inkwell
            .build_alloca(field_type, "field_tmp")
            .map_err(build_error)?;

        self.inkwell
            .build_store(field_alloca, loaded_value)
            .map_err(build_error)?;

        Ok(field_alloca)
    } 

    pub fn store_field<T, V>(&self, struct_ir: T, ptr: PointerValue<'ctx>, field: V, field_index: usize) -> SoulResult<InstructionValue<'ctx>>
    where
        T: BasicType<'ctx>,
        V: BasicValue<'ctx>,
    {
        let geb = self.inkwell.build_struct_gep(
            struct_ir,
            ptr,
            field_index as u32,
            "gep_x",
        ).map_err(build_error)?;

        self.inkwell.build_store(geb, field).map_err(build_error)
    }
}

/// From [`BuilderError`] to [`SoulError`] of [`soul_utils::error::SoulErrorKind::LlvmError`]
fn build_error(value: BuilderError) -> SoulError {
    SoulError::new(
        value.to_string(),
        soul_utils::error::SoulErrorKind::LlvmError,
        None,
    )
}