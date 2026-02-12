use ast::SoulType;

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_type(&mut self, ty: &mut SoulType) {
        
        match &mut ty.kind {
            ast::TypeKind::None => (),
            ast::TypeKind::Type => (),
            ast::TypeKind::Primitive(_) => (),
            ast::TypeKind::Array(array_type) => self.collect_type(&mut array_type.of_type),
            ast::TypeKind::Reference(reference_type) => self.collect_type(&mut reference_type.inner),
            ast::TypeKind::Pointer(soul_type) => self.collect_type(soul_type),
            ast::TypeKind::Optional(soul_type) => self.collect_type(soul_type),
        }
    }
} 