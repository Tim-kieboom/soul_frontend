//! Operand/rvalue codegen: turns a MIR `Operand`/`Rvalue` into an LLVM
//! `BasicValueEnum` — split out of `function` since it's a distinct concern
//! from control flow (blocks/statements/terminators) and place resolution.

use ast_model::{
    SoulType,
    operators::{BinaryOperatorKind, UnaryOperatorKind},
};
use inkwell::{
    IntPredicate,
    intrinsics::Intrinsic,
    module::Linkage,
    types::BasicTypeEnum,
    values::{BasicValueEnum, IntValue, PointerValue},
};
use mir_model::{AggregateKind, ConstValue, Operand, Place, Rvalue};

use crate::{
    err,
    fault::{CodegenErrorKind, CodegenResult},
    function::FunctionCodegen,
    llvm_err,
    types::{const_int, expect_int, is_signed},
};

impl<'ctx, 'a> FunctionCodegen<'ctx, 'a> {
    /// Materializes a Soul string literal as a null-terminated LLVM global
    /// byte-array constant and returns a pointer to it — the only way a
    /// `cstr` value currently comes into existence (there's no other
    /// `cstr`-producing expression in this slice, so this is the sole
    /// producer of one). Not deduplicated across equal literals: correctness
    /// over compactness for this first slice. Also reused by
    /// `terminator::codegen_assert` to materialize a panic's location string.
    pub(crate) fn codegen_string_constant(&self, s: &str) -> PointerValue<'ctx> {
        let id = self.string_counter.get();
        self.string_counter.set(id + 1);

