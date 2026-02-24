use ast::{ArrayKind, BinaryOperator, BinaryOperatorKind, UnaryOperator};
use hir::{BlockId, ExpressionId, FunctionId, HirType, HirTypeKind, IdAlloc, TypeId};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    span::Span,
};

use crate::HirTypedContext;

const MUT: bool = true;
const CONST: bool = false;

impl<'a> HirTypedContext<'a> {
    pub(crate) fn infer_expression(&mut self, expression_id: hir::ExpressionId) -> TypeId {
        if self.type_table.expressions.get(expression_id) == Some(&TypeId::error()) {
            return TypeId::error();
        }

        let value = &self.hir.expressions[expression_id];
        let span = self.expression_span(expression_id);
        let ty = match &value.kind {
            hir::ExpressionKind::Null => self.new_infer_optional(span),
            hir::ExpressionKind::Load(place) => self.infer_place(place),
            hir::ExpressionKind::Block(body) => self.infer_block(*body),
            hir::ExpressionKind::Local(local) => self.type_table.locals[*local],
            hir::ExpressionKind::Literal(_) => value.ty, /*already handled by hir*/
            hir::ExpressionKind::DeRef(inner) => {
                let inner = self.infer_expression(*inner);
                let deref = self.get_type(inner).try_deref(&self.hir.types, span);
                match deref {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        TypeId::error()
                    }
                }
            }
            hir::ExpressionKind::Function(function) => self.hir.functions[*function].return_type,
            hir::ExpressionKind::Ref { place, mutable } => {
                let place_type = self.infer_place(place);
                let resolved = self.resolve_type_lazy(place_type, span);
                let resolved_ty = self.get_type(resolved);
                let ty = match &resolved_ty.kind {
                    hir::HirTypeKind::Array {
                        element,
                        kind: ArrayKind::HeapArray | ArrayKind::StackArray(_),
                    } => {
                        let kind = match *mutable {
                            MUT => ArrayKind::MutSlice,
                            CONST => ArrayKind::ConstSlice,
                        };
                        HirType::new(hir::HirTypeKind::Array {
                            element: *element,
                            kind,
                        })
                    }

                    _ => HirType::new(hir::HirTypeKind::Ref {
                        of_type: place_type,
                        mutable: *mutable,
                    }),
                };

                self.add_type(ty)
            }
            hir::ExpressionKind::Cast { value, cast_to } => {
                let span = self.expression_span(*value);
                let from = self.infer_expression(*value);
                let from_type = self.get_type(from);
                let to_type = self.get_type(*cast_to);
                match from_type.unify_primitive_cast(
                    &self.type_table.types,
                    to_type,
                    self.is_in_unsafe,
                ) {
                    Ok(()) => (),
                    Err(err) => self.log_error(SoulError::new(
                        err,
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    )),
                }
                *cast_to
            }
            hir::ExpressionKind::While { condition, body } => {
                self.infer_while(*condition, *body, span)
            }
            hir::ExpressionKind::Unary {
                operator,
                expression,
            } => self.infer_unary(operator, *expression, span),
            hir::ExpressionKind::Binary {
                left,
                operator,
                right,
            } => self.infer_binary(*left, operator, *right, span),
            hir::ExpressionKind::Call {
                function,
                callee,
                arguments,
            } => self.infer_call(*function, *callee, arguments),
            hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
            } => self.infer_if(*condition, *then_block, *else_block),
            hir::ExpressionKind::InnerRawStackArray { .. } => value.ty,
        };

        if let HirTypeKind::InferType(id, _) = &self.get_type(value.ty).kind {
            self.infer_table.add_infer_binding(*id, ty);
        }

        self.type_expression(expression_id, ty);
        ty
    }

    fn infer_if(
        &mut self,
        condition: ExpressionId,
        then_block: BlockId,
        else_block: Option<BlockId>,
    ) -> TypeId {
        let bool = self.add_type(HirType::bool_type());

        let condition_span = self.expression_span(condition);
        let condition_type = self.infer_expression(condition);
        self.unify(condition, bool, condition_type, condition_span);

        let then_type = self.infer_block(then_block);
        let then_span = self.block_span(then_block);
        let else_block = match else_block {
            Some(val) => val,
            None => {
                self.unify(ExpressionId::error(), self.none_type, then_type, then_span);
                return self.none_type;
            }
        };

        let else_type = self.infer_block(else_block);
        let else_span = self.block_span(else_block);
        self.unify(ExpressionId::error(), then_type, else_type, else_span);

        self.get_priority_type(then_type, else_type)
    }

    fn infer_call(
        &mut self,
        function_id: FunctionId,
        callee: Option<ExpressionId>,
        arguments: &Vec<ExpressionId>,
    ) -> TypeId {
        let function = &self.hir.functions[function_id];
        let return_type = self.type_table.functions[function_id];
        if let Some(callee) = callee {
            self.infer_expression(callee);
        }

        if function.parameters.len() != arguments.len() {
            self.log_error(SoulError::new(
                format!(
                    "functionCall has {} arguments but expects {} arguments",
                    arguments.len(),
                    function.parameters.len()
                ),
                SoulErrorKind::InvalidContext,
                Some(function.name.span),
            ));
            return return_type;
        }

        for (argument, parameter) in arguments.iter().zip(function.parameters.iter()) {
            let ty = self.infer_expression(*argument);
            let span = self.expression_span(*argument);
            self.unify(*argument, parameter.ty, ty, span);
        }

        return_type
    }

    fn infer_binary(
        &mut self,
        left: ExpressionId,
        operator: &BinaryOperator,
        right: ExpressionId,
        span: Span,
    ) -> TypeId {
        let left_id = self.infer_expression(left);
        let right_id = self.infer_expression(right);
        let binary_typecheck = to_binary_typecheck(&operator.node);
        match binary_typecheck {
            BinaryTypeCheck::Logical => {
                let bool = self.add_type(HirType::bool_type());
                self.unify(left, bool, left_id, span);
                self.unify(right, bool, right_id, span);
                bool
            }
            BinaryTypeCheck::Equal | BinaryTypeCheck::Compare => {
                self.unify(right, left_id, right_id, span);
                self.add_type(HirType::bool_type())
            }
            BinaryTypeCheck::Bitwise | BinaryTypeCheck::Numeric => {
                self.unify(right, left_id, right_id, span);
                let left_type = self.get_type(left_id);
                let right_type = self.get_type(right_id);
                if !left_type.is_numeric() {
                    self.log_error(SoulError::new(
                        format!(
                            "type is '{}' but can only be used for number types (f32, uint, int, i32, ect..)", 
                            left_type.display(&self.type_table.types)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(self.expression_span(left)),
                    ));
                    return TypeId::error();
                }

                if !right_type.is_numeric() {
                    self.log_error(SoulError::new(
                        format!(
                            "type is '{}' but can only be used for number types (f32, uint, int, i32, ect..)", 
                            left_type.display(&self.type_table.types)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(self.expression_span(left)),
                    ));
                    return TypeId::error();
                }

                self.get_priority_type(left_id, right_id)
            }
        }
    }

    fn infer_unary(&mut self, operator: &UnaryOperator, value: ExpressionId, span: Span) -> TypeId {
        use ast::UnaryOperatorKind as Unary;

        match operator.node {
            Unary::Invalid => todo!("should not have invalid"),
            Unary::Neg => {
                let value_type = self.infer_expression(value);
                let ty = self.get_type(value_type);
                if !ty.is_numeric() {
                    self.log_error(SoulError::new(
                        format!(
                            "type is '{}' but can only be used for number types (f32, uint, int, i32, ect..)", 
                            operator.node.as_str()
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                    return TypeId::error();
                }

                value_type
            }
            Unary::Not => {
                let value_type = self.infer_expression(value);
                let ty = self.get_type(value_type);
                if !ty.is_boolean() {
                    self.log_error(SoulError::new(
                        format!(
                            "type is '{}' but can only be used for bool type",
                            operator.node.as_str()
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                    return TypeId::error();
                }

                value_type
            }
            Unary::Increment { .. } | Unary::Decrement { .. } => {
                let value_type = self.infer_expression(value);
                let ty = self.get_type(value_type);
                if !ty.is_numeric() {
                    self.log_error(SoulError::new(
                        format!(
                            "type is '{}' but can only be used for number types (f32, uint, int, i32, ect..)",
                            operator.node.as_str()
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                    return TypeId::error();
                }

                value_type
            }
        }
    }

    fn infer_while(
        &mut self,
        condition: Option<ExpressionId>,
        body: BlockId,
        span: Span,
    ) -> TypeId {
        let return_type = self.infer_block(body);

        let condition = match condition {
            Some(val) => val,
            None => return return_type,
        };

        let condition_type = self.infer_expression(condition);
        let bool_type = self.add_type(HirType::bool_type());

        self.unify(condition, bool_type, condition_type, span);

        if return_type != self.none_type {
            self.log_error(SoulError::new(
                "while loops with a condition can not have return type",
                SoulErrorKind::InvalidContext,
                Some(span),
            ));
            return TypeId::error();
        }

        self.none_type
    }

    fn new_infer_optional(&mut self, span: Span) -> TypeId {
        let id = self.infer_table.alloc(span);
        let infer = self.add_type(HirType::infer_type(id, span));
        self.add_type(HirType::new(hir::HirTypeKind::Optional(infer)))
    }
}

enum BinaryTypeCheck {
    Equal,
    Logical,
    Compare,
    Numeric,
    Bitwise,
}

fn to_binary_typecheck(operator: &BinaryOperatorKind) -> BinaryTypeCheck {
    use ast::BinaryOperatorKind as Binary;

    match operator {
        Binary::Add
        | Binary::Sub
        | Binary::Mul
        | Binary::Div
        | Binary::Log
        | Binary::Pow
        | Binary::Mod
        | Binary::Root => BinaryTypeCheck::Numeric,

        Binary::Eq | Binary::NotEq => BinaryTypeCheck::Equal,

        Binary::Lt | Binary::Gt | Binary::Le | Binary::Ge => BinaryTypeCheck::Compare,

        Binary::LogOr | Binary::LogAnd => BinaryTypeCheck::Logical,

        Binary::BitOr | Binary::BitAnd | Binary::BitXor => BinaryTypeCheck::Bitwise,

        Binary::Range | Binary::TypeOf => todo!("{} not yet impl", operator.as_str()),
        Binary::Invalid => todo!("should not be invalid"),
    }
}
