use std::{cell::RefMut, collections::HashMap};

use ast::{ArrayKind, Literal};
use hir::{ComplexLiteral, CustomTypeId, TypeId};
use inkwell::{
    AddressSpace,
    module::Linkage,
    types::{ArrayType, BasicType, StructType},
    values::{
        ArrayValue, AsValueRef, BasicValue, BasicValueEnum, GlobalValue, PointerValue, StructValue,
    },
};
use mir_parser::mir::{Operand, OperandKind, PlaceId, PlaceKind};
use soul_utils::{error::SoulResult, soul_error_internal, soul_names::PrimitiveSize};
use typed_hir::{ThirTypeKind, display_thir::DisplayThirType};

use crate::{GenericSubstitute, IrOperand, LlvmBackend, OperandInfo};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn lower_operand(
        &self,
        operand: &Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        Ok(match &operand.kind {
            OperandKind::Nullptr => {
                let ptr = self.context.ptr_type(AddressSpace::default());
                let value = ptr.const_null().into();
                IrOperand {
                    value,
                    info: crate::OperandInfo::new_loaded(operand.ty, ptr.into()),
                }
            }
            OperandKind::Sizeof(ty) => {
                let size = self.sizeof(*ty, generics)?;
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
                #[cfg(debug_assertions)]
                panic!();

                #[cfg(not(debug_assertions))]
                {
                    let id = self.current.function_key();
                    return Err(soul_error_internal!(
                        format!(
                            "operand should be Some(_) {:?}",
                            self.function_keys.id_to_key(id)
                        ),
                        None
                    ));
                }
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
                // For Int(0), try the normal lowering but fall back to zeroinitializer
                // for non-primitive types (used by union constructor unused variant fields)
                if matches!(literal, Literal::Int(0)) {
                    match self.lower_basic_literal(literal, should_be, generics) {
                        Ok(val) => return Ok(val),
                        Err(_) => {
                            let ir_type = self.lower_type(should_be, generics)?.ok_or(
                                soul_error_internal!("zeroinitializer type not found", None),
                            )?;
                            let value = ir_type.const_zero().into();
                            return self.new_loaded_operand(value, should_be, generics);
                        }
                    }
                }
                self.lower_basic_literal(literal, should_be, generics)
            }
            ComplexLiteral::Array { array_type, values } => {
                let hir_array_type = self.get_type_kind(*array_type)?;
                let element_type = match hir_array_type {
                    ThirTypeKind::Array { element, .. } => element,
                    _ => {
                        return Err(soul_error_internal!(
                            "arrayType can only be of ThirTypeKind::Array",
                            None
                        ));
                    }
                };

                let hir_should_type = self.get_type_kind(should_be)?;
                match hir_should_type {
                    ThirTypeKind::Array { element, .. }
                        if self.get_type_kind(*element)?
                            == self.get_type_kind(*element_type)? =>
                    {
                        ()
                    }
                    _ => {
                        let array_str = hir_array_type.display(&self.types.types_map);
                        let should_str = hir_should_type.display(&self.types.types_map);
                        return Err(soul_error_internal!(
                            format!(
                                "should type does not match array type {array_str} != {should_str}",
                            ),
                            None
                        ));
                    }
                }

                self.create_const_array(*array_type, *element_type, values, generics)
            }
            ComplexLiteral::Struct {
                struct_id,
                struct_type,
                values,
                all_fields_const: _,
            } => {
                let hir_should_type = self.get_type(should_be)?;
                match hir_should_type.kind {
                    ThirTypeKind::CustomTypes(CustomTypeId::Struct(id)) if id == *struct_id => (),
                    _ => {
                        let struct_str =
                            self.get_type(*struct_type)?.display(&self.types.types_map);
                        let should_str = hir_should_type.display(&self.types.types_map);
                        return Err(soul_error_internal!(
                            format!(
                                "should type does not match struct type {struct_str} != {should_str}",
                            ),
                            None
                        ));
                    }
                }

                let struct_ir = self.get_or_create_struct(*struct_id, generics)?;
                self.lower_const_aggregate(struct_ir, *struct_type, values, generics)
            }
        }
    }

    fn lower_ref(&self, place: PlaceId, generics: &GenericSubstitute) -> SoulResult<IrOperand<'a>> {
        // For Deref places, the inner operand is the pointer we want the address of.
        // Skip the generic lower_place_to_operand path (which loads the *value*)
        // and store the pointer directly into a new alloca.
        if let PlaceKind::Deref(operand) = &self.mir.tree.places[place].kind {
            return self.deref_place(operand, place, generics);
        }

        let inner = self.lower_place_to_operand(place, generics)?;
        let ty = self.mir.tree.places[place].ty;
        let hir_type = self.get_type(ty)?;

        Ok(match hir_type.kind {
            ThirTypeKind::Array {
                kind: ArrayKind::HeapArray,
                ..
            } => {
                let ptr = inner.value.into_pointer_value();
                let loaded = self
                    .builder
                    .build_load(inner.info.ir_type, ptr, "heap_slice")?;
                IrOperand {
                    value: loaded,
                    info: OperandInfo::new_loaded(inner.info.type_id, inner.info.ir_type),
                }
            }
            ThirTypeKind::Array {
                kind: ArrayKind::StackArray(len),
                ..
            } => {
                let ptr = inner.value.into_pointer_value();
                self.fixed_array_to_slice(ty, ptr, len, generics)?
            }
            _ => {
                if inner.value.is_pointer_value() {
                    let ptr = inner.value.into_pointer_value();
                    let ptr_type = ptr.get_type();
                    let new_ptr = self.builder.build_alloca(ptr_type, "ref_ptr")?;
                    let operand_to_store = IrOperand {
                        value: ptr.into(),
                        info: OperandInfo::new_loaded(inner.info.type_id, ptr_type.into()),
                    };
                    self.builder.store_operand(new_ptr, operand_to_store)?;
                    let info = OperandInfo::new_unloaded(ty, ptr_type.into());
                    return Ok(IrOperand {
                        value: new_ptr.into(),
                        info,
                    });
                }
                let value = unsafe { BasicValueEnum::new(inner.value.as_value_ref()) };
                IrOperand {
                    value,
                    info: inner.info.clone(),
                }
            }
        })
    }

    /// For Deref places, use the inner pointer operand directly
    /// (the pointer IS the address of the deref'd location).
    fn deref_place(
        &self,
        operand: &Operand,
        place: PlaceId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ptr_op = self.lower_operand(operand, generics)?;
        let ptr = ptr_op.value.into_pointer_value();
        let ptr_type = ptr.get_type();
        let new_ptr = self.builder.build_alloca(ptr_type, "ref_ptr")?;
        let operand_to_store = IrOperand {
            value: ptr.into(),
            info: OperandInfo::new_loaded(ptr_op.info.type_id, ptr_type.into()),
        };
        self.builder.store_operand(new_ptr, operand_to_store)?;
        let ty = self.mir.tree.places[place].ty;
        let info = OperandInfo::new_unloaded(ty, ptr_type.into());
        Ok(IrOperand {
            value: new_ptr.into(),
            info,
        })
    }

    // Lowers a basic (non-aggregate) literal to LLVM IR.
    //
    // TYPE DETERMINATION:
    // The `should_be` parameter is the expression's type from typed_hIR (after type
    // inference/unification). This type is what determines the LLVM integer type used:
    // - should_be = i32  → generates i32 constant
    // - should_be = i64  → generates i64 constant
    //
    // This is why casting in MIR is important - if the type isn't correct here,
    // the wrong LLVM constant type will be generated.
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
                    ThirTypeKind::CustomTypes(hir::CustomTypeId::Enum(enum_id)) => {
                        self.get_enum_size(enum_id)
                    }
                    other => {
                        return Err(soul_error_internal!(
                            format!(
                                "literal should be primitive type not {}",
                                other.display_variant()
                            ),
                            None
                        ));
                    }
                };

                let negative = *value < 0;
                let int_type = match size {
                    PrimitiveSize::CIntSize => self.default_c_int_type,
                    PrimitiveSize::CharSize => self.default_char_type,
                    PrimitiveSize::IntAndPtrSize => self.default_int_type,
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
                    PrimitiveSize::IntAndPtrSize => self.default_int_type,
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
            ast::Literal::Cstr(text) => {
                let ptr = self.const_string_ptr(&text);
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                IrOperand {
                    value: ptr.into(),
                    info: crate::OperandInfo::new_loaded(should_be, ptr_type.into()),
                }
            }
            ast::Literal::Str(text) => {
                let (ty, value) = self.const_string_slice(&text, generics);
                IrOperand {
                    value: value.into(),
                    info: crate::OperandInfo::new_loaded(should_be, ty.into()),
                }
            }
        })
    }

    fn const_string_ptr(&self, text: &String) -> PointerValue<'a> {
        let strings = self.strings.borrow_mut();
        let global = match strings.get(text).copied() {
            Some(val) => val,
            None => self.create_global_string(text, strings),
        };

        global.as_basic_value_enum().into_pointer_value()
    }

    fn const_string_slice(
        &self,
        text: &String,
        generics: &GenericSubstitute,
    ) -> (StructType<'a>, StructValue<'a>) {
        let ptr = self.const_string_ptr(text);
        self.fixed_array_to_const_slice(ptr, text.len() as u64, generics)
    }

    fn create_global_string(
        &self,
        text: &str,
        mut strings: RefMut<'_, HashMap<String, GlobalValue<'a>>>,
    ) -> GlobalValue<'a> {
        let bytes = self.context.const_string(text.as_bytes(), true);
        let array_ty = bytes.get_type();

        let global = self.module.add_global(array_ty, None, "str");
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.set_initializer(&bytes);

        strings.insert(text.to_string(), global);
        global
    }

    fn fixed_array_to_slice(
        &self,
        slice_type_id: TypeId,
        ptr: PointerValue<'a>,
        len: u64,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let array_struct = self.types.types_map.array_struct;
        let slice_type = self.get_or_create_struct(array_struct, generics)?;

        let slice_ptr = self.builder.build_alloca(slice_type, "slice")?;
        let len_val = self.default_int_type.const_int(len, false);

        let ptr: BasicValueEnum<'a> = ptr.into();
        let len_val: BasicValueEnum<'a> = len_val.into();
        self.builder.store_field(slice_type, slice_ptr, ptr, 0)?;
        self.builder
            .store_field(slice_type, slice_ptr, len_val, 1)?;

        Ok(IrOperand {
            value: slice_ptr.into(),
            info: OperandInfo::new_unloaded(slice_type_id, slice_type.into()),
        })
    }

    fn fixed_array_to_const_slice(
        &self,
        ptr: PointerValue<'a>,
        len: u64,
        generics: &GenericSubstitute,
    ) -> (StructType<'a>, StructValue<'a>) {
        let len = self.default_int_type.const_int(len, false);

        let array_struct = self.types.types_map.array_struct;
        let slice_ty = self
            .get_or_create_struct(array_struct, generics)
            .expect("___Array struct should lower");

        (
            slice_ty,
            slice_ty.const_named_struct(&[ptr.into(), len.into()]),
        )
    }

    fn create_const_array(
        &self,
        array_type_id: TypeId,
        element_type_id: TypeId,
        values: &Vec<hir::ComplexLiteral>,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let array_type = self.resolve_array_type(array_type_id, generics)?;

        let mut elements: Vec<BasicValueEnum<'a>> = Vec::with_capacity(values.len());
        for value in values {
            let operand = self.lower_literal(value, element_type_id, generics)?;
            elements.push(operand.value);
        }

        let array_value = unsafe { ArrayValue::new_const_array(&array_type, &elements) };

        Ok(IrOperand {
            value: array_value.into(),
            info: OperandInfo::new_loaded(array_type_id, array_type.into()),
        })
    }

    fn resolve_array_type(
        &self,
        type_id: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<ArrayType<'a>> {
        let ty = self.get_type(type_id)?;
        let (element, len) = match &ty.kind {
            ThirTypeKind::Array {
                element,
                kind: ArrayKind::StackArray(len),
            } => (element, len),
            _ => {
                return Err(soul_error_internal!(
                    "arrayType should be of ThirTypeKind::Array{kind: ArrayKind::StackArray, ..}",
                    None
                ));
            }
        };

        let ir_type = self
            .lower_type(*element, generics)?
            .ok_or(soul_error_internal!(
                "elementType of array should be Some(_)",
                None
            ))?;

        Ok(ir_type.array_type(*len as u32))
    }
}
