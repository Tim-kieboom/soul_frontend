use crate::{GenericSubstitute, IrOperand, LlvmBackend, Local, OperandInfo};
use ast::ArrayKind;
use hir::{ComplexLiteral, StructId, TypeId};
use inkwell::{types::StructType, values::BasicValueEnum, AddressSpace};
use mir_parser::mir::{self, AggregateBody, Place, PlaceId, Rvalue, RvalueKind};
use soul_utils::{
    Span, error::{SoulError, SoulErrorKind, SoulResult}, soul_error_internal
};
use typed_hir::{FieldInfo, ThirTypeKind, display_thir::DisplayThirType};

pub(crate) mod binary_unary;
pub(crate) mod cast;
pub(crate) mod operand;

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(crate) fn lower_rvalue(
        &self,
        value: &Rvalue,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<Option<IrOperand<'a>>> {
        Ok(Some(match &value.kind {
            RvalueKind::Place(place) => self.lower_rvalue_place(place, generics)?,
            RvalueKind::CastUse { value, cast_to } => self.lower_cast(value, *cast_to, generics)?,
            RvalueKind::Operand(operand) => self.lower_operand(operand, generics)?,
            RvalueKind::Binary {
                left,
                operator,
                right,
            } => self.lower_binary(left, operator, right, generics)?,
            RvalueKind::Unary { operator, value } => self.lower_unary(value, operator, generics)?,
            RvalueKind::StackAlloc(ty) => self.lower_stack_alloc(*ty, generics)?,
            RvalueKind::Aggregate { struct_type, body } => {
                self.lower_struct_contructor(ty, *struct_type, body, generics)?
            }
            RvalueKind::PtrOffset { pointer, offset } => {
                self.lower_ptr_offset(pointer, offset, generics)?
            }
            RvalueKind::StackArrayIndex { array, index } => {
                self.lower_stack_array_index(array, index, ty, generics)?
            }
            RvalueKind::HeapAlloc { ty: inner_type, count } => {
                self.lower_heap_alloc(ty, *inner_type, *count, generics)?
            }
            RvalueKind::Drop { value, span } => {
                self.lower_drop(value, *span, generics)?;
                return Ok(None);
            }
        }))
    }

    pub(crate) fn new_loaded_operand(
        &self,
        value: BasicValueEnum<'a>,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ir_type = self
            .lower_type(ty, generics)?
            .unwrap_or(self.context.i8_type().into());

        Ok(IrOperand {
            value,
            info: OperandInfo::new_loaded(ty, ir_type),
        })
    }

    pub(crate) fn new_unloaded_operand(
        &self,
        value: BasicValueEnum<'a>,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ir_type = self
            .lower_type(ty, generics)?
            .unwrap_or(self.context.i8_type().into());

        Ok(IrOperand {
            value,
            info: OperandInfo::new_unloaded(ty, ir_type),
        })
    }

    fn lower_heap_alloc(&self, ty: TypeId, inner_type: TypeId, count: u64, generics: &GenericSubstitute) -> SoulResult<IrOperand<'a>> {
        let size_info = self.sizeof_bit(inner_type, generics)?;
        let size_bits = size_info.size as u64;

        let eight = self.default_int_type.const_int(8, false);
        let size_bits_val = self.default_int_type.const_int(size_bits, false);
        let size_bytes = self.builder.build_int_unsigned_div(size_bits_val, eight)?;

        let total_size = if count > 1 {
            let count_val = self.default_int_type.const_int(count, false);
            self.builder.build_int_mul(size_bytes, count_val)?
        } else {
            size_bytes
        };

        let malloc_fn = self.malloc_function.ok_or_else(|| {
            soul_error_internal!("malloc function not declared", None)
        })?;
        let call = self.builder.build_call(malloc_fn, &[total_size.into()])?;
        let ptr = call.try_as_basic_value().basic().ok_or_else(|| {
            soul_error_internal!("malloc call returned no value", None)
        })?;

        self.new_loaded_operand(ptr, ty, generics)
    }

    fn lower_drop(&self, value: &mir::Operand, span: Span, generics: &GenericSubstitute) -> SoulResult<()> {
        let value_op = self.lower_operand(value, generics)?;
        let hir_type = self.get_type(value.ty)?;

        match &hir_type.kind {
            ThirTypeKind::Pointer(_) => {
                let ptr = if value_op.value.is_pointer_value() {
                    value_op.value.into_pointer_value()
                } else {
                    return Err(soul_error_internal!("Drop: expected pointer value", Some(span)));
                };

                let Some(free_fn) = self.free_function else {
                    return Err(soul_error_internal!("free function not declared", Some(span)))
                };

                self.builder.build_call(free_fn, &[ptr.into()])?;
            }
            ThirTypeKind::Array {
                kind: ast::ArrayKind::HeapArray,
                ..
            } => {
                let heap_struct_ir_type = value_op.info.ir_type;
                let data_ptr = if value_op.info.is_unloaded {
                    let ptr = value_op.value.into_pointer_value();

                    let data_ptr_ptr = self.builder.build_struct_gep_index(
                        heap_struct_ir_type.into_struct_type(),
                        ptr,
                        0,
                        "drop_heap_array_ptr",
                    )?;
                    
                    self.builder.build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "drop_heap_array_data",
                    )?.into_pointer_value()

                } else {

                    if !value_op.value.is_struct_value() {
                        return Err(soul_error_internal!("Drop: expected struct value for loaded heap array", Some(span)));
                    }

                    let struct_val = value_op.value.into_struct_value();
                    let struct_ty = heap_struct_ir_type.into_struct_type();
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let ptr_alloca = self.builder.build_alloca(struct_ty, "drop_heap_array_temp")?;
                    let _ = self.builder.inkwell().build_store(ptr_alloca, struct_val);
                    let data_ptr_ptr = self.builder.build_struct_gep_index(
                        struct_ty,
                        ptr_alloca,
                        0,
                        "drop_heap_array_data_ptr",
                    )?;
                    self.builder.build_load(ptr_type, data_ptr_ptr, "drop_heap_array_data")?.into_pointer_value()
                };
                let free_fn = self.free_function.ok_or_else(|| {
                    soul_error_internal!("free function not declared", Some(span))
                })?;
                self.builder.build_call(free_fn, &[data_ptr.into()])?;
            }
            _ => (),
        }

        Ok(())
    }

    fn lower_rvalue_place(
        &self,
        place: &Place,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        match &place.kind {
            mir::PlaceKind::Field {
                base,
                field_id,
                struct_type: _,
            } => {
                let field_info = &match self.types.types_table.fields.get(*field_id) {
                    Some(val) => val,
                    None => {
                        return Err(soul_error_internal!(
                            format!("fieldId: {:?} not found", field_id),
                            None
                        ));
                    }
                };
                self.lower_field_access(*base, field_info, generics)
            }
            mir::PlaceKind::Deref(operand) => {
                let ir_type = self
                    .lower_type(place.ty, generics)?
                    .unwrap_or(self.context.i8_type().into());

                let ptr_op = self.lower_operand(operand, generics)?;
                let ptr = ptr_op.value.into_pointer_value();
                let value = self.builder.build_load(ir_type, ptr, "load")?.into();
                self.new_loaded_operand(value, place.ty, generics)
            }
            mir::PlaceKind::Temp(temp_id) => self.get_temp(*temp_id),
            mir::PlaceKind::Local(local_id) => {
                let local = self.get_local(*local_id);
                let ptr = match local {
                    Local::Runtime(ptr) => ptr,
                    Local::Comptime(op) => return Ok(op.clone()),
                };
                let hir_type = self.get_type(place.ty)?;
                match &hir_type.kind {
                    ThirTypeKind::Ref { .. } | ThirTypeKind::Pointer(_) => {
                        let ir_type = self
                            .lower_type(place.ty, generics)?
                            .unwrap_or(self.context.i8_type().into());
                        let loaded = self.builder.build_load(ir_type, ptr, "load_ref")?;
                        self.new_loaded_operand(loaded, place.ty, generics)
                    }
                    _ => self.new_loaded_operand(ptr.into(), place.ty, generics),
                }
            }
        }
    }

    fn lower_field_access(
        &self,
        base: PlaceId,
        field_info: &FieldInfo,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let base_operand = self.lower_place_to_operand(base, generics)?;
        let base_ptr = base_operand.get_or_convert_pointer(&self.builder)?;

        if let Some(len) = self.is_stack_array_len_field(field_info, base) {
            let value = self.default_int_type.const_int(len, false).into();
            return Ok(IrOperand {
                value,
                info: OperandInfo::new_loaded(field_info.field_type, self.default_int_type.into()),
            });
        }

        self.expect_type_can_field(field_info.base_type)?;
        let base_type =
            self.lower_type(field_info.base_type, generics)?
                .ok_or(soul_error_internal!(
                    "none type found as base_type in field",
                    None
                ))?;

        let field_type = self
            .lower_type(field_info.field_type, generics)?
            .ok_or(soul_error_internal!("type should be Some", None))?;

        let field = self
            .builder
            .build_field_access(base_type, field_type, base_ptr, field_info)?;

        self.new_unloaded_operand(field.into(), field_info.field_type, generics)
    }

    pub(crate) fn expect_type_can_field(&self, base_type: TypeId) -> SoulResult<()> {
        let hir_type = self.get_type(base_type)?;
        match &hir_type.kind {
            ThirTypeKind::CustomTypes(_) => Ok(()),
            _ => Err(soul_error_internal!(
                format!(
                    "trying to access field but base type '{}' is not struct like",
                    hir_type.display(&self.types.types_map)
                ),
                None
            )),
        }
    }

    pub(crate) fn is_stack_array_len_field(
        &self,
        field_info: &FieldInfo,
        base: mir::PlaceId,
    ) -> Option<u64> {
        let ty = self.mir.tree.places[base].ty;
        let hir_type = self.get_type(ty).ok()?;
        match &hir_type.kind {
            ThirTypeKind::Array {
                kind: ArrayKind::StackArray(num),
                ..
            } => {
                if field_info.field_index == 1 {
                    Some(*num)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn lower_struct_contructor(
        &self,
        ty: TypeId,
        struct_id: StructId,
        body: &AggregateBody,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let struct_ir = self.get_or_create_struct(struct_id, generics)?;
        match body {
            AggregateBody::Runtime(operands) => {
                let mut ir_operands = Vec::with_capacity(operands.len());
                for op in operands {
                    ir_operands.push(self.lower_operand(op, generics)?.value);
                }

                self.lower_aggregate(struct_ir, ty, &ir_operands, generics)
            }
            AggregateBody::Comptime(literals) => {
                self.lower_const_aggregate(struct_ir, ty, literals, generics)
            }
        }
    }

    pub(crate) fn lower_aggregate(
        &self,
        struct_ir: StructType<'a>,
        ty: TypeId,
        fields: &[BasicValueEnum<'a>],
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ptr = self.builder.build_alloca(struct_ir, "tmp_struct")?;

        for (i, field) in fields.into_iter().enumerate() {
            self.builder.store_field(struct_ir, ptr, *field, i)?;
        }

        self.new_unloaded_operand(ptr.into(), ty, generics)
    }

    pub(crate) fn lower_const_aggregate(
        &self,
        struct_ir: StructType<'a>,
        ty: TypeId,
        literals: &Vec<(ComplexLiteral, TypeId)>,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let mut fields = Vec::with_capacity(literals.len());
        for (literal, ty) in literals {
            fields.push(self.lower_literal(literal, *ty, generics)?.value);
        }

        let aggregate = struct_ir.const_named_struct(fields.as_slice());
        self.new_loaded_operand(aggregate.into(), ty, generics)
    }

    pub(crate) fn lower_place_to_operand(
        &self,
        place: PlaceId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ty = self.mir.tree.places[place].ty;
        match &self.mir.tree.places[place].kind {
            mir::PlaceKind::Local(local_id) => {
                let local = self.get_local(*local_id);
                let ptr = match local {
                    Local::Runtime(ptr) => ptr,
                    Local::Comptime(op) => return Ok(op.clone()),
                };

                let hir_type = self.get_type(ty)?;
                match &hir_type.kind {
                    ThirTypeKind::Ref { .. } | ThirTypeKind::Pointer(_) => {
                        let ir_type = self.lower_type(ty, generics)?
                            .unwrap_or(self.context.i8_type().into());
                        let loaded = self.builder.build_load(ir_type, ptr, "load_ref")?;
                        self.new_loaded_operand(loaded, ty, generics)
                    }
                    _ => self.new_loaded_operand(ptr.into(), ty, generics),
                }
            }
            mir::PlaceKind::Temp(temp_id) => {
                let temp_op = self.get_temp(*temp_id)?;
                Ok(temp_op.clone())
            }
            mir::PlaceKind::Deref(operand) => {
                let ir_type = self
                    .lower_type(ty, generics)?
                    .unwrap_or(self.context.i8_type().into());

                let ptr_op = self.lower_operand(operand, generics)?;
                let ptr = ptr_op.value.into_pointer_value();
                let value = self.builder.build_load(ir_type, ptr, "load")?.into();
                self.new_loaded_operand(value, ty, generics)
            }
            mir::PlaceKind::Field {
                struct_type: _,
                base,
                field_id,
            } => {
                let field_info = &self.types.types_table.fields[*field_id];
                self.lower_field_access(*base, field_info, generics)
            }
        }
    }

    fn lower_stack_alloc(
        &self,
        ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ir_type = self
            .lower_type(ty, generics)?
            .ok_or(soul_error_internal!("stackalloc type should be Some", None))?;

        let ptr = self.builder.build_alloca(ir_type, "rvalue")?.into();

        self.new_loaded_operand(ptr, ty, generics)
    }

    fn lower_ptr_offset(
        &self,
        pointer: &mir::Operand,
        offset: &mir::Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let pointer_op = self.lower_operand(pointer, generics)?;
        let pointer_val = pointer_op.value.into_pointer_value();

        let offset_op = self.lower_operand(offset, generics)?;
        let offset_val = offset_op.value.into_int_value();

        let pointee_ty = match self.types.types_map.id_to_type(pointer.ty) {
            Some(thir_type) => match thir_type.kind {
                ThirTypeKind::Pointer(inner) => inner,
                other => {
                    return Err(SoulError::new(
                        format!("PtrOffset requires a pointer type got {}", other.display_variant()),
                        SoulErrorKind::LlvmError,
                        None,
                    ));
                }
            },
            None => {
                return Err(SoulError::new(
                    "PtrOffset pointer type not found",
                    SoulErrorKind::LlvmError,
                    None,
                ));
            }
        };

        let size_info = self.sizeof_bit(pointee_ty, generics)?;
        let size_bits = size_info.size;

        let ptr_int = self.builder.build_ptr_to_int(pointer_val, self.default_int_type)?;

        let size_bits_val = self.default_int_type.const_int(size_bits as u64, false);
        let eight = self.default_int_type.const_int(8, false);
        let size_bytes = self.builder.build_int_unsigned_div(size_bits_val, eight)?;

        let offset_ext = if offset_val.get_type() == self.default_int_type {
            offset_val
        } else if offset_val.get_type().get_bit_width() < self.default_int_type.get_bit_width() {
            self.builder.build_int_s_extend(offset_val, self.default_int_type)?
        } else {
            self.builder.build_int_truncate(offset_val, self.default_int_type)?
        };

        let byte_offset = self.builder.build_int_mul(offset_ext, size_bytes)?;
        let result_int = self.builder.build_int_add(ptr_int, byte_offset)?;
        let result_ptr = self.builder.build_int_to_ptr(result_int, self.context.ptr_type(AddressSpace::default()))?;

        Ok(IrOperand {
            value: result_ptr.into(),
            info: crate::OperandInfo::new_loaded(pointer.ty, result_ptr.get_type().into()),
        })
    }

    fn lower_stack_array_index(
        &self,
        array: &mir::Operand,
        index: &mir::Operand,
        result_ty: TypeId,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let array_op = self.lower_operand(array, generics)?;
        let array_ptr = array_op.get_or_convert_pointer(&self.builder)?;

        let index_op = self.lower_operand(index, generics)?;
        let index_val = index_op.value.into_int_value();

        let element_ty = match self.types.types_map.id_to_type(array.ty) {
            Some(thir_type) => match &thir_type.kind {
                ThirTypeKind::Array { element, .. } => *element,
                _ => {
                    return Err(SoulError::new(
                        "StackArrayIndex requires an array type".to_string(),
                        SoulErrorKind::LlvmError,
                        None,
                    ));
                }
            },
            None => {
                return Err(SoulError::new(
                    "StackArrayIndex array type not found".to_string(),
                    SoulErrorKind::LlvmError,
                    None,
                ));
            }
        };

        let size_info = self.sizeof_bit(element_ty, generics)?;
        let size_bits = size_info.size;

        let ptr_int = self.builder.build_ptr_to_int(array_ptr, self.default_int_type)?;

        let size_bits_val = self.default_int_type.const_int(size_bits as u64, false);
        let eight = self.default_int_type.const_int(8, false);
        let size_bytes = self.builder.build_int_unsigned_div(size_bits_val, eight)?;

        let index_ext = if index_val.get_type() == self.default_int_type {
            index_val
        } else if index_val.get_type().get_bit_width() < self.default_int_type.get_bit_width() {
            self.builder.build_int_s_extend(index_val, self.default_int_type)?
        } else {
            self.builder.build_int_truncate(index_val, self.default_int_type)?
        };

        let byte_offset = self.builder.build_int_mul(index_ext, size_bytes)?;
        let result_int = self.builder.build_int_add(ptr_int, byte_offset)?;
        let result_ptr = self.builder.build_int_to_ptr(result_int, self.context.ptr_type(AddressSpace::default()))?;

        Ok(IrOperand {
            value: result_ptr.into(),
            info: crate::OperandInfo::new_loaded(result_ty, result_ptr.get_type().into()),
        })
    }
}
