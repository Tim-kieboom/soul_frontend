use hir_model::{HirType, HirTypeKind, Primitive, PrimitiveSize};
use soul_utils::{error::{SoulError, SoulErrorKind, SoulResult}, span::Span};

use crate::{TypedContext, model::InferType};

impl<'a> TypedContext<'a> {

    pub(crate) fn try_resolve_untyped(&mut self, ltype: &mut InferType, to_primitive: Option<Primitive>, span: Span) -> bool {

        let kown_type = match ltype {
            InferType::Known(hir_type) => hir_type,
            InferType::Variable(_,_) => return true,
        };

        let untyped = match &kown_type.kind {
            HirTypeKind::Primitive(Primitive::UntypedFloat) => Untyped::Float,
            HirTypeKind::Primitive(Primitive::UntypedUint) => Untyped::Uint,
            HirTypeKind::Primitive(Primitive::UntypedInt) => Untyped::Int,
            _ => return true,
        };

        let to_type = match to_primitive {
            Some(val) => val,
            None => {
                kown_type.kind = HirTypeKind::Primitive(untyped.to_default());
                return true
            }
        };

        if to_type.compatible(&untyped.to_primitive()) {
            kown_type.kind = HirTypeKind::Primitive(to_type);
            return true
        }

        self.log_error(SoulError::new(
            format!("untyped type '{}' can not be resolved to '{}'", untyped.to_primitive().display(), to_type.display()),
            SoulErrorKind::UnifyTypeError,
            Some(span),
        ));
        false
    }

    pub(crate) fn unify(&mut self, ltype: &InferType, rtype: &InferType, span: Span) {

        let lsolve = self.environment.resolve(ltype);
        let rsolve = self.environment.resolve(rtype);

        match (lsolve, rsolve) {
            (InferType::Variable(variable, _), ty)
            | (ty, InferType::Variable(variable, _)) => {
                
                self.environment.insert_substitution(variable, ty);
            }

            (InferType::Known(a), InferType::Known(b)) => {
                match self.unify_type(&a, &b, span) {
                    Ok(()) => (),
                    Err(err) => self.log_error(err),
                }
            }
        }
    }

    pub(crate) fn unify_ltype(&mut self, ltype: &HirType, rtype: &InferType, span: Span) {
        let infer_solve = self.environment.resolve(rtype);

        match infer_solve {
            InferType::Variable(variable, _) => {
                self.environment.insert_substitution(variable, InferType::Known(ltype.clone()));
            }

            InferType::Known(a) => {
                match self.unify_type(ltype, &a, span) {
                    Ok(()) => (),
                    Err(err) => self.log_error(err),
                }
            }
        }
    }
    
    pub(crate) fn unify_rtype(&mut self, ltype: &InferType, rtype: &HirType, span: Span) {
        let infer_solve = self.environment.resolve(ltype);

        match infer_solve {
            InferType::Variable(variable, _) => {
                self.environment.insert_substitution(variable, InferType::Known(rtype.clone()));
            }

            InferType::Known(a) => {
                match self.unify_type(&a, rtype, span) {
                    Ok(()) => (),
                    Err(err) => self.log_error(err),
                }
            }
        }
    }

    fn unify_type(&mut self, a: &HirType, b: &HirType, span: Span) -> SoulResult<()> {
        let incompatible_msg = match a.unify_compatible(&b) {
            Err(msg) => msg,
            Ok(()) => return Ok(()),
        };


        Err(
            SoulError::new(
                format!("type '{}' and '{}' mismatched because {incompatible_msg}", a.display(), b.display()),
                SoulErrorKind::UnifyTypeError,
                Some(span),
            )
        )
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
