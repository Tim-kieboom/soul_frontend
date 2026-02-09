use hir_model::{
    ArrayType, Binary, ExpressionId, HirType, HirTypeKind, Primitive, PrimitiveSize, UnifyResult
};
use parser_models::scope::NodeId;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_error_internal,
    span::Span,
};

use crate::{
    TypedContextAnalyser,
    model::{InferType, Place}, utils::{bool_ty, none_ty},
};

impl<'a> TypedContextAnalyser<'a> {
    pub(crate) fn try_resolve_untyped_var(&mut self, place: &Place, should_be: &HirType) -> bool {
        let known_type = match place.get_type() {
            InferType::Known(hir_type) => hir_type,
            InferType::Variable(_, _) => return true,
        };

        const IS_INNER_TYPE: bool = false;
        self.inner_resolve_untyped(place.get_id(), known_type, should_be, IS_INNER_TYPE)
    }

    fn inner_resolve_untyped(
        &mut self,
        local_id: NodeId,
        ty: &HirType,
        should_be: &HirType,
        is_inner_type: bool,
    ) -> bool {
        match &ty.kind {
            HirTypeKind::Array(array_type) => {
                self.inner_resolve_untyped(local_id, &array_type.type_of, should_be, true)
            }
            HirTypeKind::Untyped => {
                match self.locals.get_mut(local_id) {
                    Some(val) => {
                        let modifier = match val {
                            InferType::Known(hir_type) => hir_type.modifier,
                            InferType::Variable(_, _) => None,
                        };
                        let mut ty = should_be.clone();
                        ty.modifier = match is_inner_type {
                            true => None,
                            false => modifier,
                        };
                        *val = InferType::Known(ty);
                    }
                    None => {
                        self.log_error(soul_error_internal!(
                            "local_id could not be found",
                            Some(ty.span)
                        ));
                        return false;
                    }
                }
                true
            }
            _ => true,
        }
    }

    pub(crate) fn try_resolve_untyped_number(
        &mut self,
        ltype: &mut InferType,
        to_primitive: Option<Primitive>,
        span: Span,
    ) -> bool {
        let kown_type = match ltype {
            InferType::Known(hir_type) => hir_type,
            InferType::Variable(_, _) => return true,
        };

        self.inner_resolve_untyped_number(kown_type, to_primitive, span)
    }

    fn inner_resolve_untyped_number(
        &mut self,
        kown_type: &mut HirType,
        to_primitive: Option<Primitive>,
        span: Span,
    ) -> bool {
        let untyped = match &mut kown_type.kind {
            HirTypeKind::Primitive(Primitive::UntypedFloat) => Untyped::Float,
            HirTypeKind::Primitive(Primitive::UntypedUint) => Untyped::Uint,
            HirTypeKind::Primitive(Primitive::UntypedInt) => Untyped::Int,
            HirTypeKind::Array(ArrayType { type_of, .. })
            | HirTypeKind::Optional(type_of)
            | HirTypeKind::Pointer(type_of)
            | HirTypeKind::Ref { ty: type_of, .. } => {
                return self.inner_resolve_untyped_number(type_of, to_primitive, span);
            }
            _ => return true,
        };

        let to_type = match to_primitive {
            Some(val) => val,
            None => {
                kown_type.kind = HirTypeKind::Primitive(untyped.to_default());
                return true;
            }
        };

        if to_type.compatible(&untyped.to_primitive()) {
            kown_type.kind = HirTypeKind::Primitive(to_type);
            return true;
        }

        self.log_error(SoulError::new(
            format!(
                "untyped type '{}' can not be resolved to '{}'",
                untyped.to_primitive().display(),
                to_type.display()
            ),
            SoulErrorKind::UnifyTypeError,
            Some(span),
        ));
        false
    }

