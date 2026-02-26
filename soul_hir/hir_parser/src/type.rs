use hir::{HirType, HirTypeKind, IdAlloc, TypeId, TypesMap};
use soul_utils::{
    soul_error_internal,
    soul_names::{PrimitiveTypes, TypeModifier},
    span::Span,
};

use crate::HirContext;

const CHAR: HirType = HirType::new(hir::HirTypeKind::Primitive(PrimitiveTypes::Char));

impl<'a> HirContext<'a> {
    /// this function is needed is the borrow checker does not validate self.lower_type
    pub(crate) fn convert_type(ty: &ast::SoulType, types: &mut TypesMap) -> hir::TypeId {
        let ty = match &ty.kind {
            ast::TypeKind::None => HirTypeKind::None,
            ast::TypeKind::Type => HirTypeKind::Type,
            ast::TypeKind::Pointer(inner) => HirTypeKind::Pointer(Self::convert_type(inner, types)),
            ast::TypeKind::Optional(inner) => {
                HirTypeKind::Optional(Self::convert_type(inner, types))
            }
            ast::TypeKind::Array(array) => HirTypeKind::Array {
                element: Self::convert_type(&array.of_type, types),
                kind: array.kind,
            },
            ast::TypeKind::Reference(reference) => HirTypeKind::Ref {
                of_type: Self::convert_type(&reference.inner, types),
                mutable: reference.mutable,
            },
            ast::TypeKind::Primitive(prim) => HirTypeKind::Primitive(*prim),
        };
        types.insert(HirType::new(ty))
    }

    pub(crate) fn lower_type(&mut self, ty: &ast::SoulType) -> hir::TypeId {
        Self::convert_type(ty, &mut self.hir.types)
    }

    pub(crate) fn type_from_literal(&mut self, literal: &ast::Literal) -> TypeId {
        let ty = match literal.get_literal_type().to_internal_primitive_type() {
            ast::TypeResult::Primitive(val) => HirTypeKind::Primitive(val),
            ast::TypeResult::Str => HirTypeKind::Array {
                element: self.add_type(CHAR),
                kind: ast::ArrayKind::ConstSlice,
            },
        };

        self.add_type(HirType::new(ty))
    }

    pub(crate) fn type_from_array(&mut self, array: &ast::Array, span: Span) -> TypeId {
        if array.collection_type.is_some() {
            self.log_error(soul_error_internal!(
                "collection type in array is unstable",
                Some(span)
            ));
            return TypeId::error();
        }

        let kind = ast::ArrayKind::StackArray(array.values.len() as u64);
        let element = match &array.element_type {
            Some(val) => self.lower_type(val),
            None => self.new_infer_type(span),
        };

        let array_ty = self.add_type(HirType::new(HirTypeKind::Array { element, kind }));
        array_ty
    }

    pub(crate) fn new_infer_type(&mut self, span: Span) -> TypeId {
        self.hir.types.new_infertype(None, span)
    }

    pub(crate) fn new_infer_with_modifier(&mut self, modifier: TypeModifier, span: Span) -> TypeId {
        self.hir.types.new_infertype(Some(modifier), span)
    }

    pub(crate) fn add_type(&mut self, ty: HirType) -> TypeId {
        self.hir.types.insert(ty)
    }
}
