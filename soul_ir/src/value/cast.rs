use ast::ArrayKind;
use hir::TypeId;
use inkwell::{AddressSpace, types::BasicTypeEnum, values::BasicValueEnum};
use mir_parser::mir;
use soul_utils::{error::SoulResult, soul_error_internal, soul_names::PrimitiveTypes};
use typed_hir::{ThirType, ThirTypeKind};

use crate::{GenericSubstitute, IrOperand, LlvmBackend};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(super) fn lower_cast(
        &self,
        value: &mir::Operand,
        cast_to: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let source_operand = self.lower_operand(value, generics)?;
        let source_value = if source_operand.info.is_unloaded {
            let ptr = source_operand.value.into_pointer_value();
            self.builder
                .build_load(source_operand.info.ir_type, ptr, "cast_source")?
        } else {
            source_operand.value
        };
        let cast_type = match self.lower_type(cast_to, generics)? {
            Some(val) => val,
            None => self.context.i8_type().into(),
        };

        let mir_source_type = self.get_type(value.ty)?;
        let mir_cast_type = self.get_type(cast_to)?;
        match (mir_source_type.kind, mir_cast_type.kind) {
            (ThirTypeKind::Array { kind: ArrayKind::StackArray(_), .. }, ThirTypeKind::Pointer(_)) => {
                let value = source_operand.get_or_convert_pointer(&self.builder)?.into();
                Ok(IrOperand { value, info: crate::OperandInfo::new_loaded(cast_to, cast_type) })
            }
            (ThirTypeKind::Pointer(_), ThirTypeKind::Pointer(_)) => {
                //llvm doesn't care ptr's are ptr's
                self.new_loaded_operand(source_value, cast_to, generics)
            }
            (ThirTypeKind::Primitive(_), ThirTypeKind::Pointer(_)) => {
                // int → ptr
                let (source, cast) = (
                    source_value.into_int_value(),
                    cast_type.into_pointer_type(),
                );
                let res = self.builder.build_int_to_ptr(source, cast)?;

                self.new_loaded_operand(res.into(), cast_to, generics)
            }
            (ThirTypeKind::Pointer(_), ThirTypeKind::Primitive(_)) => {
                // ptr → int
                let (source, cast) = (
                    source_value.into_pointer_value(),
                    cast_type.into_int_type(),
                );
                let res = self.builder.build_ptr_to_int(source, cast)?;

                self.new_loaded_operand(res.into(), cast_to, generics)
            }
            (ThirTypeKind::Primitive(_), ThirTypeKind::Primitive(_)) => {
                let info = self.get_primitive_cast_info(
                    mir_source_type,
                    mir_cast_type,
                    cast_to,
                    IrOperand {
                        value: source_value,
                        info: source_operand.info,
                    },
                    cast_type,
                )?;
                self.cast_primitives(info, generics)
            }
            (ThirTypeKind::Array { .. }, ThirTypeKind::Pointer(_)) => {
                let base_ptr = source_operand.get_or_convert_pointer(&self.builder)?;
                if source_operand.info.ir_type.is_struct_type() {
                    let struct_ty = source_operand.info.ir_type.into_struct_type();
                    let data_ptr_ptr = self.builder.build_struct_gep_index(
                        struct_ty,
                        base_ptr,
                        0,
                        "array_data_ptr",
                    )?;
                    let loaded_data_ptr = self
                        .builder
                        .build_load(cast_type.into_pointer_type(), data_ptr_ptr, "array_data")?
                        .into_pointer_value();
                    let info = crate::OperandInfo::new_loaded(cast_to, cast_type);
                    Ok(IrOperand {
                        value: loaded_data_ptr.into(),
                        info,
                    })
                } else {
                    let ptr_type = self.context.ptr_type(AddressSpace::default()).into();
                    let info = crate::OperandInfo::new_loaded(cast_to, ptr_type);
                    Ok(IrOperand {
                        value: base_ptr.into(),
                        info,
                    })
                }
            }
            _ => Err(soul_error_internal!(
                format!(
                    "types can not be type cast\nsource: {:#?}\ncast: {:#?}",
                    mir_source_type, mir_cast_type
                ),
                None
            )),
        }
    }

    fn cast_primitives(
        &self,
        info: PrimCastInfo<'a>,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let value = if info.source_size == info.cast_size {
            self.same_size_cast(&info)?
        } else if info.both_are_float {
            self.float_float_cast(&info)?
        } else if info.one_is_float {
            self.int_float_cast(&info)?
        } else if info.source_size < info.cast_size {
            self.int_extend(&info)?
        } else {
            self.int_trunc(&info)?
        };

        self.new_loaded_operand(value, info.cast_type_id, generics)
    }

    fn int_extend(&self, info: &PrimCastInfo<'a>) -> SoulResult<BasicValueEnum<'a>> {
        // int widening: zext or sext
        let (source, cast) = (
            info.source_operand.value.into_int_value(),
            info.cast_type.into_int_type(),
        );
        let res = if info.source_prim.can_be_negative() {
            self.builder.build_int_s_extend(source, cast)?
        } else {
            self.builder.build_int_z_extend(source, cast)?
        };

        Ok(res.into())
    }

    fn int_trunc(&self, info: &PrimCastInfo<'a>) -> SoulResult<BasicValueEnum<'a>> {
        let (source, cast) = (
            info.source_operand.value.into_int_value(),
            info.cast_type.into_int_type(),
        );
        let res = self.builder.build_int_truncate(source, cast)?;
        Ok(res.into())
    }

    fn int_float_cast(&self, info: &PrimCastInfo<'a>) -> SoulResult<BasicValueEnum<'a>> {
        if info.source_prim.is_float() {
            // float -> int
            let (source, cast) = (
                info.source_operand.value.into_float_value(),
                info.cast_type.into_int_type(),
            );
            let result = if info.cast_prim.can_be_negative() {
                self.builder.build_float_to_signed_int(source, cast)?
            } else {
                self.builder.build_float_to_unsigned_int(source, cast)?
            };

            Ok(result.into())
        } else {
            // int -> float
            let (source, cast) = (
                info.source_operand.value.into_int_value(),
                info.cast_type.into_float_type(),
            );
            let result = if info.source_prim.can_be_negative() {
                self.builder.build_signed_int_to_float(source, cast)?
            } else {
                self.builder.build_unsigned_int_to_float(source, cast)?
            };

            Ok(result.into())
        }
    }

    fn same_size_cast(&self, info: &PrimCastInfo<'a>) -> SoulResult<BasicValueEnum<'a>> {
        if info.one_is_float {
            self.builder
                .build_bit_cast(info.source_operand.value, info.cast_type)
        } else {
            Ok(info.source_operand.value)
        }
    }

    fn float_float_cast(&self, info: &PrimCastInfo<'a>) -> SoulResult<BasicValueEnum<'a>> {
        let (source, cast) = (
            info.source_operand.value.into_float_value(),
            info.cast_type.into_float_type(),
        );
        let float = if info.source_size < info.cast_size {
            self.builder.build_float_ext(source, cast)?
        } else {
            self.builder.build_float_trunc(source, cast)?
        };

        Ok(float.into())
    }

    fn get_primitive_cast_info(
        &self,
        mir_source_type: &ThirType,
        mir_cast_type: &ThirType,
        mir_cast_type_id: TypeId,
        source_operand: IrOperand<'a>,
        destination_type: BasicTypeEnum<'a>,
    ) -> SoulResult<PrimCastInfo<'a>> {
        let (source_prim, cast_prim) = match (mir_source_type.kind, mir_cast_type.kind) {
            (ThirTypeKind::Primitive(source), ThirTypeKind::Primitive(cast)) => (source, cast),
            _ => unreachable!(),
        };

        let int_size = self.default_int_size;
        let char_size = self.default_char_size;
        let c_int_size = self.default_c_int_size;
        Ok(PrimCastInfo {
            cast_prim,
            source_prim,
            source_operand,
            cast_type: destination_type,
            cast_type_id: mir_cast_type_id,
            cast_size: cast_prim.to_size_bit_u8(c_int_size, int_size, char_size),
            one_is_float: source_prim.is_float() != cast_prim.is_float(),
            source_size: source_prim.to_size_bit_u8(c_int_size, int_size, char_size),
            both_are_float: source_prim.is_float() && cast_prim.is_float(),
        })
    }
}

struct PrimCastInfo<'a> {
    /// both are floating point
    both_are_float: bool,
    /// one is floating point other interger
    one_is_float: bool,

    /// source type size
    cast_size: u8,
    /// cast type size
    source_size: u8,

    cast_type_id: TypeId,
    cast_prim: PrimitiveTypes,
    source_prim: PrimitiveTypes,
    cast_type: BasicTypeEnum<'a>,
    source_operand: IrOperand<'a>,
}
