use ast::ArrayKind;
use hir::{StructId, TypeId};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_error_internal,
};
use typed_hir::{Field, ThirTypeKind, display_thir::DisplayThirType};

use crate::{LlvmBackend, utils::GenericSubstitute};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn sizeof(&self, sizeof: TypeId, generics: &GenericSubstitute) -> SoulResult<u32> {
        self.sizeof_bit(sizeof, generics)
            .map(|sizeof| sizeof.bits / 8)
    }

    pub(crate) fn sizeof_bit(
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
        let ptr_align = Alignment::from_bits(ptr).expect("should be value in alignment");

        Ok(match sizeof.kind {
            ThirTypeKind::Error | ThirTypeKind::Type | ThirTypeKind::Never => {
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
                bits: 0,
                alignment: Alignment::Null,
            },
            ThirTypeKind::Primitive(primitive_types) => {
                let size =
                    primitive_types.to_size_bit_u8(c_int as u8, int as u8, char as u8) as u32;
                let alignment = Alignment::from_bits(size).expect(&format!(
                    "should be value in alignment, size: {}, type {:?}",
                    size, primitive_types
                ));

                Sizeof {
                    bits: size,
                    alignment,
                }
            }
            ThirTypeKind::Array { kind, element } => {
                let size = match kind {
                    ArrayKind::StackArray(num) => {
                        num as u32 * self.sizeof_bit(element, generics)?.bits
                    }
                    _ => int + ptr,
                };
                Sizeof {
                    bits: size,
                    alignment: ptr_align,
                }
            }
            ThirTypeKind::Ref { .. } | ThirTypeKind::Pointer(_) => Sizeof {
                bits: ptr,
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
                self.sizeof_bit(ty, generics)?
            }
            ThirTypeKind::CustomTypes(id) => {
                match id {
                    hir::CustomTypeId::Struct(struct_id) => {
                        self.sizeof_struct(struct_id, generics)?
                    }
                    hir::CustomTypeId::Enum(_) => todo!(),
                    hir::CustomTypeId::Union(union_id) => {
                        let union_info = self.types.types_map.id_to_union(union_id).ok_or(
                            soul_error_internal!(format!("union {:?} not found", union_id), None),
                        )?;

                        let index_type_id = self.types.types_table.index_type;
                        let tag = self.sizeof_bit(index_type_id, generics)?;
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
                        let elem_bits = max_align.as_u32();
                        let elem_bits = if elem_bits == 0 { 8 } else { elem_bits };
                        let array_count = if max_bits == 0 {
                            0
                        } else {
                            (max_bits + elem_bits - 1) / elem_bits
                        };
                        let array_bits = array_count * elem_bits;
                        let total_align = if tag.alignment > max_align {
                            tag.alignment
                        } else {
                            max_align
                        };
                        let padding = total_align.get_padding(tag.bits);
                        let unaligned_bits = tag.bits + padding + array_bits;
                        let total = (unaligned_bits + total_align.as_u32() - 1)
                            / total_align.as_u32()
                            * total_align.as_u32();
                        Sizeof {
                            bits: total,
                            alignment: total_align,
                        }
                    }
                    hir::CustomTypeId::Trait(_) => {
                        return Err(soul_error_internal!(
                            format!("traits have no runtime size"),
                            None
                        ));
                    }
                }
            }
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
            let inner_alignment = self.sizeof_bit(field.ty, generics)?.alignment;

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
            let field = self.sizeof_bit(*ty, generics)?;

            if !is_packed {
                let padding = field.alignment.get_padding(offset);
                offset += padding;
            }

            offset += field.bits;
            size = offset;
        }

        if !is_packed {
            let align = alignment.as_u32();
            size = (size + align - 1) / align * align;
        }

        Ok(Sizeof {
            bits: size,
            alignment,
        })
    }
}

pub(crate) struct Sizeof {
    pub bits: u32,
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
    const fn from_bits(size: u32) -> Option<Self> {
        match size {
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
