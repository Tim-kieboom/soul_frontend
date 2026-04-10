use hir::{FieldId, HirTypeKind, LazyTypeId, TypeId};
use soul_utils::{
    ids::IdAlloc, vec_map::{VecMap, VecMapIndex}
};
use typed_hir::{FieldInfo, ThirType, ThirTypeKind, ThirTypesMap, TypedHir};

use crate::{TypedHirContext, infer_table::InferBinding};

impl<'a> TypedHirContext<'a> {
    pub(crate) fn finalize(mut self) -> TypedHir {
        self.finalize_infers();
        use std::mem::take;

        let expressions = take(&mut self.expressions);
        let statements = take(&mut self.statements);
        let sizeofs = take(&mut self.sizeofs);
        let places = take(&mut self.places);
        let locals = take(&mut self.locals);
        let blocks = take(&mut self.blocks);

        let table = typed_hir::TypeTable {
            none_type: self.none_type,
            bool_type: self.bool_type,
            u32_type: self.u32_type,

            expressions: self.resolve_map(expressions),
            sizeofs: self.resolve_map(sizeofs),
            statements: self.resolve_map(statements),
            functions: take(&mut self.functions),
            places: self.resolve_map(places),
            locals: self.resolve_map(locals),
            blocks: self.resolve_map(blocks),

            fields: self.resolve_fields(),
            place_fields: take(&mut self.place_fields),

            auto_copy: take(&mut self.auto_copys),
            generic_instantiations: take(&mut self.generic_defines),
        };

        // 3. convert TypesMap → ThirTypesMap
        let types_map = self.lower_types_map();

        TypedHir {
            types_map,
            types_table: table,
        }
    }

    fn resolve_fields(&mut self) -> VecMap<FieldId, FieldInfo> {
        let mut fields = VecMap::with_capacity(self.fields.len());

        let mut temp = VecMap::new();
        std::mem::swap(&mut self.fields, &mut temp);

        for (id, field) in temp.entries() {
            fields.insert(
                id,
                FieldInfo {
                    base_type: field.base_type,
                    field_type: self.to_known(field.field_type),
                    field_index: field.field_index,
                },
            );
        }
        std::mem::swap(&mut temp, &mut self.fields);

        fields
    }

    fn lower_types_map(&mut self) -> ThirTypesMap {
        let mut out = ThirTypesMap::new(self.hir.info.types.array_struct);

        let keys = self.types.types_keys().collect::<Vec<_>>();
        for id in keys {
            let hir_ty = self.types.id_to_type(id).expect("should have key");
            let modifier = hir_ty.modifier;
            let generics = hir_ty.generics.clone();
            let kind = self.lower_type_kind(hir_ty.kind);
            let thir_ty = ThirType {
                kind,
                generics,
                modifier,
            };

            out.types.force_insert(id, thir_ty);
        }

        for (id, s) in self.hir.info.types.structs_entries() {
            let mut fields = Vec::with_capacity(s.fields.len());

            for field in &s.fields {
                let id = field.id;
                let ty = self.to_known(self.fields[id].field_type);

                fields.push(typed_hir::Field { id, ty })
            }

            let warning = "to do impl check resursive inclusion";

            out.structs.insert(
                id,
                typed_hir::Struct {
                    id,
                    fields,
                    name: s.name.to_string(),
                    packed: self.options.default_packed(),
                },
            );
        }

        out
    }

    fn lower_type_kind(&mut self, kind: HirTypeKind) -> ThirTypeKind {
        match kind {
            HirTypeKind::None => ThirTypeKind::None,
            HirTypeKind::Type => ThirTypeKind::Type,
            HirTypeKind::Primitive(p) => ThirTypeKind::Primitive(p),

            HirTypeKind::Pointer(t) => ThirTypeKind::Pointer(self.to_known(t)),
            HirTypeKind::Optional(t) => ThirTypeKind::Optional(self.to_known(t)),

            HirTypeKind::Ref { of_type, mutable } => ThirTypeKind::Ref {
                of_type: self.to_known(of_type),
                mutable,
            },

            HirTypeKind::Array { element, kind } => ThirTypeKind::Array {
                element: self.to_known(element),
                kind,
            },

            HirTypeKind::Struct(id) => ThirTypeKind::Struct(id),
            HirTypeKind::Generic(id) => ThirTypeKind::Generic(id),

            HirTypeKind::Error => ThirTypeKind::Error,
        }
    }

    fn to_known(&mut self, id: LazyTypeId) -> TypeId {
        match self
            .infer_table
            .resolve_type_strict(&mut self.types, id, None)
        {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                TypeId::error()
            }
        }
    }

    fn finalize_infers(&mut self) {
        let unbounds = self
            .infer_table
            .table
            .entries()
            .filter(|(_, value)| matches!(value, InferBinding::Unbound(_)))
            .map(|(key, _)| key)
            .collect::<Vec<_>>();

        for id in unbounds {
            self.infer_table
                .table
                .insert(id, InferBinding::Bound(TypeId::error()));
        }
    }

    fn resolve_map<K>(&mut self, map: VecMap<K, LazyTypeId>) -> VecMap<K, TypeId>
    where
        K: VecMapIndex + IdAlloc,
    {
        let mut new_map = map
            .into_entries()
            .map(|(k, lazy)| {
                let ty = self
                    .infer_table
                    .resolve_type_strict(&mut self.types, lazy, None)
                    .unwrap_or(TypeId::error());

                (k, ty)
            })
            .collect::<VecMap<K, TypeId>>();

        new_map.insert(K::error(), TypeId::error());
        new_map
    }
}
