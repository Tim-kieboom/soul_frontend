use ast::{BinaryOperator, UnaryOperator};
use inkwell::{FloatPredicate, IntPredicate, values::BasicValueEnum};
use mir_parser::mir;
use soul_utils::error::{SoulError, SoulErrorKind, SoulResult};

use crate::{GenericSubstitute, IrOperand, LlvmBackend, build_error};

impl<'f, 'a> LlvmBackend<'f, 'a> {
    pub(super) fn lower_binary(
        &self,
        left: &mir::Operand,
        operator: &BinaryOperator,
        right: &mir::Operand,
        generics: &GenericSubstitute,
    ) -> SoulResult<IrOperand<'a>> {
        let ir_left = self.lower_operand(left, generics)?;
        let ir_right = self.lower_operand(right, generics)?;

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
            ast::BinaryOperatorKind::NotEq => self.compare(IrCompare::NotEq, ir_left, ir_right),
            ast::BinaryOperatorKind::Eq => self.compare(IrCompare::Eq, ir_left, ir_right),
            ast::BinaryOperatorKind::Lt => self.compare(IrCompare::Lt, ir_left, ir_right),
            ast::BinaryOperatorKind::Gt => self.compare(IrCompare::Gt, ir_left, ir_right),
            ast::BinaryOperatorKind::Le => self.compare(IrCompare::Le, ir_left, ir_right),
            ast::BinaryOperatorKind::Ge => self.compare(IrCompare::Ge, ir_left, ir_right),

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
        let ir_value = self.lower_operand(value, generics)?;

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
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_add(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_add(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_sub(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_sub(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_int_mul(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_mul(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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
                        .build_int_signed_div(l, r, "rvalue")
                        .map_err(build_error)
                        .map(BasicValueEnum::from)
                } else {
                    self.builder
                        .build_int_unsigned_div(l, r, "rvalue")
                        .map_err(build_error)
                        .map(BasicValueEnum::from)
                }
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_div(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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
                        .build_int_signed_rem(l, r, "rvalue")
                        .map_err(build_error)
                        .map(BasicValueEnum::from)
                } else {
                    self.builder
                        .build_int_unsigned_rem(l, r, "rvalue")
                        .map_err(build_error)
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
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_and(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_or(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => self
                .builder
                .build_xor(l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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
    ) -> SoulResult<IrOperand<'a>> {
        let value = match (left.value, right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                let predict = if self.is_signed_interger(&left.info) {
                    cmp.to_signed_int_cmp()
                } else {
                    cmp.to_unsigned_int_cmp()
                };

                self.builder
                    .build_int_compare(predict, l, r, "rvalue")
                    .map_err(build_error)
                    .map(BasicValueEnum::from)
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => self
                .builder
                .build_float_compare(cmp.to_float_cmp_no_nan(), l, r, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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

    fn neg(&self, operand: IrOperand<'a>) -> SoulResult<IrOperand<'a>> {
        let value = match operand.value {
            BasicValueEnum::IntValue(l) => self
                .builder
                .build_int_neg(l, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
            BasicValueEnum::FloatValue(l) => self
                .builder
                .build_float_neg(l, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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
            BasicValueEnum::IntValue(l) => self
                .builder
                .build_not(l, "rvalue")
                .map_err(build_error)
                .map(BasicValueEnum::from),
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

enum IrCompare {
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    NotEq,
}
impl IrCompare {
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
