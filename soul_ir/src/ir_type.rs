use hir::TypeId;
use inkwell::{
    AddressSpace,
    types::{BasicType, BasicTypeEnum},
};
use soul_utils::{error::SoulResult, soul_error_internal, soul_names::PrimitiveTypes};

use crate::{GenericSubstitute, LlvmBackend};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub fn lower_type(
        &self,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<Option<BasicTypeEnum<'a>>> {
        let hir_type = self.get_type(ty)?;

        Ok(Some(match hir_type.kind {
            hir::HirTypeKind::Generic(id) => {
                let ty = generics.resolve(id).ok_or(soul_error_internal!(
                    format!("generic {:?} substitute type not found", ty),
                    None
                ))?;

                return self.lower_type(ty, generics);
            }
            hir::HirTypeKind::Struct(id) => {
                let obj = self.types.types.id_to_struct(id).expect("should have struct");
                if !obj.generics.is_empty() {
                    todo!()
                }

                let mut fields = vec![];
                for field in &obj.fields {
                    let ty = self.types.types.ref_to_id(field.ty).expect("should hev type");
                    let field = match self.lower_type(ty, generics)? {
                        Some(val) => val,
                        None => continue,
                    };
                    
                    fields.push(field);
                }

                self.context.struct_type(fields.as_slice(), false).into()
            }
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
                let element_type = match self.lower_type(type_id, generics)? {
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
                let len_type = self.default_int_type.into();

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
                    | ast::ArrayKind::ConstSlice => self
                        .context
                        .struct_type(&[ptr_type, len_type], false)
                        .into(),
                }
            }

            hir::HirTypeKind::None | hir::HirTypeKind::Type => {
                return Ok(None);
            }
            
            hir::HirTypeKind::InferType(_, _) => panic!("inferType type should not be in ir"),
            hir::HirTypeKind::Error => panic!("error type should not be in ir"),
        }))
    }

    fn lower_primitive_type(&self, primitive: PrimitiveTypes) -> Option<BasicTypeEnum<'a>> {
        Some(match primitive {
            PrimitiveTypes::None => return None,

            PrimitiveTypes::Char => self.default_char_type.into(),

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
