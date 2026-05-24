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

use crate::{GenericSubstitute, LlvmBackend, OperandInfo, value::sizeof::Alignment};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub fn lower_type(
        &self,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<Option<BasicTypeEnum<'a>>> {
        if let Some(ir_type) = self.lowered_types.borrow().get(ty).copied() {
            return Ok(ir_type);
        }

        let hir_type = self.get_type(ty)?;

        let ir_type = match hir_type.kind {
            ThirTypeKind::Generic(id) => {
                let ty = generics.resolve(id).ok_or(soul_error_internal!(
                    format!("generic {:?} substitute type not found", ty),
                    None
                ))?;

                self.lower_type(ty, generics)?
            }
            ThirTypeKind::CustomTypes(id) => Some(match id {
                hir::CustomTypeId::Struct(struct_id) => {
                    let s = self.lower_struct(struct_id, generics)?;
                    <inkwell::types::BasicTypeEnum as From<inkwell::types::StructType>>::from(s)
                }
                hir::CustomTypeId::Enum(enum_id) => self.lower_enum(enum_id).into(),
                hir::CustomTypeId::Union(union_id) => {
                    let union_info = self.types.types_map.id_to_union(union_id)
                        .ok_or(soul_error_internal!(
                            format!("union {:?} not found in ThirTypesMap", union_id),
                            None
                        ))?;

                        let index_ty = self.types.types_table.index_type;
                    let tag_ir = match self.lower_type(index_ty, generics)? {
                        Some(val) => val,
                        None => self.default_int_type.into(),
                    };

                    let mut max_bits = 0u32;
                    let mut max_align = Alignment::Null;
                    for &variant_type in &union_info.variant_types {
                        let field = self.sizeof_bit(variant_type, generics)?;
                        if field.bits > max_bits {
                            max_bits = field.bits;
                        }
                        if field.alignment > max_align {
                            max_align = field.alignment;
                        }
                    }

                    let elem_ty: inkwell::types::BasicTypeEnum<'a> = match max_align {
                        Alignment::Null | Alignment::Bit8 => self.context.i8_type().into(),
                        Alignment::Bit16 => self.context.i16_type().into(),
                        Alignment::Bit32 => self.context.i32_type().into(),
                        Alignment::Bit64 => self.context.i64_type().into(),
                    };
                    let elem_bits = max_align.as_u32();
                    let elem_bits = if elem_bits == 0 { 8 } else { elem_bits };
                    let array_count = if max_bits == 0 {
                        0
                    } else {
                        (max_bits + elem_bits - 1) / elem_bits
                    };
                    let array_ty = elem_ty.array_type(array_count);
                    let struct_ty = self.context.struct_type(&[tag_ir, array_ty.into()], false);
                    <inkwell::types::BasicTypeEnum as From<inkwell::types::StructType>>::from(struct_ty)
                }
            }),
            ThirTypeKind::Primitive(primitive_types) => self.lower_primitive_type(primitive_types),

            ThirTypeKind::Pointer(_) => {
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                Some(ptr_type.into())
            }
            ThirTypeKind::Ref { of_type, .. } => {
                let array_struct = self.types.types_map.array_struct;
                let pointee = self.get_type(of_type)?;
                match &pointee.kind {
                    ThirTypeKind::Array {
                        kind: ast::ArrayKind::StackArray(_) | ast::ArrayKind::HeapArray,
                        ..
                    } => Some(self.get_or_create_struct(array_struct, generics)?.into()),
                    _ => {
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        Some(ptr_type.into())
                    }
                }
            }
            ThirTypeKind::Optional(type_id) => {
                let element_type = match self.lower_type(type_id, generics)? {
                    Some(ty) => ty,
                    None => self.context.i8_type().into(),
                };
                let is_null = self.context.bool_type().into();
                Some(
                    self.context
                        .struct_type(&[is_null, element_type], false)
                        .into(),
                )
            }
            ThirTypeKind::Array { element, kind } => {
                let array_struct = self.types.types_map.array_struct;

                match kind {
                    ast::ArrayKind::StackArray(num) => {
                        let element_type = match self.lower_type(element, generics)? {
                            Some(ty) => ty,
                            None => self.context.i8_type().into(),
                        };
                        Some(element_type.array_type(num as u32).into())
                    }
                    ast::ArrayKind::MutSlice
                    | ast::ArrayKind::HeapArray
                    | ast::ArrayKind::ConstSlice => {
                        Some(self.get_or_create_struct(array_struct, generics)?.into())
                    }
                }
            }
            ThirTypeKind::None | ThirTypeKind::Type | ThirTypeKind::Never => None,
            ThirTypeKind::Error => {
                #[cfg(debug_assertions)]
                panic!("error type should not be in ir");
                #[cfg(not(debug_assertions))]
                return Err(soul_error_internal!("error type should not be in ir", None));
            }
        };

        self.lowered_types.borrow_mut().insert(ty, ir_type);
        Ok(ir_type)
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