    pub(crate) fn unify(
        &mut self,
        expression: ExpressionId,
        ty: &InferType,
        should_be: &InferType,
        span: Span,
    ) {
        let ty_solve = self.environment.resolve(ty);
        let should_be_solve = self.environment.resolve(should_be);

        match (ty_solve, should_be_solve) {
            (InferType::Variable(variable, _), ty) | (ty, InferType::Variable(variable, _)) => {
                if ty.contains_variable(variable) {
                    self.log_error(SoulError::new(
                        "contains recurive type",
                        SoulErrorKind::TypeInferenceError,
                        Some(span),
                    ));
                }
                self.environment.insert_substitution(variable, ty);
            }

            (InferType::Known(a), InferType::Known(b)) => {
                match self.unify_type(expression, &a, &b, span) {
                    Ok(()) => (),
                    Err(err) => self.log_error(err),
                }
            }
        }
    }

    pub(crate) fn unify_rtype(
        &mut self,
        expression: ExpressionId,
        ty: &InferType,
        should_be: &HirType,
        span: Span,
    ) {
        let ty_solve = self.environment.resolve(ty);

        match ty_solve {
            InferType::Variable(variable, _) => {
                self.environment
                    .insert_substitution(variable, InferType::Known(should_be.clone()));
            }

            InferType::Known(a) => match self.unify_type(expression, &a, should_be, span) {
                Ok(()) => (),
                Err(err) => {
                    self.log_error(err);
                }
            },
        }
    }

    pub(crate) fn unify_type(
        &mut self,
        expression: ExpressionId,
        a: &HirType,
        b: &HirType,
        span: Span,
    ) -> SoulResult<()> {
        let incompatible_msg = match a.unify_compatible(b) {
            Err(msg) => msg,
            Ok(result) => {
                self.handle_unify_result(expression, result);
                return Ok(());
            }
        };

        Err(SoulError::new(
            format!(
                "has type '{}' but expected type '{}' mismatched because {incompatible_msg}",
                a.display(),
                b.display()
            ),
            SoulErrorKind::UnifyTypeError,
            Some(span),
        ))
    }

    pub(crate) fn unify_primitive_cast(
        &mut self,
        value: &HirType,
        cast: &HirType,
        span: Span,
    ) -> SoulResult<()> {
        if value.is_pointer() && cast.is_pointer() {
            self.log_error(soul_error_internal!(
                "TODO add check if ptr cast is in unsafe",
                Some(span)
            ));
            return Ok(());
        }

        let incompatible_msg = match value.unify_primitive_cast(cast) {
            Err(msg) => msg,
            Ok(()) => return Ok(()),
        };

        Err(SoulError::new(
            format!(
                "type '{}' and '{}' mismatched because {incompatible_msg}",
                value.display(),
                cast.display()
            ),
            SoulErrorKind::UnifyTypeError,
            Some(span),
        ))
    }

    pub(crate) fn resolve_binary_type(
        &mut self,
        binary: &Binary,
        ltype: &InferType,
        rtype: &InferType,
        span: Span,
    ) -> InferType {
        let expression = binary.left;
        let operator = binary.operator.node;

        self.unify(expression, ltype, rtype, span);
        match (ltype, rtype) {
            (InferType::Known(left), InferType::Known(right)) => {
                
                let ty = match operator.is_boolean_operator() {
                    true => bool_ty(None, span),
                    false => left.new_priority(right),
                };

                InferType::Known(ty)
            }
            _ => {
                self.log_error(SoulError::new(
                    "type has to be known at his time", 
                    SoulErrorKind::UnifyTypeError, 
                    Some(span),
                ));
                InferType::Known(none_ty(span))
            }
        }
    }

    fn handle_unify_result(&mut self, expression: ExpressionId, result: UnifyResult) {
        match result {
            UnifyResult::Ok => (),
            UnifyResult::AutoCopy => {
                // TODO: add autoCopy impl check
                self.auto_copys.insert(expression);
            }
        }
    }
}

enum Untyped {
    Float,
    Uint,
    Int,
}
impl Untyped {
    pub fn to_default(&self) -> Primitive {
        match self {
            Untyped::Float => Primitive::Float(PrimitiveSize::Bit32),
            Untyped::Uint => Primitive::Int(PrimitiveSize::SystemSize),
            Untyped::Int => Primitive::Int(PrimitiveSize::SystemSize),
        }
    }

    pub fn to_primitive(&self) -> Primitive {
        match self {
            Untyped::Float => Primitive::UntypedFloat,
            Untyped::Uint => Primitive::UntypedUint,
            Untyped::Int => Primitive::UntypedInt,
        }
    }
}
