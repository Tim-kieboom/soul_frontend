use ast::ast::SoulType;

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_type(&mut self, ty: &mut SoulType) {
        
        match &mut ty.kind {
            ast::ast::TypeKind::None => (),
            ast::ast::TypeKind::Type => (),
            ast::ast::TypeKind::Primitive(_) => (),
            ast::ast::TypeKind::Array(array_type) => self.collect_type(&mut array_type.of_type),
            ast::ast::TypeKind::Reference(reference_type) => self.collect_type(&mut reference_type.inner),
            ast::ast::TypeKind::Pointer(soul_type) => self.collect_type(soul_type),
            ast::ast::TypeKind::Optional(soul_type) => self.collect_type(soul_type),
        }
    }
} 