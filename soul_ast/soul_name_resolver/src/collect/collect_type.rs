use parser_models::ast::SoulType;

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_type(&mut self, ty: &mut SoulType) {
        
        match &mut ty.kind {
            parser_models::ast::TypeKind::None => (),
            parser_models::ast::TypeKind::Type => (),
            parser_models::ast::TypeKind::Primitive(_) => (),
            parser_models::ast::TypeKind::Array(array_type) => self.collect_type(&mut array_type.of_type),
            parser_models::ast::TypeKind::Reference(reference_type) => self.collect_type(&mut reference_type.inner),
            parser_models::ast::TypeKind::Pointer(soul_type) => self.collect_type(soul_type),
            parser_models::ast::TypeKind::Optional(soul_type) => self.collect_type(soul_type),
        }
    }
} 