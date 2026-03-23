use ast::{ArrayKind, BinaryOperator, BinaryOperatorKind, UnaryOperator};
use hir::{Binary, BlockId, ExpressionId, HirType, HirTypeKind, RefTypeId, TypeId, Unary};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc},
    span::Span,
    vec_map::VecMap,
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
            hir::ExpressionKind::Error => TypeId::error(),
            hir::ExpressionKind::Null => self.new_infer_optional(span),
            hir::ExpressionKind::Load(place) => self.infer_place(place),
            hir::ExpressionKind::Block(body) => self.infer_block(*body),
            hir::ExpressionKind::Local(local) => self.type_table.locals[*local],
            hir::ExpressionKind::Literal(_) => value.ty, /*already handled by hir*/
            hir::ExpressionKind::DeRef(inner) => {
                let inner = self.infer_expression(*inner);
                let ty = match self.resolve_type_strict(inner, span) {
                    Some(val) => val,
                    None => return TypeId::error(),
                };

                let deref = self.id_to_type(ty).try_deref(&self.hir.types, span);
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
                let resolved_ty = self.id_to_type(resolved);
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
                let from_type = self.id_to_type(from);
                let to_type = self.ref_to_type(*cast_to);
                match from_type.unify_primitive_cast(&self.type_table.types, to_type) {
                    Ok(()) => (),
                    Err(err) => self.log_error(SoulError::new(
                        err,
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    )),
                }

                self.ref_to_id(*cast_to)
            }
            hir::ExpressionKind::While { condition, body } => {
                self.infer_while(*condition, *body, span)
            }
            hir::ExpressionKind::Unary(Unary {
                operator,
                expression,
            }) => self.infer_unary(operator, *expression, span),
            hir::ExpressionKind::Binary(Binary {
                left,
                operator,
                right,
            }) => self.infer_binary(*left, operator, *right, span),
            hir::ExpressionKind::Call {
                callee,
                function,
                generics,
                arguments,
            } => self.infer_call(*function, *callee, generics, arguments, span),
            hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
                ends_with_else,
            } => self.infer_if(*condition, *then_block, *else_block, *ends_with_else, span),
            hir::ExpressionKind::InnerRawStackArray(_) => value.ty,
        };

        if let HirTypeKind::InferType(id, _) = &self.id_to_type(value.ty).kind {
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
        ends_with_else: bool,
        if_span: Span,
    ) -> TypeId {
        let bool = self.add_type(HirType::bool_type());

        let condition_span = self.expression_span(condition);
        let condition_type = self.infer_expression(condition);
        _ = self.unify(condition, bool, condition_type, condition_span);

        let then_type = self.infer_block(then_block);
        let then_span = self.block_span(then_block);
        let else_block = match else_block {
            Some(val) => val,
            None => {
                _ = self.unify(
                    ExpressionId::error(),
                    self.type_table.none_type,
                    then_type,
                    then_span,
                );
                return self.type_table.none_type;
            }
        };

        if !ends_with_else && then_type != self.type_table.none_type {
            self.log_error(SoulError::new(
                "'if' should end with an 'else' if you want to return a value",
                SoulErrorKind::InvalidContext,
                Some(if_span),
            ));

            return TypeId::error();
        }

        let else_type = self.infer_block(else_block);
        let else_span = self.block_span(else_block);

        _ = self.unify(ExpressionId::error(), then_type, else_type, else_span);
        self.get_priority_type(then_type, else_type)
    }

    fn infer_call(
        &mut self,
        function_id: FunctionId,
        callee: Option<ExpressionId>,
        generics: &Vec<RefTypeId>,
        arguments: &Vec<ExpressionId>,
        span: Span,
    ) -> TypeId {
        let function = match self.hir.functions.get(function_id) {
            Some(val) => val,
            None => return TypeId::error(),
        };

        let mut generic_defines = VecMap::new();
        for (i, generic_id) in function.generics.iter().copied().enumerate() {
            let generic_ty = match generics.get(i) {
                Some(val) => *val,
                None => {
                    let msg = match self.hir.types.generic_name(generic_id) {
                        Some(name) => format!("generic {name} is not defined"),
                        None => format!("generic {:?} is not defined", generic_id),
                    };
                    self.log_error(SoulError::new(
                        msg,
                        SoulErrorKind::GenericDefineError,
                        Some(span),
                    ));
                    RefTypeId::error()
                }
            };
            generic_defines.insert(generic_id, self.ref_to_id(generic_ty));
            self.insert_generic_define(generic_id, generic_ty);
        }

        let unresolved_return_type = self.type_table.functions[function_id];
        let return_type = self.resolve_generic(&generic_defines, unresolved_return_type);

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
                Some(span),
            ));
            return return_type;
        }

        for (argument, parameter) in arguments.iter().zip(function.parameters.iter()) {
            let ty = self.infer_expression(*argument);
            let span = self.expression_span(*argument);

            let should_be = self.resolve_generic(&generic_defines, parameter.ty);
            self.unify(*argument, should_be, ty, span);
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
                let left_type = self.id_to_type(left_id);
                let right_type = self.id_to_type(right_id);
                if !left_type.is_numeric_type() {
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

                if !right_type.is_numeric_type() {
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
                let ty = self.id_to_type(value_type);
                if !ty.is_numeric_type() {
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
                let ty = self.id_to_type(value_type);
                if !ty.is_boolean_type() {
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
                let ty = self.id_to_type(value_type);
                if !ty.is_numeric_type() {
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

        if return_type != self.type_table.none_type {
            self.log_error(SoulError::new(
                "while loops with a condition can not have return type",
                SoulErrorKind::InvalidContext,
                Some(span),
            ));
            return TypeId::error();
        }

        self.type_table.none_type
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
