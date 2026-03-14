use hir::TypeId;
use inkwell::{
    AddressSpace,
    types::{BasicType, BasicTypeEnum},
};
use soul_utils::{error::SoulResult, soul_names::PrimitiveTypes};

use crate::LlvmBackend;

impl<'a> LlvmBackend<'a> {
    pub fn lower_type(&self, ty: TypeId) -> SoulResult<Option<BasicTypeEnum<'a>>> {
        let hir_type = self.get_type(ty)?;

        Ok(Some(match hir_type.kind {
            hir::HirTypeKind::Primitive(primitive_types) => {
                match self.lower_primitive_type(primitive_types) {
                    Some(val) => val,
                    None => return Ok(None),
                }
            }

            hir::HirTypeKind::Ref { .. } | hir::HirTypeKind::Pointer(_) => {
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                ptr_type.into()
            }
            hir::HirTypeKind::Optional(type_id) => {
                let element_type = match self.lower_type(type_id)? {
                    Some(ty) => ty,
                    None => self.context.i8_type().into(),
                };
                let is_null = self.context.bool_type().into();
                self.context
                    .struct_type(&[is_null, element_type], false)
                    .into()
            }
            hir::HirTypeKind::Array { element, kind } => {
                let ptr_type = self.context.ptr_type(AddressSpace::default()).into();
                let len_type = self
                    .context
                    .ptr_sized_int_type(&self.target_data, None)
                    .into();

                match kind {
                    ast::ArrayKind::StackArray(num) => {
                        let element_type = match self.lower_type(element)? {
                            Some(ty) => ty,
                            None => self.context.i8_type().into(),
                        };
                        element_type.array_type(num as u32).into()
                    }
                    ast::ArrayKind::MutSlice
                    | ast::ArrayKind::HeapArray
                    | ast::ArrayKind::ConstSlice => self
                        .context
                        .struct_type(&[ptr_type, len_type], false)
                        .into(),
                }
            }

            hir::HirTypeKind::None | hir::HirTypeKind::Type | hir::HirTypeKind::InferType(_, _) => {
                return Ok(None);
            }

            hir::HirTypeKind::Error => panic!("error type should not be in ir"),
        }))
    }

    fn lower_primitive_type(&self, primitive: PrimitiveTypes) -> Option<BasicTypeEnum<'a>> {
        Some(match primitive {
            PrimitiveTypes::None => return None,

            PrimitiveTypes::Char
            | PrimitiveTypes::Int8
            | PrimitiveTypes::Uint8
            | PrimitiveTypes::Char8 => self.context.i8_type().into(),
            PrimitiveTypes::Boolean => self.context.bool_type().into(),

            PrimitiveTypes::Int16 | PrimitiveTypes::Char16 | PrimitiveTypes::Uint16 => {
                self.context.i16_type().into()
            }

            PrimitiveTypes::Int32 | PrimitiveTypes::Char32 | PrimitiveTypes::Uint32 => {
                self.context.i32_type().into()
            }

            PrimitiveTypes::Int
            | PrimitiveTypes::Uint
            | PrimitiveTypes::UntypedInt
            | PrimitiveTypes::UntypedUint => self
                .context
                .ptr_sized_int_type(&self.target_data, None)
                .into(),

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
