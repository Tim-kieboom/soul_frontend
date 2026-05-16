use ast::{ArrayKind, BinaryOperator, UnaryOperator};
use hir::TypeId;
use inkwell::{
    AddressSpace, FloatPredicate, IntPredicate, types::BasicTypeEnum, values::BasicValueEnum,
};
use mir_parser::mir;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_error_internal,
};
use typed_hir::{ThirType, ThirTypeKind};

use crate::{GenericSubstitute, IrOperand, LlvmBackend, OperandInfo};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(super) fn lower_binary(
        &self,
        left: &mir::Operand,
        operator: &BinaryOperator,
        right: &mir::Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let mut ir_left = self.lower_operand(left, generics)?;
        let mut ir_right = self.lower_operand(right, generics)?;

        if ir_left.info.is_unloaded {
            let ptr = ir_left.get_or_convert_pointer(&self.builder)?;
            ir_left.value = self
                .builder
                .build_load(ir_left.info.ir_type, ptr, "load_left")?;
            ir_left.info.is_unloaded = false;
        }

        if ir_right.info.is_unloaded {
            let ptr = ir_right.get_or_convert_pointer(&self.builder)?;
            ir_right.value = self
                .builder
                .build_load(ir_right.info.ir_type, ptr, "load_left")?;
            ir_right.info.is_unloaded = false;
        }

        match operator.node {
            ast::BinaryOperatorKind::Invalid => {
                return Err(SoulError::new(
                    "ast::BinaryOperatorKind::Invalid should not exist in llvm lowerer",
                    SoulErrorKind::LlvmError,
                    None,
                ));
            }
            ast::BinaryOperatorKind::Add => self.add(ir_left, ir_right),
            ast::BinaryOperatorKind::Sub => self.sub(ir_left, ir_right),
            ast::BinaryOperatorKind::Mul => self.mul(ir_left, ir_right),
            ast::BinaryOperatorKind::Div => self.div(ir_left, ir_right),
            ast::BinaryOperatorKind::BitAnd => self.bit_and(ir_left, ir_right),
            ast::BinaryOperatorKind::BitOr => self.bit_or(ir_left, ir_right),
            ast::BinaryOperatorKind::BitXor => self.bit_xor(ir_left, ir_right),
            ast::BinaryOperatorKind::LogAnd => self.bit_and(ir_left, ir_right),
            ast::BinaryOperatorKind::LogOr => self.bit_or(ir_left, ir_right),
            ast::BinaryOperatorKind::NotEq => {
                self.compare(IrCompare::NotEq, ir_left, ir_right, generics)
            }
            ast::BinaryOperatorKind::Eq => self.compare(IrCompare::Eq, ir_left, ir_right, generics),
            ast::BinaryOperatorKind::Lt => self.compare(IrCompare::Lt, ir_left, ir_right, generics),
            ast::BinaryOperatorKind::Gt => self.compare(IrCompare::Gt, ir_left, ir_right, generics),
            ast::BinaryOperatorKind::Le => self.compare(IrCompare::Le, ir_left, ir_right, generics),
            ast::BinaryOperatorKind::Ge => self.compare(IrCompare::Ge, ir_left, ir_right, generics),

            ast::BinaryOperatorKind::Mod => self.modulo(ir_left, ir_right),
            ast::BinaryOperatorKind::Log => todo!("impl log llvm"),
            ast::BinaryOperatorKind::Pow => todo!("impl pow llvm"),
            ast::BinaryOperatorKind::Root => todo!("impl root llvm"),
            ast::BinaryOperatorKind::Range => todo!("impl range llvm"),
            ast::BinaryOperatorKind::TypeOf => todo!("impl typeof llvm"),
        }
    }

    pub(super) fn lower_unary(
        &self,
        value: &mir::Operand,
        operator: &UnaryOperator,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let mut ir_value = self.lower_operand(value, generics)?;

        if ir_value.info.is_unloaded {
            let ptr = ir_value.get_or_convert_pointer(&self.builder)?;
            ir_value.value = self
                .builder
                .build_load(ir_value.info.ir_type, ptr, "load_unary")?;
            ir_value.info.is_unloaded = false;
        }

        match &operator.node {
            ast::UnaryOperatorKind::Invalid => {
                return Err(SoulError::new(
                    "ast::UnaryOperatorKind::Invalid should not exist in llvm lowerer",
                    SoulErrorKind::LlvmError,
                    None,
                ));
            }
            ast::UnaryOperatorKind::Neg => self.neg(ir_value),
            ast::UnaryOperatorKind::Not => self.not(ir_value),
            ast::UnaryOperatorKind::Increment { .. } => todo!("impl Increment llvm"),
            ast::UnaryOperatorKind::Decrement { .. } => todo!("impl Decrement llvm"),
        }
    }

    fn add(&self, left: IrOperand<'a>, right: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                self.builder.build_int_add(l, r).map(BasicValueEnum::from)
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                self.builder.build_float_add(l, r).map(BasicValueEnum::from)
            }
            _ => Err(SoulError::new(
                format!(
                    "add requires int or float values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn sub(&self, left: IrOperand<'a>, right: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                self.builder.build_int_sub(l, r).map(BasicValueEnum::from)
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                self.builder.build_float_sub(l, r).map(BasicValueEnum::from)
            }
            _ => Err(SoulError::new(
                format!(
                    "sub requires int or float values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn mul(&self, left: IrOperand<'a>, right: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                self.builder.build_int_mul(l, r).map(BasicValueEnum::from)
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                self.builder.build_float_mul(l, r).map(BasicValueEnum::from)
            }
            _ => Err(SoulError::new(
                format!(
                    "mul requires int or float values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn div(&self, left: IrOperand<'a>, right: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                if self.is_signed_interger(&left.info) {
                    self.builder
                        .build_int_signed_div(l, r)
                        .map(BasicValueEnum::from)
                } else {
                    self.builder
                        .build_int_unsigned_div(l, r)
                        .map(BasicValueEnum::from)
                }
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                self.builder.build_float_div(l, r).map(BasicValueEnum::from)
            }
            _ => Err(SoulError::new(
                format!(
                    "div requires int or float values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn modulo(&self, left: IrOperand<'a>, right: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                if self.is_signed_interger(&left.info) {
                    self.builder
                        .build_int_signed_rem(l, r)
                        .map(BasicValueEnum::from)
                } else {
                    self.builder
                        .build_int_unsigned_rem(l, r)
                        .map(BasicValueEnum::from)
                }
            }
            _ => Err(SoulError::new(
                format!(
                    "mod requires int values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn bit_and(&self, left: IrOperand<'a>, right: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                self.builder.build_and(l, r).map(BasicValueEnum::from)
            }
            (BasicValueEnum::FloatValue(_), BasicValueEnum::FloatValue(_)) => Err(SoulError::new(
                "bitwise_and does not work in float",
                SoulErrorKind::LlvmError,
                None,
            )),
            _ => Err(SoulError::new(
                format!(
                    "bitwise_and requires int or float values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn bit_or(&self, left: IrOperand<'a>, right: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                self.builder.build_or(l, r).map(BasicValueEnum::from)
            }
            (BasicValueEnum::FloatValue(_), BasicValueEnum::FloatValue(_)) => Err(SoulError::new(
                "bitwise_or does not work in float",
                SoulErrorKind::LlvmError,
                None,
            )),
            _ => Err(SoulError::new(
                format!(
                    "bitwise_or requires int or float values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn bit_xor(&self, left: IrOperand<'a>, right: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                self.builder.build_xor(l, r).map(BasicValueEnum::from)
            }
            (BasicValueEnum::FloatValue(_), BasicValueEnum::FloatValue(_)) => Err(SoulError::new(
                "bitwise_xor does not work in float",
                SoulErrorKind::LlvmError,
                None,
            )),
            _ => Err(SoulError::new(
                format!(
                    "bitwise_xor requires int or float values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn compare(
        &self,
        cmp: IrCompare,
        left: IrOperand<'a>,
        right: IrOperand<'a>,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let left_type = self.get_type(left.info.type_id)?;
        let right_type = self.get_type(right.info.type_id)?;
        if left_type.kind.is_array() && right_type.kind.is_array() {
            return self.array_compare(cmp, left, left_type, right, right_type, generics);
        }

        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                let predict = if self.is_signed_interger(&left.info) {
                    cmp.to_signed_int_cmp()
                } else {
                    cmp.to_unsigned_int_cmp()
                };

                self.builder
                    .build_int_compare(predict, l, r)
                    .map(BasicValueEnum::from)
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_compare(cmp.to_float_cmp_no_nan(), l, r)
                .map(BasicValueEnum::from),

            (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
                let l = self
                    .builder
                    .build_ptr_to_int(l, self.default_int_type)?;

                let r = self
                    .builder
                    .build_ptr_to_int(r, self.default_int_type)?;

                self.builder
                    .build_int_compare(cmp.to_unsigned_int_cmp(), l, r)
                    .map(BasicValueEnum::from)
            }
            _ => Err(SoulError::new(
                format!(
                    "bitwise_xor requires int or float values (left: {:?}, right: {:?})",
                    left.value, right.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: left.info.clone(),
        })
    }

    fn array_compare(
        &self,
        cmp: IrCompare,
        left: IrOperand<'a>,
        left_type: &ThirType,
        right: IrOperand<'a>,
        right_type: &ThirType,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let (
            ThirTypeKind::Array {
                element,
                kind: left_kind,
            },
            ThirTypeKind::Array {
                element: _,
                kind: right_kind,
            },
        ) = (left_type.kind, right_type.kind)
        else {
            return Err(soul_error_internal!(
                "left_type and right_type should be ThirTypeKind::Array",
                None
            ));
        };

        let left_len = self.get_array_len(left, left_kind)?;
        let right_len = self.get_array_len(right, right_kind)?;
        if cmp.is_order_kind() {
            return self.compare(cmp, left_len, right_len, generics);
        }

        let left_ptr = self.get_array_ptr(left, left_kind)?;
        let right_ptr = self.get_array_ptr(right, right_kind)?;

        let sizeof = self.sizeof(element, generics)? as u64;
        let u32_type = self.context.i32_type();
        let element_size = u32_type.const_int(sizeof, false);

        let Some(array_equal_fn) = self.internal_functions.arraycmp_function else {
            return Err(soul_error_internal!("arraycmp is not initialized", None));
        };

        let call = self.builder.build_call(
            array_equal_fn,
            &[
                element_size.into(),
                left_ptr.value.into(),
                left_len.value.into(),
                right_ptr.value.into(),
                right_len.value.into(),
            ],
        )?;

        let Some(return_value) = call.try_as_basic_value().basic() else {
            return Err(soul_error_internal!(
                "call to internal 'arraycmp' returned no value but return_place was provided",
                None
            ));
        };

        let value = if cmp == IrCompare::Eq {
            return_value
        } else {
            self.builder
                .build_not(return_value.into_int_value())?
                .into()
        };

        let bool_type_id = self.types.types_table.bool_type;
        let bool_ir_type = self.context.bool_type().into();

        Ok(IrOperand {
            value,
            info: OperandInfo::new_loaded(bool_type_id, bool_ir_type),
        })
    }

    fn get_array_len(&self, operand: IrOperand<'a>, kind: ArrayKind) -> SoulResult<IrOperand<'a>> {
        let len_type = self.types.types_table.index_type;
        let len_ir_type = self.default_int_type;

        match kind {
            ArrayKind::MutSlice | ArrayKind::HeapArray | ArrayKind::ConstSlice => {
                self.extract_struct_field(operand, 1, len_type, len_ir_type.into())
            }
            ArrayKind::StackArray(num) => Ok(IrOperand {
                value: len_ir_type.const_int(num, false).into(),
                info: OperandInfo::new_loaded(len_type, len_ir_type.into()),
            }),
        }
    }

    fn get_array_ptr(&self, operand: IrOperand<'a>, kind: ArrayKind) -> SoulResult<IrOperand<'a>> {
        let ptr_type = self.types.types_table.index_type;
        let ptr_ir_type = self.context.ptr_type(AddressSpace::default());

        match kind {
            ArrayKind::MutSlice | ArrayKind::HeapArray | ArrayKind::ConstSlice => {
                self.extract_struct_field(operand, 0, ptr_type, ptr_ir_type.into())
            }
            ArrayKind::StackArray(_) => {
                let ptr = operand.get_or_convert_pointer(&self.builder)?;
                Ok(IrOperand {
                    value: ptr.into(),
                    info: OperandInfo::new_loaded(ptr_type, ptr_ir_type.into()),
                })
            }
        }
    }

    fn extract_struct_field(
        &self,
        operand: IrOperand<'a>,
        field_index: u32,
        field_type_id: TypeId,
        field_ir_type: BasicTypeEnum<'a>,
    ) -> SoulResult<IrOperand<'a>> {
        let struct_ty = operand.info.ir_type.into_struct_type();
        let tmp = self.builder.build_alloca(struct_ty, "tmp")?;
        self.builder.store_operand(tmp, operand)?;
        let field_ptr =
            self.builder
                .build_struct_gep_index(struct_ty, tmp, field_index, "field_ptr")?;
        let field_val = self
            .builder
            .build_load(field_ir_type, field_ptr, "field_val")?;

        Ok(IrOperand {
            value: field_val.into(),
            info: OperandInfo::new_loaded(field_type_id, field_ir_type),
        })
    }

    fn neg(&self, operand: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match operand.value {
            BasicValueEnum::IntValue(l) => self.builder.build_int_neg(l).map(BasicValueEnum::from),
            BasicValueEnum::FloatValue(l) => {
                self.builder.build_float_neg(l).map(BasicValueEnum::from)
            }
            _ => Err(SoulError::new(
                format!(
                    "negative requires int or float values operand: {:?}",
                    operand.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: operand.info.clone(),
        })
    }

    fn not(&self, operand: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match operand.value {
            BasicValueEnum::IntValue(l) => self.builder.build_not(l).map(BasicValueEnum::from),
            BasicValueEnum::FloatValue(_) => Err(SoulError::new(
                "not does not work in float",
                SoulErrorKind::LlvmError,
                None,
            )),
            _ => Err(SoulError::new(
                format!(
                    "not requires int or float values operand: {:?}",
                    operand.value
                ),
                SoulErrorKind::LlvmError,
                None,
            )),
        }?;

        Ok(IrOperand {
            value,
            info: operand.info.clone(),
        })
    }
}

#[derive(PartialEq, Eq)]
enum IrCompare {
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    NotEq,
}
impl IrCompare {
    fn is_order_kind(&self) -> bool {
        match self {
            IrCompare::Lt | IrCompare::Gt | IrCompare::Le | IrCompare::Ge => true,
            _ => false,
        }
    }

    fn to_signed_int_cmp(&self) -> IntPredicate {
        match self {
            IrCompare::Lt => IntPredicate::SLT,
            IrCompare::Gt => IntPredicate::SGT,
            IrCompare::Le => IntPredicate::SLE,
            IrCompare::Ge => IntPredicate::SGE,
            IrCompare::Eq => IntPredicate::EQ,
            IrCompare::NotEq => IntPredicate::NE,
        }
    }

    fn to_unsigned_int_cmp(&self) -> IntPredicate {
        match self {
            IrCompare::Lt => IntPredicate::ULT,
            IrCompare::Gt => IntPredicate::UGT,
            IrCompare::Le => IntPredicate::ULE,
            IrCompare::Ge => IntPredicate::UGE,
            IrCompare::Eq => IntPredicate::EQ,
            IrCompare::NotEq => IntPredicate::NE,
        }
    }

    fn to_float_cmp_no_nan(&self) -> FloatPredicate {
        match self {
            IrCompare::Lt => FloatPredicate::OLT,
            IrCompare::Gt => FloatPredicate::OGT,
            IrCompare::Le => FloatPredicate::OLE,
            IrCompare::Ge => FloatPredicate::OGE,
            IrCompare::Eq => FloatPredicate::OEQ,
            IrCompare::NotEq => FloatPredicate::ONE,
        }
    }
}
