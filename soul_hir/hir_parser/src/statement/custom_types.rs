use hir::{CustomTypeId, Field, HirType, HirTypeKind, Struct, Union, UnionVariant};
use soul_utils::{Ident, Span, soul_error_internal, span::ItemMetaData};

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

    pub(crate) fn add_union(&mut self, object: &ast::Union) {
        let name = object.name.clone();

        let mut generics = vec![];
        for generic in &object.generics {
            let id = self.insert_generic(generic.name.to_string());
            generics.push(id);
        }

        let union_id = self.tree.info.types.alloc_union();
        let internal_struct_id = self.tree.info.types.alloc_struct();

        self.insert_union(
            union_id,
            Union {
                name,
                variants: vec![],
                internal_struct: internal_struct_id,
            },
        );

        let internal_name = Ident::new(
            format!("___Union_{}", object.name.as_str()),
            object.name.span,
        );
        self.insert_struct(
            internal_struct_id,
            Struct {
                name: internal_name,
                fields: vec![],
            },
        );

        let union_type = HirType::new(HirTypeKind::CustomType(CustomTypeId::Union(union_id)));
        self.add_type(union_type);
    }

    pub(crate) fn lower_union(&mut self, object: &ast::Union) {
        let Some(scope) = self.scopes.last() else {
            self.log_error(soul_error_internal!(
                format!("self.scopes.last() not found"),
                Some(object.name.span)
            ));
            return;
        };

        let union_id = match scope.custom_types.get(object.name.as_str()) {
            Some(CustomTypeId::Union(val)) => *val,
            _ => {
                self.log_error(soul_error_internal!(
                    format!("union '{}' not found", object.name.as_str()),
                    Some(object.name.span)
                ));
                return;
            }
        };

        let union_def = match self.tree.info.types.id_to_union_mut(union_id) {
            Some(val) => val,
            None => return,
        };
        let internal_struct_id = union_def.internal_struct;

        let index_type = self.add_type(HirType::index_type());

        let tag_field_id = self.id_generator.alloc_field();
        let tag_field = Field {
            struct_id: internal_struct_id,
            id: tag_field_id,
            name: Ident::new("__tag".to_string(), object.name.span),
            ty: index_type.to_lazy(),
        };
        self.tree
            .nodes
            .fields
            .insert(tag_field_id, tag_field.clone());

        let mut hir_variants = vec![];
        let mut internal_fields = vec![tag_field];
        for (_i, variant) in object.variants.iter().enumerate() {
            let variant_field_id = self.id_generator.alloc_field();
            let variant_ty = self.lower_type(&variant.ty, variant.name.span);

            let variant_field = Field {
                struct_id: internal_struct_id,
                id: variant_field_id,
                name: variant.name.clone(),
                ty: variant_ty,
            };
            self.tree
                .nodes
                .fields
                .insert(variant_field_id, variant_field.clone());
            internal_fields.push(variant_field);

            let union_field_id = self.tree.info.types.alloc_union_field();
            hir_variants.push(UnionVariant {
                id: union_field_id,
                name: variant.name.clone(),
                ty: variant_ty,
            });
        }

        if let Some(union_def) = self.tree.info.types.id_to_union_mut(union_id) {
            union_def.variants = hir_variants;
        }

        if let Some(internal_struct) = self.tree.info.types.id_to_struct_mut(internal_struct_id) {
            internal_struct.fields = internal_fields;
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

    pub(crate) fn lower_trait(&mut self, object: &ast::Trait) {
        if object.id.is_none() {
            self.log_error(soul_error_internal!(
                format!("Trait '{}' has no id", object.name.as_str()),
                Some(object.name.span)
            ));
            return;
        };

        let trait_id = self.tree.info.types.alloc_trait();
        let mut method_ids = vec![];

        for method in &object.methods {
            match method.signature.node.id {
                Some(fid) => method_ids.push(fid),
                None => {
                    self.log_error(soul_error_internal!(
                        format!(
                            "Trait method '{}' has no id",
                            method.signature.node.name.as_str()
                        ),
                        Some(method.signature.span)
                    ));
                }
            }
        }

        self.tree.info.types.insert_trait(
            trait_id,
            hir::Trait {
                name: object.name.clone(),
                methods: method_ids,
            },
        );

        let Some(scope) = self.scopes.last_mut() else {
            return;
        };
        scope
            .custom_types
            .insert(object.name.to_string(), CustomTypeId::Trait(trait_id));
    }

    pub(crate) fn lower_impl_block(
        &mut self,
        object: &ast::ImplBlock,
        meta_data: &ItemMetaData,
        span: Span,
    ) {
        for method in &object.methodes {
            let function_id = self.lower_function(method);
            let kind = hir::GlobalKind::Function(function_id);
            let id = self.alloc_statement(meta_data, span);
            self.insert_global(self.current.module, hir::Global::new(kind, id));
        }
    }
}