        let const_str = self.ctx.context.const_string(s.as_bytes(), true);
        let global = self
            .ctx
            .module
            .add_global(const_str.get_type(), None, &format!("str.{id}"));
        global.set_initializer(&const_str);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.as_pointer_value()
    }

    pub(crate) fn codegen_rvalue(
        &mut self,
        rvalue: &Rvalue,
        result_ty: BasicTypeEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match rvalue {
            Rvalue::Use(operand) => self.codegen_operand(operand, result_ty),
            Rvalue::BinaryOp(op, left, right) => self.codegen_binary(result_ty, op, left, right),
            Rvalue::CheckedBinaryOp(op, left, right) => {
                self.codegen_checked_binary(result_ty, *op, left, right)
            }
            Rvalue::UnaryOp(op, operand) => self.codegen_unary(op, operand),
            Rvalue::Aggregate(AggregateKind::Struct | AggregateKind::Array, operands) => {
                self.codegen_aggregate(operands, result_ty)
            }
            // Just the address `resolve_place` already computes, no load —
            // this is the bare-pointer case only (see `resolve_field_place`'s
            // docs in `mir_parser`): a reference to an array-typed place is
            // never lowered as a plain `Ref`, it's lowered as an
            // `Aggregate(Array, [Ref-to-first-element, len])` instead, so by
            // the time a `Ref` reaches codegen its place is never an array.
            Rvalue::Ref { place, .. } => {
                let (ptr, _) = self.resolve_place(place)?;
                Ok(ptr.into())
            }
            Rvalue::Len(place) => self.codegen_len(place),
            Rvalue::Cast(operand, _) => self.codegen_cast(operand, result_ty),
            Rvalue::Aggregate(..) => Err(err(CodegenErrorKind::UnsupportedRvalue)),
        }
    }

    fn codegen_unary(
        &mut self,
        op: &UnaryOperatorKind,
        operand: &Operand,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let bool_ty = self.ctx.context.bool_type().into();
        let value = expect_int(self.codegen_operand(operand, bool_ty)?)?;
        match op {
            UnaryOperatorKind::Not => Ok(self
                .builder
                .build_not(value, "not")
                .map_err(llvm_err)?
                .into()),

            other => Err(err(CodegenErrorKind::UnsupportedUnaryOperator {
                op: format!("{other:?}").into_boxed_str(),
            })),
        }
    }

    fn codegen_binary(
        &mut self,
        result_ty: BasicTypeEnum<'ctx>,
        op: &BinaryOperatorKind,
        left: &Operand,
        right: &Operand,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let operand_ty = self
            .operand_type(left)
            .or_else(|| self.operand_type(right))
            .unwrap_or(result_ty);

        let l = expect_int(self.codegen_operand(left, operand_ty)?)?;
        let r = expect_int(self.codegen_operand(right, operand_ty)?)?;
        let signed = self.operand_is_signed(left) || self.operand_is_signed(right);
        Ok(self.codegen_binary_op(*op, l, r, signed)?.into())
    }

    /// Builds an aggregate value element-by-element: an `undef` of the
    /// destination type, then one `insertvalue` per operand. Used for real
    /// structs and the compiler-synthesized `{ptr, len}` slice fat pointer
    /// (both a `StructType` destination — `operands` in declared field
    /// order for a struct, always `[data pointer, length]` for a slice —
    /// this function doesn't care which, since it only ever reads the
    /// *destination* LLVM type, never the `AggregateKind` tag), and for a
    /// fixed-size array literal (an `ArrayType` destination, one uniform
    /// element type instead of a per-index field type).
    fn codegen_aggregate(
        &mut self,
        operands: &[Operand],
        result_ty: BasicTypeEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match result_ty {
            BasicTypeEnum::StructType(struct_ty) => {
                let mut value = struct_ty.get_undef();
                for (index, operand) in operands.iter().enumerate() {
                    let field_ty = struct_ty
                        .get_field_type_at_index(index as u32)
                        .ok_or(CodegenErrorKind::PlaceProjectionUnsupported)
                        .map_err(err)?;

                    let field_value = self.codegen_operand(operand, field_ty)?;
                    value = self
                        .builder
                        .build_insert_value(value, field_value, index as u32, "field")
                        .map_err(llvm_err)?
                        .into_struct_value();
                }
                Ok(value.into())
            }
            BasicTypeEnum::ArrayType(array_ty) => {
                let element_ty = array_ty.get_element_type();
                let mut value = array_ty.get_undef();
                for (index, operand) in operands.iter().enumerate() {
                    let element_value = self.codegen_operand(operand, element_ty)?;
                    value = self
                        .builder
                        .build_insert_value(value, element_value, index as u32, "elem")
                        .map_err(llvm_err)?
                        .into_array_value();
                }
                Ok(value.into())
            }
            _ => Err(err(CodegenErrorKind::PlaceProjectionUnsupported)),
        }
    }

    fn codegen_binary_op(
        &mut self,
        op: BinaryOperatorKind,
        l: IntValue<'ctx>,
        r: IntValue<'ctx>,
        signed: bool,
    ) -> CodegenResult<IntValue<'ctx>> {
        use BinaryOperatorKind::*;
        let b = &self.builder;
        let v = match op {
            // Reached only when `mir_parser`'s `CHECK_ALGORITHMIC_OVERFLOW`
            // option is off — otherwise `Add`/`Sub`/`Mul` always lower to
            // `Rvalue::CheckedBinaryOp` (`codegen_checked_binary`) instead.
            // Plain wrapping ops, matching the option being disabled.
            Add => b.build_int_add(l, r, "add"),
            Sub => b.build_int_sub(l, r, "sub"),
            Mul => b.build_int_mul(l, r, "mul"),

            Div if signed => b.build_int_signed_div(l, r, "sdiv"),
            Div => b.build_int_unsigned_div(l, r, "udiv"),
            Mod if signed => b.build_int_signed_rem(l, r, "srem"),
            Mod => b.build_int_unsigned_rem(l, r, "urem"),
            Eq => b.build_int_compare(IntPredicate::EQ, l, r, "eq"),
            NotEq => b.build_int_compare(IntPredicate::NE, l, r, "ne"),
            Lt if signed => b.build_int_compare(IntPredicate::SLT, l, r, "lt"),
            Lt => b.build_int_compare(IntPredicate::ULT, l, r, "lt"),
            Gt if signed => b.build_int_compare(IntPredicate::SGT, l, r, "gt"),
            Gt => b.build_int_compare(IntPredicate::UGT, l, r, "gt"),
            Le if signed => b.build_int_compare(IntPredicate::SLE, l, r, "le"),
            Le => b.build_int_compare(IntPredicate::ULE, l, r, "le"),
            Ge if signed => b.build_int_compare(IntPredicate::SGE, l, r, "ge"),
            Ge => b.build_int_compare(IntPredicate::UGE, l, r, "ge"),
            LogAnd => b.build_and(l, r, "and"),
            LogOr => b.build_or(l, r, "or"),
            other => {
                return Err(err(CodegenErrorKind::UnsupportedBinaryOperator {
                    op: format!("{other:?}").into_boxed_str(),
                }));
            }
        };
        v.map_err(llvm_err)
    }

    /// `Add`/`Sub`/`Mul` via LLVM's `{s,u}{add,sub,mul}.with.overflow`
    /// intrinsics (chosen on `signed`, matching every other signed-vs-
    /// unsigned branch in this file), returning the raw `{result, i1
    /// overflowed}` struct the intrinsic produces directly as the `(T,
    /// bool)` tuple value `mir_parser`'s `lower_checked_binary_op` assigns
    /// this into — LLVM uniques anonymous struct types structurally, so the
    /// intrinsic's own `{iN, i1}` return type and the destination tuple's
    /// synthesized `StructType` are the same type, no repacking needed.
    /// Deciding whether the overflow actually traps isn't this function's
    /// concern any more: `mir_parser` already emits a `Terminator::Assert`
    /// on the tuple's `bool` field before the result (field `0`) is ever
    /// read, so this only has to compute the pair.
    fn codegen_checked_binary(
        &mut self,
        result_ty: BasicTypeEnum<'ctx>,
        op: BinaryOperatorKind,
        left: &Operand,
        right: &Operand,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let BasicTypeEnum::StructType(tuple_ty) = result_ty else {
            return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
        };
        let operand_ty = tuple_ty
            .get_field_type_at_index(0)
            .ok_or_else(|| err(CodegenErrorKind::PlaceProjectionUnsupported))?;

        let l = expect_int(self.codegen_operand(left, operand_ty)?)?;
        let r = expect_int(self.codegen_operand(right, operand_ty)?)?;
        let signed = self.operand_is_signed(left) || self.operand_is_signed(right);

        use BinaryOperatorKind::*;
        let name = match (op, signed) {
            (Add, true) => "llvm.sadd.with.overflow",
            (Add, false) => "llvm.uadd.with.overflow",
            (Sub, true) => "llvm.ssub.with.overflow",
            (Sub, false) => "llvm.usub.with.overflow",
            (Mul, true) => "llvm.smul.with.overflow",
            (Mul, false) => "llvm.umul.with.overflow",
            _ => unreachable!("codegen_checked_binary is only reached for Add/Sub/Mul"),
        };

        let intrinsic = Intrinsic::find(name).ok_or_else(|| {
            err(CodegenErrorKind::OverflowIntrinsicUnavailable { name: name.into() })
        })?;
        let fn_value = intrinsic
            .get_declaration(self.ctx.module, &[operand_ty])
            .ok_or_else(|| {
                err(CodegenErrorKind::OverflowIntrinsicUnavailable { name: name.into() })
            })?;

        let call = self
            .builder
            .build_call(fn_value, &[l.into(), r.into()], "checked_arith")
            .map_err(llvm_err)?;

        call.try_as_basic_value()
            .left()
            .ok_or(CodegenErrorKind::CallResultIsNone)
            .map_err(err)
    }

    /// The runtime length of a slice place — loads field `1` of its
    /// `{ptr, len}` fat pointer (`resolve_place` on a bare slice-typed place
    /// returns the fat pointer's own address and `StructType`). Used by
    /// `mir_parser`'s bounds-check lowering (`emit_bounds_check`) ahead of a
    /// comparison and a `Terminator::Assert`.
    fn codegen_len(&mut self, place: &Place) -> CodegenResult<BasicValueEnum<'ctx>> {
        let (ptr, ty) = self.resolve_place(place)?;
        let BasicTypeEnum::StructType(slice_ty) = ty else {
            return Err(err(CodegenErrorKind::PlaceProjectionUnsupported));
        };
        let len_field_ty = slice_ty
            .get_field_type_at_index(1)
            .ok_or_else(|| err(CodegenErrorKind::PlaceProjectionUnsupported))?;
        let len_addr = self
            .builder
            .build_struct_gep(slice_ty, ptr, 1, "slice_len_addr")
            .map_err(llvm_err)?;
        self.builder
            .build_load(len_field_ty, len_addr, "slice_len")
            .map_err(llvm_err)
    }

    /// Widens/narrows an int operand to `result_ty`'s width — sign-extending
    /// when the source is a signed int, zero-extending otherwise, truncating
    /// when narrowing (a no-op when the widths already match). Used by
    /// `mir_parser`'s bounds-check lowering to normalize an index to the
    /// slice `len` field's `uint` width before comparing them — a
    /// bare-`Variable` index keeps its own declared type rather than being
    /// retyped (see `operand_local`), so it isn't necessarily already that
    /// width. A negative signed index sign-extends to a huge unsigned value,
    /// so it's still caught by the same unsigned compare as an over-long one.
    fn codegen_cast(
        &mut self,
        operand: &Operand,
        result_ty: BasicTypeEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let BasicTypeEnum::IntType(dst_ty) = result_ty else {
            return Err(err(CodegenErrorKind::UnsupportedRvalue));
        };
        let src_ty = self.operand_type(operand).unwrap_or(result_ty);
        let signed = self.operand_is_signed(operand);
        let value = expect_int(self.codegen_operand(operand, src_ty)?)?;

        let src_bits = value.get_type().get_bit_width();
        let dst_bits = dst_ty.get_bit_width();
        let casted = match src_bits.cmp(&dst_bits) {
            std::cmp::Ordering::Less if signed => self
                .builder
                .build_int_s_extend(value, dst_ty, "cast_sext")
                .map_err(llvm_err)?,
            std::cmp::Ordering::Less => self
                .builder
                .build_int_z_extend(value, dst_ty, "cast_zext")
                .map_err(llvm_err)?,
            std::cmp::Ordering::Greater => self
                .builder
                .build_int_truncate(value, dst_ty, "cast_trunc")
                .map_err(llvm_err)?,
            std::cmp::Ordering::Equal => value,
        };
        Ok(casted.into())
    }

    pub(crate) fn codegen_operand(
        &mut self,
        operand: &Operand,
        ty: BasicTypeEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let (ptr, _) = self.resolve_place(place)?;
                self.builder.build_load(ty, ptr, "load").map_err(llvm_err)
            }
            Operand::Constant(value) => self.codegen_constant(ty, value),
        }
    }

    fn codegen_constant(
        &mut self,
        ty: BasicTypeEnum<'ctx>,
        value: &ConstValue,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match ty {
            BasicTypeEnum::IntType(int_ty) => Ok(const_int(int_ty, value)?.into()),
            BasicTypeEnum::PointerType(_) => match value {
                ConstValue::Str(s) | ConstValue::Cstr(s) => {
                    Ok(self.codegen_string_constant(s).into())
                }
                other => Err(err(CodegenErrorKind::UnsupportedConstant {
                    value: format!("{other:?}").into_boxed_str(),
                })),
            },
            other => Err(err(CodegenErrorKind::UnsupportedPrimitiveType {
                ty: format!("{other:?}").into_boxed_str(),
            })),
        }
    }

    /// The LLVM type a place-backed operand is stored as, if it is one — a
    /// bare constant operand carries no type of its own (see the module docs).
    fn operand_type(&self, operand: &Operand) -> Option<BasicTypeEnum<'ctx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) if place.projection.is_empty() => {
                self.local_type(place.local).ok()
            }
            _ => None,
        }
    }

    fn operand_is_signed(&self, operand: &Operand) -> bool {
        match operand {
            Operand::Copy(place) | Operand::Move(place) if place.projection.is_empty() => {
                matches!(&self.function.locals[place.local].ty, SoulType::Primitive(p) if is_signed(*p))
            }
            Operand::Constant(ConstValue::Int(_)) => true,
            _ => false,
        }
    }
}
