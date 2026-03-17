use hir::TypeId;
use mir_parser::mir::{Operand, OperandKind};
use soul_utils::{error::SoulResult, soul_error_internal, soul_names::PrimitiveSize};

use crate::{IrOperand, LlvmBackend, build_error};

impl<'a> LlvmBackend<'a> {
    pub(crate) fn lower_operand(&self, condition: &Operand) -> SoulResult<IrOperand<'a>> {
        Ok(match &condition.kind {
            OperandKind::Temp(temp_id) => {
                self.temps
                    .get(*temp_id)
                    .copied()
                    .ok_or(soul_error_internal!(
                        format!("{:?} not found", *temp_id),
                        None
                    ))?
            }
            OperandKind::Local(local_id) => {
                let local = &self.mir.tree.locals[*local_id];
                let ty = match self.lower_type(local.ty)? {
                    Some(val) => val,
                    None => self.context.i8_type().into(),
                };
                let ptr = self.locals[*local_id];
                let is_signed_interger = self
                    .types
                    .types
                    .get_type(local.ty)
                    .ok_or(soul_error_internal!(
                        format!("{:?} not found", local.ty),
                        None
                    ))?
                    .is_any_int_type();

                return self
                    .builder
                    .build_load(ty, ptr, "load")
                    .map(|value| IrOperand {
                        value,
                        is_signed_interger,
                    })
                    .map_err(|err| build_error(err));
            }
            OperandKind::Comptime(literal) => self.lower_literal(literal, condition.ty)?,
            OperandKind::Ref { .. } => {
                todo!("ref not yet impl")
            }
            OperandKind::None => {
                return Err(soul_error_internal!("operand should be Some(_)", None));
            }
        })
    }

    pub(crate) fn lower_literal(
        &self,
        literal: &ast::Literal,
        should_be: TypeId,
    ) -> SoulResult<IrOperand<'a>> {
        Ok(match literal {
            ast::Literal::Int(value) => {
                let size = match self
                    .types
                    .types
                    .get_type(should_be)
                    .expect("should have type")
                    .kind
                {
                    hir::HirTypeKind::Primitive(primitive_types) => {
                        primitive_types.to_primitive_size()
                    }
                    _ => {
                        return Err(soul_error_internal!(
                            "literal should be primitive type",
                            None
                        ));
                    }
                };

                let negative = *value < 0;
                let int_type = match size {
                    PrimitiveSize::CharSize => self.default_char_type,
                    PrimitiveSize::IntSize => self.default_int_type,
                    PrimitiveSize::Bit8 => self.context.i8_type(),
                    PrimitiveSize::Bit16 => self.context.i16_type(),
                    PrimitiveSize::Bit32 => self.context.i32_type(),
                    PrimitiveSize::Bit64 => self.context.i64_type(),
                    PrimitiveSize::Bit128 => self.context.i128_type(),
                };

                let value = int_type.const_int(value.abs() as u64, negative).into();

                IrOperand {
                    value,
                    is_signed_interger: true,
                }
            }
            ast::Literal::Uint(value) => {
                let size = match self
                    .types
                    .types
                    .get_type(should_be)
                    .expect("should have type")
                    .kind
                {
                    hir::HirTypeKind::Primitive(primitive_types) => {
                        primitive_types.to_primitive_size()
                    }
                    _ => {
                        return Err(soul_error_internal!(
                            "literal should be primitive type",
                            None
                        ));
                    }
                };

                let int_type = match size {
                    PrimitiveSize::CharSize => self.default_char_type,
                    PrimitiveSize::IntSize => self.default_int_type,
                    PrimitiveSize::Bit8 => self.context.i8_type(),
                    PrimitiveSize::Bit16 => self.context.i16_type(),
                    PrimitiveSize::Bit32 => self.context.i32_type(),
                    PrimitiveSize::Bit64 => self.context.i64_type(),
                    PrimitiveSize::Bit128 => self.context.i128_type(),
                };

                let value = int_type.const_int(*value as u64, false).into();

                IrOperand {
                    value,
                    is_signed_interger: false,
                }
            }
            ast::Literal::Float(value) => {
                let size = match self
                    .types
                    .types
                    .get_type(should_be)
                    .expect("should have type")
                    .kind
                {
                    hir::HirTypeKind::Primitive(primitive_types) => {
                        primitive_types.to_primitive_size()
                    }
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

                IrOperand {
                    value,
                    is_signed_interger: false,
                }
            }
            ast::Literal::Bool(value) => {
                let value = self
                    .context
                    .bool_type()
                    .const_int(*value as u64, false)
                    .into();

                IrOperand {
                    value,
                    is_signed_interger: false,
                }
            }
            ast::Literal::Char(value) => {
                let value = self
                    .context
                    .i8_type()
                    .const_int(*value as u64, false)
                    .into();

                IrOperand {
                    value,
                    is_signed_interger: false,
                }
            }
            ast::Literal::Str(_) => {
                todo!("string literal not yet impl")
            }
        })
    }
}