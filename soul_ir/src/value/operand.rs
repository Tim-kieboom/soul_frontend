use ast::{ArrayKind, Literal};
use hir::{ComplexLiteral, StructId, TypeId};
use inkwell::{AddressSpace, module::Linkage, values::{AsValueRef, BasicValue, BasicValueEnum}};
use mir_parser::mir::{Operand, OperandKind, PlaceId};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_error_internal,
    soul_names::PrimitiveSize,
};
use typed_hir::{Field, ThirTypeKind, display_thir::DisplayThirType};

use crate::{GenericSubstitute, IrOperand, LlvmBackend};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn lower_operand(
        &self,
        operand: &Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        Ok(match &operand.kind {
            OperandKind::Sizeof(ty) => {
                let Sizeof { size, alignment: _ } = self.sizeof(*ty, generics)?;
                let value = self.context.i32_type().const_int(size as u64, false).into();
                let u32 = self.types.types_table.u32_type;
                let ir_u32 = self
                    .lower_type(u32, generics)?
                    .ok_or(soul_error_internal!("u32 have none type", None))?;

                IrOperand {
                    value,
                    info: crate::OperandInfo::new_loaded(u32, ir_u32),
                }
            }
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
            OperandKind::Ref { place, .. } => self.lower_ref(*place, generics)?,
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
            ComplexLiteral::Basic(literal) => {
                self.lower_basic_literal(literal, should_be, generics)
            }
            ComplexLiteral::Struct {
                struct_id,
                struct_type,
                values,
                all_fields_const: _,
            } => {
                let struct_ir = self.get_or_create_struct(*struct_id, generics)?;
                self.lower_const_aggregate(struct_ir, *struct_type, values, generics)
            }
        }
    }

    fn lower_ref(&self, place: PlaceId, generics: &GenericSubstitute) -> SoulResult<IrOperand<'a>> {
        let inner = self.lower_place_to_operand(place, generics)?;
        let ty = self.mir.tree.places[place].ty;
        let hir_type = self.get_type(ty)?;

        Ok(match hir_type.kind {
            ThirTypeKind::Array {
                kind: ArrayKind::HeapArray,
                ..
            } => {
                todo!()
            }
            ThirTypeKind::Array {
                kind: ArrayKind::StackArray(_),
                ..
            } => {
                todo!()
            }
            _ => {
                let value = unsafe { BasicValueEnum::new(inner.value.as_value_ref()) };
                IrOperand {
                    value,
                    info: inner.info.clone(),
                }
            }
        })
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
                    PrimitiveSize::CIntSize => self.default_c_int_type,
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
                    PrimitiveSize::CIntSize => self.default_c_int_type,
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
            ast::Literal::Str(text) => {
                let bytes = self.context.const_string(text.as_bytes(), false);
                let array_ty = bytes.get_type();
                let global = self.module.add_global(array_ty, None, "str");
                global.set_constant(true);
                global.set_linkage(Linkage::Private);
                global.set_initializer(&bytes);

                let ptr = self.builder.build_pointer_cast(global.as_basic_value_enum().into_pointer_value(), self.context.ptr_type(AddressSpace::default()), "str_ptr")?;
                let len = self.default_int_type.const_int(text.len() as u64, false).into();
                let slice_ty = self.context.struct_type(&[self.context.ptr_type(AddressSpace::default()).into(), self.default_int_type.into()], false);
                self.lower_aggregate(slice_ty, should_be, &[ptr.into(), len], generics)?
            }
        })
    }

    pub(crate) fn sizeof(
        &self,
        sizeof: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<Sizeof> {
        let sizeof = self.get_type(sizeof)?;

        if !sizeof.generics.is_empty() {
            todo!("impl generic sizeof")
        }

        let c_int = self.default_c_int_size as u32;
        let int = self.default_int_size as u32;
        let ptr = self.default_ptr_size as u32;
        let char = self.default_char_size as u32;
        let ptr_align = Alignment::from_u8(ptr as u8).expect("should be value in alignment");

        Ok(match sizeof.kind {
            ThirTypeKind::Error | ThirTypeKind::Type => {
                return Err(SoulError::new(
                    format!(
                        "type '{}' does not have a size",
                        sizeof.display(&self.types.types_map)
                    ),
                    SoulErrorKind::InvalidContext,
                    None,
                ));
            }

            ThirTypeKind::None => Sizeof {
                size: 0,
                alignment: Alignment::Null,
            },
            ThirTypeKind::Primitive(primitive_types) => {
                let size = primitive_types.to_size_bit_u8(c_int as u8, int as u8, char as u8) as u32;
                let alignment =
                    Alignment::from_u8(size as u8).expect("should be value in alignment");
                Sizeof { size, alignment }
            }
            ThirTypeKind::Array { kind, element } => {
                let size = match kind {
                    ArrayKind::StackArray(num) => num as u32 * self.sizeof(element, generics)?.size,
                    _ => int + ptr,
                };
                Sizeof {
                    size,
                    alignment: ptr_align,
                }
            }
            ThirTypeKind::Ref { .. } | ThirTypeKind::Pointer(_) => Sizeof {
                size: ptr,
                alignment: ptr_align,
            },
            ThirTypeKind::Optional(_) => todo!("impl"),
            ThirTypeKind::Generic(generic_id) => {
                let ty = match generics.resolve(generic_id) {
                    Some(val) => val,
                    None => {
                        return Err(SoulError::new(
                            "generic not found",
                            SoulErrorKind::TypeNotFound,
                            None,
                        ));
                    }
                };
                self.sizeof(ty, generics)?
            }
            ThirTypeKind::Struct(struct_id) => self.sizeof_struct(struct_id, generics)?,
        })
    }

    fn sizeof_struct(
        &self,
        struct_id: StructId,
        generics: &GenericSubstitute,
    ) -> SoulResult<Sizeof> {
        let struct_type =
            self.types
                .types_map
                .id_to_struct(struct_id)
                .ok_or(soul_error_internal!(
                    format!("{:?} not found", struct_id),
                    None
                ))?;

        let is_packed = struct_type.packed;

        let mut alignment = Alignment::Null;
        for field in &struct_type.fields {
            let inner_alignment = self.sizeof(field.ty, generics)?.alignment;

            if alignment < inner_alignment {
                alignment = inner_alignment;
                if inner_alignment == Alignment::max() {
                    break;
                }
            }
        }

        let mut offset = 0u32;
        let mut size = 0u32;

        for Field { ty, .. } in &struct_type.fields {
            let field = self.sizeof(*ty, generics)?;

            if !is_packed {
                let padding = field.alignment.get_padding(offset);
                offset += padding;
            }

            offset += field.size;
            size = offset;
        }

        if !is_packed {
            let align = alignment.as_u32();
            size = (size + align - 1) / align * align;
        }

        Ok(Sizeof { size, alignment })
    }
}

pub(crate) struct Sizeof {
    pub size: u32,
    pub alignment: Alignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Alignment {
    Null = 0,
    Bit8 = 8,
    Bit16 = 16,
    Bit32 = 32,
    Bit64 = 64,
}
impl Alignment {
    const fn from_u8(sizeof_bit: u8) -> Option<Self> {
        match sizeof_bit {
            0 => Some(Self::Null),
            8 => Some(Self::Bit8),
            16 => Some(Self::Bit16),
            32 => Some(Self::Bit32),
            64 => Some(Self::Bit64),
            _ => None,
        }
    }

    const fn get_padding(self, offset: u32) -> u32 {
        let align = self.as_u32();
        (align - (offset % align)) % align
    }

    const fn max() -> Self {
        Self::Bit64
    }

    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}
