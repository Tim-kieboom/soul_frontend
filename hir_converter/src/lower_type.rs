use crate::HirLowerer;
use soul_ast::abstract_syntax_tree::{self as ast, SoulType};
use soul_hir::{self as hir, HirType};

impl<'hir> HirLowerer<'hir> {
    pub(super) fn get_hir_type(&mut self, ty: &SoulType) -> Option<HirType> {
        let ast_generics = &ty.generics;
        let mut generics = Vec::with_capacity(ast_generics.len());
        for el in ast_generics {
            generics.push(self.generics_define_to_type(el)?);
        }

        Some(HirType {
            kind: hir::HirTypeKind::None,
            modifier: ty.modifier,
            generics,
        })
    }

    pub(super) fn generics_define_to_type(
        &mut self,
        generics: &ast::GenericDefine,
    ) -> Option<hir::GenericDefine> {
        Some(match generics {
            ast::GenericDefine::Type(soul_type) => {
                hir::GenericDefine::Type(self.get_hir_type(soul_type)?)
            }
            ast::GenericDefine::Lifetime(spanned) => hir::GenericDefine::Lifetime(spanned.clone()),
            ast::GenericDefine::Expression(spanned) => {
                hir::GenericDefine::Expression(self.lower_expression(spanned)?)
            }
        })
    }
}
