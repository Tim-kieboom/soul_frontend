use hir::{StructId, TypeId};
use inkwell::{
    AddressSpace,
    types::{BasicType, BasicTypeEnum, IntType, StructType},
};
use soul_utils::{
    error::SoulResult,
    soul_error_internal,
    soul_names::{PrimitiveSize, PrimitiveTypes},
};
use typed_hir::ThirTypeKind;

use crate::{GenericSubstitute, LlvmBackend, OperandInfo};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub fn lower_type(
        &self,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<Option<BasicTypeEnum<'a>>> {
        let hir_type = self.get_type(ty)?;

        Ok(Some(match hir_type.kind {
            ThirTypeKind::Generic(id) => {
                let ty = generics.resolve(id).ok_or(soul_error_internal!(
                    format!("generic {:?} substitute type not found", ty),
                    None
                ))?;

                return self.lower_type(ty, generics);
            }
            ThirTypeKind::CustomTypes(id) => match id {
                hir::CustomTypeId::Struct(struct_id) => {
                    self.lower_struct(struct_id, generics).map(|s| s.into())?
                }
                hir::CustomTypeId::Enum(enum_id) => self.lower_enum(enum_id).into(),
            },
            ThirTypeKind::Primitive(primitive_types) => {
                match self.lower_primitive_type(primitive_types) {
                    Some(val) => val,
                    None => return Ok(None),
                }
            }

            ThirTypeKind::Ref { .. } | ThirTypeKind::Pointer(_) => {
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                ptr_type.into()
            }
            ThirTypeKind::Optional(type_id) => {
                let element_type = match self.lower_type(type_id, generics)? {
                    Some(ty) => ty,
                    None => self.context.i8_type().into(),
                };
                let is_null = self.context.bool_type().into();
                self.context
                    .struct_type(&[is_null, element_type], false)
                    .into()
            }
            ThirTypeKind::Array { element, kind } => {
                let array_struct = self.types.types_map.array_struct;

                match kind {
                    ast::ArrayKind::StackArray(num) => {
                        let element_type = match self.lower_type(element, generics)? {
                            Some(ty) => ty,
                            None => self.context.i8_type().into(),
                        };
                        element_type.array_type(num as u32).into()
                    }
                    ast::ArrayKind::MutSlice
                    | ast::ArrayKind::HeapArray
                    | ast::ArrayKind::ConstSlice => {
                        self.get_or_create_struct(array_struct, generics)?.into()
                    }
                }
            }

            ThirTypeKind::None | ThirTypeKind::Type => {
                return Ok(None);
            }

            ThirTypeKind::Error => {
                #[cfg(debug_assertions)]
                panic!("error type should not be in ir");
                #[cfg(not(debug_assertions))]
                return Err(soul_error_internal!("error type should not be in ir", None));
            }
        }))
    }

    pub(crate) fn get_or_create_struct(
        &self,
        id: StructId,
        generics: &GenericSubstitute,
    ) -> SoulResult<StructType<'a>> {
        match self.structs.get(id) {
            Some(val) => Ok(val),
            None => self.lower_struct(id, generics),
        }
    }

    pub(crate) fn is_signed_interger(&self, info: &OperandInfo) -> bool {
        let ty = match self.get_type(info.type_id) {
            Ok(val) => val,
            Err(_) => return false,
        };

        ty.is_any_int_type()
    }

    pub(crate) fn lower_struct(
        &self,
        id: StructId,
        generics: &GenericSubstitute,
    ) -> SoulResult<StructType<'a>> {
        let object = self
            .types
            .types_map
            .id_to_struct(id)
            .expect("should have struct");

        let mut fields = vec![];
        for (i, field) in object.fields.iter().enumerate() {
            let ty = field.ty;
            let ir_field = match self.lower_type(ty, generics)? {
                Some(val) => val,
                None => continue,
            };

            self.field_indexs.borrow_mut().insert(field.id, i);
            fields.push(ir_field);
        }

        let ty = self.context.struct_type(fields.as_slice(), object.packed);
        self.structs.insert(id, ty);
        Ok(ty)
    }

    pub(crate) fn lower_enum(&self, id: hir::EnumId) -> IntType<'a> {
        match self.get_enum_size(id) {
            PrimitiveSize::Bit8 => self.context.i8_type(),
            PrimitiveSize::Bit16 => self.context.i16_type(),
            PrimitiveSize::Bit32 => self.context.i32_type(),
            PrimitiveSize::Bit64 => self.context.i64_type(),
            PrimitiveSize::Bit128 => self.context.i128_type(),
            PrimitiveSize::CIntSize => self.default_c_int_type,
            PrimitiveSize::CharSize => self.default_char_type,
            PrimitiveSize::IntAndPtrSize => self.default_int_type,
        }
    }

    pub(crate) fn get_enum_size(&self, id: hir::EnumId) -> PrimitiveSize {
        if let Some(object) = self.types.types_map.id_to_enum(id) {
            let variant_count = object.variants.len() as u64;
            let bit_width = variant_count.next_power_of_two().max(8);
            let ty = match bit_width {
                1..=8 => PrimitiveSize::Bit8,
                9..=16 => PrimitiveSize::Bit16,
                17..=32 => PrimitiveSize::Bit32,
                _ => PrimitiveSize::Bit64,
            };
            ty
        } else {
            PrimitiveSize::Bit32
        }
    }

    fn lower_primitive_type(&self, primitive: PrimitiveTypes) -> Option<BasicTypeEnum<'a>> {
        Some(match primitive {
            PrimitiveTypes::None => return None,

            PrimitiveTypes::Char => self.default_char_type.into(),
            PrimitiveTypes::CStr => self.context.ptr_type(AddressSpace::default()).into(),

            PrimitiveTypes::Int8 | PrimitiveTypes::Uint8 | PrimitiveTypes::Char8 => {
                self.context.i8_type().into()
            }
            PrimitiveTypes::Boolean => self.context.bool_type().into(),

            PrimitiveTypes::Int16 | PrimitiveTypes::Char16 | PrimitiveTypes::Uint16 => {
                self.context.i16_type().into()
            }

            PrimitiveTypes::Int32 | PrimitiveTypes::Char32 | PrimitiveTypes::Uint32 => {
                self.context.i32_type().into()
            }

            PrimitiveTypes::CInt | PrimitiveTypes::CUint => self.default_c_int_type.into(),

            PrimitiveTypes::Int
            | PrimitiveTypes::Uint
            | PrimitiveTypes::UntypedInt
            | PrimitiveTypes::UntypedUint => self.default_int_type.into(),
            PrimitiveTypes::Int64 | PrimitiveTypes::Char64 | PrimitiveTypes::Uint64 => {
                self.context.i64_type().into()
            }

            PrimitiveTypes::Int128 | PrimitiveTypes::Uint128 => self.context.i128_type().into(),

            PrimitiveTypes::Float16 => self.context.bf16_type().into(),

            PrimitiveTypes::Float32 | PrimitiveTypes::UntypedFloat => {
                self.context.f32_type().into()
            }

            PrimitiveTypes::Float64 => self.context.f64_type().into(),
        })
    }
}
