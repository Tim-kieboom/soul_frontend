use hir::{HirType, HirTypeKind, TypeId};
use inkwell::{types::BasicTypeEnum, values::BasicValueEnum};
use mir_parser::mir;
use soul_utils::{error::SoulResult, soul_error_internal, soul_names::PrimitiveTypes};

use crate::{GenericSubstitute, IrOperand, LlvmBackend, build_error};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(super) fn lower_cast(
        &self,
        value: &mir::Operand,
        cast_to: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let source_operand = self.lower_operand(value, generics)?;
        let cast_type = match self.lower_type(cast_to, generics)? {
            Some(val) => val,
            None => self.context.i8_type().into(),
        };

        let mir_source_type = self.get_type(value.ty)?;
        let mir_cast_type = self.get_type(cast_to)?;
        match (mir_source_type.kind, mir_cast_type.kind) {
            (HirTypeKind::Pointer(_), HirTypeKind::Pointer(_)) => {
                //llvm doesn't care ptr's are ptr's
                Ok(source_operand)
            }
            (HirTypeKind::Primitive(_), HirTypeKind::Pointer(_)) => {
                // int → ptr
                let (source, cast) = (
                    source_operand.value.into_int_value(),
                    cast_type.into_pointer_type(),
                );
                let res = self
                    .builder
                    .build_int_to_ptr(source, cast, "castNumPtr")
                    .map_err(build_error)?;
                Ok(IrOperand {
                    value: res.into(),
                    is_signed_interger: false,
                })
            }
            (HirTypeKind::Pointer(_), HirTypeKind::Primitive(_)) => {
                // ptr → int
                let (source, cast) = (
                    source_operand.value.into_pointer_value(),
                    cast_type.into_int_type(),
                );
                let res = self
                    .builder
                    .build_ptr_to_int(source, cast, "castPtrNum")
                    .map_err(build_error)?;
                Ok(IrOperand {
                    value: res.into(),
                    is_signed_interger: false,
                })
            }
            (HirTypeKind::Primitive(_), HirTypeKind::Primitive(_)) => {
                let info = self.get_primitive_cast_info(
                    mir_source_type,
                    mir_cast_type,
                    source_operand,
                    cast_type,
                )?;
                self.cast_primitives(info)
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

    fn cast_primitives(&self, info: PrimCastInfo<'a>) -> SoulResult<IrOperand<'a>> {
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

        let is_signed_interger = info.cast_prim.is_signed_interger();
        Ok(IrOperand {
            value,
            is_signed_interger,
        })
    }

    fn int_extend(&self, info: &PrimCastInfo<'a>) -> SoulResult<BasicValueEnum<'a>> {
        // int widening: zext or sext
        let (source, cast) = (
            info.source_operand.value.into_int_value(),
            info.cast_type.into_int_type(),
        );
        let res = if info.source_prim.can_be_negative() {
            self.builder
                .build_int_s_extend(source, cast, "castIntExt")
                .map_err(build_error)?
        } else {
            self.builder
                .build_int_z_extend(source, cast, "castUintExt")
                .map_err(build_error)?
        };

        Ok(res.into())
    }

    fn int_trunc(&self, info: &PrimCastInfo<'a>) -> SoulResult<BasicValueEnum<'a>> {
        let (source, cast) = (
            info.source_operand.value.into_int_value(),
            info.cast_type.into_int_type(),
        );
        let res = self
            .builder
            .build_int_truncate(source, cast, "castIntTrunc")
            .map_err(build_error)?;
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
                self.builder
                    .build_float_to_signed_int(source, cast, "castFloatInt")
            } else {
                self.builder
                    .build_float_to_unsigned_int(source, cast, "castFloatUint")
            }
            .map_err(build_error)?;

            Ok(result.into())
        } else {
            // int -> float
            let (source, cast) = (
                info.source_operand.value.into_int_value(),
                info.cast_type.into_float_type(),
            );
            let result = if info.source_prim.can_be_negative() {
                self.builder
                    .build_signed_int_to_float(source, cast, "castIntFloat")
            } else {
                self.builder
                    .build_unsigned_int_to_float(source, cast, "castUintFloat")
            }
            .map_err(build_error)?;

            Ok(result.into())
        }
    }

    fn same_size_cast(&self, info: &PrimCastInfo<'a>) -> SoulResult<BasicValueEnum<'a>> {
        if info.one_is_float {
            self.builder
                .build_bit_cast(info.source_operand.value, info.cast_type, "castBitcast")
                .map_err(build_error)
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
            self.builder
                .build_float_ext(source, cast, "castFloatExt")
                .map_err(build_error)?
        } else {
            self.builder
                .build_float_trunc(source, cast, "castFloatTrunc")
                .map_err(build_error)?
        };

        Ok(float.into())
    }

    fn get_primitive_cast_info(
        &self,
        mir_source_type: &HirType,
        mir_cast_type: &HirType,
        source_operand: IrOperand<'a>,
        destination_type: BasicTypeEnum<'a>,
    ) -> SoulResult<PrimCastInfo<'a>> {
        let (source_prim, cast_prim) = match (mir_source_type.kind, mir_cast_type.kind) {
            (HirTypeKind::Primitive(source), HirTypeKind::Primitive(cast)) => (source, cast),
            _ => unreachable!(),
        };

        let int_size = self.default_int_size;
        let char_size = self.default_char_size;
        Ok(PrimCastInfo {
            cast_prim,
            source_prim,
            source_operand,
            cast_type: destination_type,
            cast_size: cast_prim.to_size_bit_u8(int_size, char_size),
            one_is_float: source_prim.is_float() != cast_prim.is_float(),
            source_size: source_prim.to_size_bit_u8(int_size, char_size),
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

    cast_prim: PrimitiveTypes,
    source_prim: PrimitiveTypes,
    cast_type: BasicTypeEnum<'a>,
    source_operand: IrOperand<'a>,
}
