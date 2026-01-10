use parser_models::ast::SoulType;

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_type(&mut self, ty: &mut SoulType) {
        
        match &mut ty.kind {
            parser_models::ast::TypeKind::None => (),
            parser_models::ast::TypeKind::Type => (),
            parser_models::ast::TypeKind::Stub { .. } => todo!("impl stub"),
            parser_models::ast::TypeKind::Primitive(_) => (),
            parser_models::ast::TypeKind::Array(array_type) => self.collect_type(&mut array_type.of_type),
            parser_models::ast::TypeKind::Tuple(soul_types) => {
                for ty in soul_types {
                    self.collect_type(ty);
                }
            }
            parser_models::ast::TypeKind::NamedTuple(items) => {
                for (_name, ty, id) in items {
                    self.collect_type(ty);
                    *id = Some(self.alloc_id());
                }
            }
            parser_models::ast::TypeKind::Generic { node_id, kind:_ } => *node_id = Some(self.alloc_id()),
            parser_models::ast::TypeKind::Reference(reference_type) => self.collect_type(&mut reference_type.inner),
            parser_models::ast::TypeKind::Pointer(soul_type) => self.collect_type(soul_type),
            parser_models::ast::TypeKind::Optional(soul_type) => self.collect_type(soul_type),
        }
    }
} 