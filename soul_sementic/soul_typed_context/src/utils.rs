use hir_model::{
    ArrayType, Body, BodyId, Expression, ExpressionId, FunctionSignature, HirType, HirTypeKind,
    Primitive,
};
use parser_models::{
    ast::{ArrayKind, Literal, TypeResult},
    scope::NodeId,
};
use soul_utils::{
    error::SoulError, sementic_level::SementicFault, soul_names::TypeModifier, span::Span
};

use crate::{TypedContextAnalyser, model::InferType};

impl<'a> TypedContextAnalyser<'a> {
    pub(crate) fn get_expression(&mut self, id: ExpressionId) -> &'a Expression {
        &self.tree.root.expressions[id]
    }

    pub(crate) fn get_body(&mut self, id: BodyId) -> &'a Body {
        &self.tree.root.bodies[id]
    }

    pub(crate) fn get_function_signature(&mut self, id: NodeId) -> &'a FunctionSignature {
        &self.tree.root.functions[id]
    }

    pub(crate) fn log_error(&mut self, err: SoulError) {
        self.faults.push(SementicFault::error(err));
    }
}

pub(crate) fn kown_from_literal(literal: &Literal, span: Span) -> InferType {
    InferType::Known(literal_ty(literal, span))
}

pub(crate) fn known_bool(modifier: Option<TypeModifier>, span: Span) -> InferType {
    InferType::Known(bool_ty(modifier, span))
}

pub(crate) fn known_none(span: Span) -> InferType {
    InferType::Known(none_ty(span))
}

pub(crate) fn to_array_ty(element_type: HirType, kind: ArrayKind) -> HirType {
    let span = element_type.span;
    let type_of = Box::new(element_type);
    HirType {
        kind: HirTypeKind::Array(ArrayType { type_of, kind }),
        modifier: None,
        span,
    }
}

pub(crate) fn empty_array_ty(span: Span) -> HirType {
    let type_of = Box::new(HirType {
        kind: HirTypeKind::Untyped,
        modifier: None,
        span,
    });

    HirType {
        kind: HirTypeKind::Array(ArrayType {
            type_of,
            kind: ArrayKind::StackArray(0),
        }),
        modifier: None,
        span,
    }
}

pub(crate) fn literal_ty(literal: &Literal, span: Span) -> HirType {
    let kind = match literal.get_literal_type().to_internal_primitive_type() {
        TypeResult::Primitive(prim) => {
            let primitive = Primitive::from_internal_primitive(prim);
            HirTypeKind::Primitive(primitive)
        }
        TypeResult::Str => HirTypeKind::Str,
    };
    HirType {
        span,
        kind,
        modifier: Some(TypeModifier::Literal),
    }
}

pub(crate) fn primitive_ty(
    primitive: Primitive,
    modifier: Option<TypeModifier>,
    span: Span,
) -> HirType {
    HirType {
        kind: hir_model::HirTypeKind::Primitive(primitive),
        modifier,
        span,
    }
}

pub(crate) fn none_ty(span: Span) -> HirType {
    HirType {
        kind: hir_model::HirTypeKind::None,
        modifier: None,
        span,
    }
}

pub(crate) fn bool_ty(modifier: Option<TypeModifier>, span: Span) -> HirType {
    HirType {
        kind: hir_model::HirTypeKind::Primitive(hir_model::Primitive::Boolean),
        modifier,
        span,
    }
}
