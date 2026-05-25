use crate::{GenericSubstitute, IrOperand, LlvmBackend, Local, OperandInfo};
use ast::{ArrayKind, Literal};
use hir::{ComplexLiteral, StructId, TypeId};
use inkwell::{
    AddressSpace,
    types::{BasicType, StructType},
    values::BasicValueEnum,
};

use mir_parser::mir::{self, AggregateBody, Place, PlaceId, Rvalue, RvalueKind};
use soul_utils::{
    Span,
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_error_internal,
};
use typed_hir::{FieldInfo, ThirTypeKind, display_thir::DisplayThirType};

use crate::value::sizeof::Alignment;

pub(crate) mod binary_unary;
pub(crate) mod cast;
pub(crate) mod operand;
pub(crate) mod sizeof;

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
            RvalueKind::HeapAlloc {
                ty: inner_type,
                count,
            } => self.lower_heap_alloc(ty, *inner_type, *count, generics)?,
            RvalueKind::Drop { value, span } => {
                self.lower_drop(value, *span, generics)?;
                return Ok(None);
            }
            RvalueKind::Alloc { size } => self.lower_alloc(ty, size, generics)?,
            RvalueKind::Realloc { ptr, size } => self.lower_realloc(ty, ptr, size, generics)?,
            RvalueKind::UnionTag { value } => self.lower_union_tag(value, generics)?,
            RvalueKind::UnionExtract { value } => self.lower_union_extract(value, generics)?,
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

    fn lower_heap_alloc(
        &self,
        ty: TypeId,
        inner_type: TypeId,
        count: u64,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let size_bytes_u64 = self.sizeof(inner_type, generics)? as u64;

        let size_bytes = self.default_int_type.const_int(size_bytes_u64, false);

        let total_size = if count > 1 {
            let count_val = self.default_int_type.const_int(count, false);
            self.builder.build_int_mul(size_bytes, count_val)?
        } else {
            size_bytes
        };

        let malloc_fn = self
            .internal_functions
            .malloc_function
            .ok_or_else(|| soul_error_internal!("malloc function not declared", None))?;
        let call = self.builder.build_call(malloc_fn, &[total_size.into()])?;
        let ptr = call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| soul_error_internal!("malloc call returned no value", None))?;

        self.new_loaded_operand(ptr, ty, generics)
    }

    fn lower_alloc(
        &self,
        ty: TypeId,
        size: &mir::Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let malloc_fn = self
            .internal_functions
            .malloc_function
            .ok_or_else(|| soul_error_internal!("malloc function not declared", None))?;

        let size_val = self.lower_operand(size, generics)?.value;
        let call = self.builder.build_call(malloc_fn, &[size_val.into()])?;
        let ptr = call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| soul_error_internal!("malloc call returned no value", None))?;

        self.new_loaded_operand(ptr, ty, generics)
    }

    fn lower_union_tag(
        &self,
        value: &mir::Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let union_op = self.lower_operand(value, generics)?;
        let union_ty = value.ty;
        let index_ty_id = self.types.types_table.index_type;
        let tag_ir = self
            .lower_type(index_ty_id, generics)?
            .ok_or(soul_error_internal!("index type should lower", None))?;
        if union_op.info.is_unloaded {
            let ptr = union_op.value.into_pointer_value();
            let compact_ty = self
                .lower_type(union_ty, generics)?
                .ok_or(soul_error_internal!("union type should lower", None))?
                .into_struct_type();
            let tag_ptr = self
                .builder
                .build_struct_gep_index(compact_ty, ptr, 0, "tag")?;
            let tag_val = self.builder.build_load(tag_ir, tag_ptr, "union_tag")?;
            self.new_loaded_operand(tag_val, index_ty_id, generics)
        } else {
            let tag_val = self
                .builder
                .inkwell()
                .build_extract_value(union_op.value.into_struct_value(), 0, "union_tag")
                .map_err(|e| SoulError::new(e.to_string(), SoulErrorKind::LlvmError, None))?;
            self.new_loaded_operand(tag_val, index_ty_id, generics)
        }
    }

    fn lower_union_extract(
        &self,
        value: &mir::Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let union_op = self.lower_operand(value, generics)?;
        let union_ty = value.ty;
        if union_op.info.is_unloaded {
            let ptr = union_op.value.into_pointer_value();
            let compact_ty = self
                .lower_type(union_ty, generics)?
                .ok_or(soul_error_internal!("union type should lower", None))?
                .into_struct_type();
            let data_ptr = self
                .builder
                .build_struct_gep_index(compact_ty, ptr, 1, "data")?;

            let data_ir = compact_ty
                .get_field_type_at_index(1)
                .ok_or(soul_error_internal!(
                    "compact union has no data field",
                    None
                ))?;
            let data_val = self.builder.build_load(data_ir, data_ptr, "union_data")?;

            self.new_loaded_operand(data_val, union_ty, generics)
        } else {
            let data_val = self
                .builder
                .inkwell()
                .build_extract_value(union_op.value.into_struct_value(), 1, "union_data")
                .map_err(|e| SoulError::new(e.to_string(), SoulErrorKind::LlvmError, None))?;
            self.new_loaded_operand(data_val, union_ty, generics)
        }
    }

    fn lower_realloc(
        &self,
        ty: TypeId,
        ptr: &mir::Operand,
        size: &mir::Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let realloc_fn = self
            .internal_functions
            .realloc_function
            .ok_or_else(|| soul_error_internal!("realloc function not declared", None))?;

        let ptr_val = self.lower_operand(ptr, generics)?.value;
        let size_val = self.lower_operand(size, generics)?.value;
        let call = self
            .builder
            .build_call(realloc_fn, &[ptr_val.into(), size_val.into()])?;
        let result = call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| soul_error_internal!("realloc call returned no value", None))?;

        self.new_loaded_operand(result, ty, generics)
    }

    fn lower_drop(
        &self,
        value: &mir::Operand,
        span: Span,
        generics: &GenericSubstitute,
    ) -> SoulResult<()> {
        let value_op = self.lower_operand(value, generics)?;
        let hir_type = self.get_type(value.ty)?;

        match &hir_type.kind {
            ThirTypeKind::Pointer(_) => {
                let ptr = if value_op.value.is_pointer_value() {
                    value_op.value.into_pointer_value()
                } else {
                    return Err(soul_error_internal!(
                        "Drop: expected pointer value",
                        Some(span)
                    ));
                };

                let Some(free_fn) = self.internal_functions.free_function else {
                    return Err(soul_error_internal!(
                        "free function not declared",
                        Some(span)
                    ));
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

                    self.builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "drop_heap_array_data",
                        )?
                        .into_pointer_value()
                } else {
                    if !value_op.value.is_struct_value() {
                        return Err(soul_error_internal!(
                            "Drop: expected struct value for loaded heap array",
                            Some(span)
                        ));
                    }

                    let struct_val = value_op.value.into_struct_value();
                    let struct_ty = heap_struct_ir_type.into_struct_type();
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let ptr_alloca = self
                        .builder
                        .build_alloca(struct_ty, "drop_heap_array_temp")?;
                    let _ = self.builder.inkwell().build_store(ptr_alloca, struct_val);
                    let data_ptr_ptr = self.builder.build_struct_gep_index(
                        struct_ty,
                        ptr_alloca,
                        0,
                        "drop_heap_array_data_ptr",
                    )?;
                    self.builder
                        .build_load(ptr_type, data_ptr_ptr, "drop_heap_array_data")?
                        .into_pointer_value()
                };
                let free_fn = self.internal_functions.free_function.ok_or_else(|| {
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

        let field = self
            .builder
            .build_field_access(base_type, base_ptr, field_info)?;

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
        // Check if this struct is a union's internal struct → use compact layout
        if let Some(compact_ty) = self.try_lower_compact_union(ty, struct_id, body, generics)? {
            return Ok(compact_ty);
        }

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

    /// If `struct_id` is a union's internal struct, lower a compact tagged union.
    /// Returns `None` if this is not a union internal struct.
    fn try_lower_compact_union(
        &self,
        ty: TypeId,
        struct_id: StructId,
        body: &AggregateBody,
        generics: &GenericSubstitute,
    ) -> SoulResult<Option<IrOperand<'a>>> {
        let union_info = match self
            .types
            .types_map
            .unions
            .entries()
            .find(|(_, info)| info.internal_struct == struct_id)
            .map(|(_, info)| info)
        {
            Some(val) => val,
            None => return Ok(None),
        };

        let variant_index = match body {
            AggregateBody::Runtime(operands) => match &operands[0].kind {
                mir::OperandKind::Comptime(ComplexLiteral::Basic(lit)) => match lit {
                    Literal::Int(idx) => *idx as usize,
                    _ => {
                        return Err(soul_error_internal!(
                            "union constructor tag should be Int literal",
                            None
                        ));
                    }
                },
                _ => {
                    return Err(soul_error_internal!(
                        "union constructor tag should be comptime",
                        None
                    ));
                }
            },
            AggregateBody::Comptime(literals) => match &literals[0].0 {
                ComplexLiteral::Basic(lit) => match lit {
                    Literal::Int(idx) => *idx as usize,
                    _ => {
                        return Err(soul_error_internal!(
                            "union constructor tag should be Int literal in comptime aggregate",
                            None
                        ));
                    }
                },
                _ => {
                    return Err(soul_error_internal!(
                        "union constructor tag should be comptime in comptime aggregate",
                        None
                    ));
                }
            },
        };

        let variant_ir = match body {
            AggregateBody::Runtime(operands) => self.lower_operand(&operands[1], generics)?.value,
            AggregateBody::Comptime(literals) => {
                self.lower_literal(&literals[1].0, literals[1].1, generics)?
                    .value
            }
        };

        let index_ty = self.types.types_table.index_type;
        let tag_ir_ty = match self.lower_type(index_ty, generics)? {
            Some(val) => val,
            None => self.default_int_type.into(),
        };
        let mut max_bits = 0u32;
        let mut max_align = Alignment::Null;
        for &variant_type_id in &union_info.variant_types {
            let field = self.sizeof_bit(variant_type_id, generics)?;
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
        let compact_struct_ty = self
            .context
            .struct_type(&[tag_ir_ty, array_ty.into()], false);

        let ptr = self.builder.build_alloca(compact_struct_ty, "union")?;

        let tag_value = self
            .context
            .i64_type()
            .const_int(variant_index as u64, false);
        self.builder
            .store_field(compact_struct_ty, ptr, tag_value, 0)?;

        let data_ptr =
            self.builder
                .build_struct_gep_index(compact_struct_ty, ptr, 1, "union_data")?;
        let variant_ptr = self.builder.build_pointer_cast(
            data_ptr,
            self.context.ptr_type(AddressSpace::default()),
            "union_variant",
        )?;
        self.builder.store_parameter(variant_ptr, variant_ir)?;

        self.new_unloaded_operand(ptr.into(), ty, generics)
            .map(Some)
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
                        let ir_type = self
                            .lower_type(ty, generics)?
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
                        format!(
                            "PtrOffset requires a pointer type got {}",
                            other.display_variant()
                        ),
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

        let size_bytes_u64 = self.sizeof(pointee_ty, generics)? as u64;

        let ptr_int = self
            .builder
            .build_ptr_to_int(pointer_val, self.default_int_type)?;

        let size_bytes = self.default_int_type.const_int(size_bytes_u64, false);

        let offset_ext = if offset_val.get_type() == self.default_int_type {
            offset_val
        } else if offset_val.get_type().get_bit_width() < self.default_int_type.get_bit_width() {
            self.builder
                .build_int_s_extend(offset_val, self.default_int_type)?
        } else {
            self.builder
                .build_int_truncate(offset_val, self.default_int_type)?
        };

        let byte_offset = self.builder.build_int_mul(offset_ext, size_bytes)?;
        let result_int = self.builder.build_int_add(ptr_int, byte_offset)?;
        let result_ptr = self
            .builder
            .build_int_to_ptr(result_int, self.context.ptr_type(AddressSpace::default()))?;

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

        let size_bytes_u64 = self.sizeof(element_ty, generics)? as u64;

        let ptr_int = self
            .builder
            .build_ptr_to_int(array_ptr, self.default_int_type)?;

        let size_bytes = self.default_int_type.const_int(size_bytes_u64, false);
        let index_ext = if index_val.get_type() == self.default_int_type {
            index_val
        } else if index_val.get_type().get_bit_width() < self.default_int_type.get_bit_width() {
            self.builder
                .build_int_s_extend(index_val, self.default_int_type)?
        } else {
            self.builder
                .build_int_truncate(index_val, self.default_int_type)?
        };

        let byte_offset = self.builder.build_int_mul(index_ext, size_bytes)?;
        let result_int = self.builder.build_int_add(ptr_int, byte_offset)?;
        let result_ptr = self
            .builder
            .build_int_to_ptr(result_int, self.context.ptr_type(AddressSpace::default()))?;

        Ok(IrOperand {
            value: result_ptr.into(),
            info: crate::OperandInfo::new_loaded(result_ty, result_ptr.get_type().into()),
        })
    }
}
