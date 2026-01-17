use crate::HirLowerer;
use hir_model::{self as hir, HirType, Primitive};
use parser_models::ast::{self, SoulType};

impl HirLowerer {
    pub(crate) fn lower_type(&mut self, ty: &SoulType) -> Option<HirType> {
        let kind = match &ty.kind {
            ast::TypeKind::None => hir::HirTypeKind::None,
            ast::TypeKind::Type => hir::HirTypeKind::Type,
            ast::TypeKind::Primitive(prim) => {
                hir::HirTypeKind::Primitive(
                    Primitive::from_internal_primitive(*prim)
                )
            }
            ast::TypeKind::Array(array_type) => {
                hir::HirTypeKind::Array(
                    Box::new(self.lower_type(array_type)?)
                )
            },
            ast::TypeKind::Reference(reference_type) => {
                let ty = Box::new(self.lower_type(&reference_type.inner)?);
                hir::HirTypeKind::Ref {
                    ty,
                    mutable: reference_type.mutable,
                }
            }
            ast::TypeKind::Pointer(soul_type) => {
                hir::HirTypeKind::Pointer(
                    Box::new(self.lower_type(soul_type)?)
                )
            }
            ast::TypeKind::Optional(soul_type) => {
                hir::HirTypeKind::Pointer(
                    Box::new(self.lower_type(soul_type)?)
                )
            }
        };

        Some(HirType {
            kind,
            modifier: ty.modifier,
            span: ty.span,
        })
    }
}

