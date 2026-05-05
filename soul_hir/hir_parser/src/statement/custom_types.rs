use hir::{CustomTypeId, Field, HirType, Struct};
use soul_utils::{Ident, Span, soul_error_internal};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(crate) fn lower_struct(&mut self, object: &ast::Struct) {
        let Some(scope) = self.scopes.last() else {
            self.log_error(soul_error_internal!(
                format!("self.scopes.last() not found"),
                Some(object.name.span)
            ));
            return;
        };

        let struct_id = match scope.custom_types.get(object.name.as_str()) {
            Some(CustomTypeId::Struct(val)) => *val,
            _ => {
                self.log_error(soul_error_internal!(
                    format!("{:?} not found", object.name.as_str()),
                    Some(object.name.span)
                ));
                return;
            }
        };

        let mut fields = vec![];
        for field in &object.fields {
            let ty = self.lower_type(&field.ty, field.name.span);
            let id = self.id_generator.alloc_field();

            let hir_field = hir::Field {
                id,
                ty,
                struct_id,
                name: field.name.clone(),
            };

            fields.push(hir_field.clone());
            self.tree.nodes.fields.insert(id, hir_field);
        }

        match self.tree.info.types.id_to_struct_mut(struct_id) {
            Some(obj) => obj.fields = fields,
            None => (),
        }
    }

    pub(crate) fn lower_internal_structs(&mut self) {
        let struct_id = self.tree.info.types.alloc_struct();
        let name = Ident::new("___Array".to_string(), Span::default(self.root_id));

        let none_type = self.add_type(HirType::none_type()).to_lazy();
        let ptr_type = self.add_type(HirType::pointer_type(none_type)).to_lazy();
        let len_type = self.add_type(HirType::index_type());
        let fields = vec![
            Field {
                struct_id,
                id: self.id_generator.alloc_field(),
                name: Ident::new("ptr".to_string(), Span::error()),
                ty: ptr_type,
            },
            Field {
                struct_id,
                id: self.id_generator.alloc_field(),
                name: Ident::new("len".to_string(), Span::error()),
                ty: len_type.to_lazy(),
            },
        ];

        self.tree.info.types.array_struct = struct_id;
        // to insure struct is in compiler
        self.add_type(
            HirType::new(hir::HirTypeKind::CustomType(CustomTypeId::Struct(
                struct_id,
            )))
            .apply_generics(vec![len_type]),
        );
        self.insert_struct(struct_id, Struct { name, fields });
    }
}
