use ast::Literal;
use hir::{ComplexLiteral, TypeId};
use inkwell::values::{AsValueRef, BasicValueEnum};
use mir_parser::mir::{Operand, OperandKind};
use soul_utils::{error::SoulResult, soul_error_internal, soul_names::PrimitiveSize};
use typed_hir::{ThirTypeKind, display_thir::DisplayThirType};

use crate::{GenericSubstitute, IrOperand, LlvmBackend};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn lower_operand(
        &self,
        operand: &Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        Ok(match &operand.kind {
            OperandKind::Temp(temp_id) => self.get_temp(*temp_id)?,
            OperandKind::Local(local_id) => {
                let mir_local = &self.mir.tree.locals[*local_id];

                let ty = match self.lower_type(mir_local.ty(), generics)? {
                    Some(val) => val,
                    None => self.context.i8_type().into(),
                };

                let local = self.get_local(*local_id);

                let ptr = match local {
                    crate::Local::Runtime(val) => val,
                    crate::Local::Comptime(literal_operand) => return Ok(literal_operand),
                };

                let value = self.builder.build_load(ty, ptr, "load")?;

                self.new_loaded_operand(value, mir_local.ty(), generics)?
            }
            OperandKind::Comptime(literal) => self.lower_literal(literal, operand.ty, generics)?,
            OperandKind::Ref { place, .. } => {
                let inner = self.lower_place_to_operand(*place, generics)?;
                IrOperand { value: unsafe { BasicValueEnum::new(inner.value.as_value_ref()) }, info: inner.info.clone() }
            }
            OperandKind::None => {
                return Err(soul_error_internal!("operand should be Some(_)", None));
            }
        })
    }

    pub(crate) fn lower_literal(
        &self,
        literal: &ComplexLiteral,
        should_be: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        match literal {
            ComplexLiteral::Basic(literal) => self.lower_basic_literal(literal, should_be, generics),
            ComplexLiteral::Struct { struct_id, struct_type, values, all_fields_const:_ } => {
                self.lower_const_aggregate(*struct_id, *struct_type, values, generics)
            },
        }
    }

    fn lower_basic_literal(
        &self,
        literal: &Literal,
        should_be: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        Ok(match literal {
            ast::Literal::Int(value) => {
                let size = match self
                    .types
                    .types_map
                    .id_to_type(should_be)
                    .expect("should have type")
                    .kind
                {
                    ThirTypeKind::Primitive(primitive_types) => primitive_types.to_primitive_size(),
                    _ => {
                        return Err(soul_error_internal!(
                            "literal should be primitive type",
                            None
                        ));
                    }
                };

                let negative = *value < 0;
                let int_type = match size {
                    PrimitiveSize::CharSize => self.default_char_type,
                    PrimitiveSize::IntSize => self.default_int_type,
                    PrimitiveSize::Bit8 => self.context.i8_type(),
                    PrimitiveSize::Bit16 => self.context.i16_type(),
                    PrimitiveSize::Bit32 => self.context.i32_type(),
                    PrimitiveSize::Bit64 => self.context.i64_type(),
                    PrimitiveSize::Bit128 => self.context.i128_type(),
                };

                let value = int_type.const_int(*value as u64, negative).into();

                self.new_loaded_operand(value, should_be, generics)?
            }
            ast::Literal::Uint(value) => {
                let hir_type = self
                    .types
                    .types_map
                    .id_to_type(should_be)
                    .expect("should have type");

                let size = match hir_type.kind {
                    ThirTypeKind::Primitive(primitive_types) => primitive_types.to_primitive_size(),
                    _ => {
                        return Err(soul_error_internal!(
                            format!(
                                "literal should be primitive type is `{}`",
                                hir_type.display(&self.types.types_map)
                            ),
                            None
                        ));
                    }
                };

                let int_type = match size {
                    PrimitiveSize::CharSize => self.default_char_type,
                    PrimitiveSize::IntSize => self.default_int_type,
                    PrimitiveSize::Bit8 => self.context.i8_type(),
                    PrimitiveSize::Bit16 => self.context.i16_type(),
                    PrimitiveSize::Bit32 => self.context.i32_type(),
                    PrimitiveSize::Bit64 => self.context.i64_type(),
                    PrimitiveSize::Bit128 => self.context.i128_type(),
                };

                let value = int_type.const_int(*value as u64, false).into();

                self.new_loaded_operand(value, should_be, generics)?
            }
            ast::Literal::Float(value) => {
                let size = match self
                    .types
                    .types_map
                    .id_to_type(should_be)
                    .expect("should have type")
                    .kind
                {
                    ThirTypeKind::Primitive(primitive_types) => primitive_types.to_primitive_size(),
                    _ => {
                        return Err(soul_error_internal!(
                            "literal should be primitive type",
                            None
                        ));
                    }
                };

                let int_type = match size {
                    PrimitiveSize::Bit16 => self.context.f16_type(),
                    PrimitiveSize::Bit32 => self.context.f32_type(),
                    PrimitiveSize::Bit64 => self.context.f64_type(),
                    PrimitiveSize::Bit128 => self.context.f128_type(),
                    _ => self.context.f32_type(),
                };
                let value = int_type.const_float(*value).into();

                self.new_loaded_operand(value, should_be, generics)?
            }
            ast::Literal::Bool(value) => {
                let value = self
                    .context
                    .bool_type()
                    .const_int(*value as u64, false)
                    .into();

                self.new_loaded_operand(value, should_be, generics)?
            }
            ast::Literal::Char(value) => {
                let value = self
                    .context
                    .i8_type()
                    .const_int(*value as u64, false)
                    .into();

                self.new_loaded_operand(value, should_be, generics)?
            }
            ast::Literal::Str(_) => {
                todo!("string literal not yet impl")
            }
        })
    }
}
