use hir::{
    BlockId, ExpressionId, Field, FieldId, GenericId, HirType, HirTypeKind, LocalId, RefTypeId, StatementId, StructId, TypeId, TypesMap, UnifyResult
};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult}, ids::{FunctionId, IdAlloc}, soul_names::{PrimitiveTypes, TypeModifier}, span::Span, vec_map::{VecMap, VecMapIndex}
};

use crate::{HirTypedContext};

impl<'a> HirTypedContext<'a> {
    pub(crate) fn type_block(&mut self, id: BlockId, ty: TypeId) {
        self.type_table.blocks.insert(id, ty);
    }

    pub(crate) fn type_function(&mut self, id: FunctionId, ty: TypeId) {
        self.type_table.functions.insert(id, ty);
    }

    pub(crate) fn type_statement(&mut self, id: StatementId, ty: TypeId) {
        self.type_table.statements.insert(id, ty);
    }

    pub(crate) fn type_expression(&mut self, id: ExpressionId, ty: TypeId) {
        self.type_table.expressions.insert(id, ty);
    }

    pub(crate) fn add_type(&mut self, ty: HirType) -> TypeId {
        self.type_table.types.insert(ty)
    }

    pub(crate) fn id_to_type(&self, ty: TypeId) -> &HirType {
        self.type_table
            .types
            .id_to_type(ty)
            .expect("TypeId should always have a type")
    }

    pub(crate) fn ref_to_type(&self, ty: RefTypeId) -> &HirType {
        self.type_table
            .types
            .ref_to_type(ty)
            .expect("RefTypeId should always have a type")
    }

    pub(crate) fn ref_to_id(&self, ty: RefTypeId) -> TypeId {
        self.type_table
            .types
            .ref_to_id(ty)
            .expect("RefTypeId should always have a TypeId")
    }

    pub(crate) fn resolve_type_strict(&mut self, ty: TypeId, span: Span) -> Option<TypeId> {
        match self
            .infer_table
            .resolve_type_strict(&mut self.type_table.types, ty, span)
        {
            Ok(val) => Some(val),
            Err(err) => {
                self.log_error(err);
                None
            }
        }
    }

    pub(crate) fn resolve_type_lazy(&mut self, ty: TypeId, span: Span) -> TypeId {
        match self
            .infer_table
            .resolve_type_lazy(&mut self.type_table.types, ty, span)
        {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                TypeId::error()
            }
        }
    }

    pub(crate) fn unify(
        &mut self,
        value: ExpressionId,
        expect: TypeId,
        got: TypeId,
        span: Span,
    ) -> bool {
        match self
            .infer_table
            .unify_type_type(&mut self.type_table.types, expect, got, span)
        {
            Ok(UnifyResult::Ok) => true,
            Ok(UnifyResult::NeedsAutoCopy) => {
                self.add_autocopy(value);
                true
            }
            Err(err) => {
                self.log_error(err);
                self.posion_expression(value);
                false
            }
        }
    }

    pub(crate) fn type_field(&mut self, field: &Field, base: RefTypeId, index: usize) {
        let info = crate::FieldInfo { 
            base_type: base, 
            index,
            field_type: field.ty, 
        };
        self.type_table.fields.insert(field.id, info);
    }

    pub(crate) fn type_local(
        &mut self,
        id: LocalId,
        type_id: TypeId,
        modifier: TypeModifier,
        span: Span,
    ) -> TypeId {
        let resolved = self.resolve_untyped_primitive(type_id, span);
        let local_type_id = match resolved {
            Some(mut ty) => {
                ty.modifier = Some(modifier);
                self.add_type(ty)
            }
            None => {
                let mut ty = self.id_to_type(type_id).clone();
                ty.modifier = Some(modifier);
                self.add_type(ty)
            }
        };

        debug_assert_eq!(self.id_to_type(local_type_id).modifier, Some(modifier),);

        self.type_table.locals.insert(id, local_type_id);
        local_type_id
    }

    pub(crate) fn get_priority_type(&mut self, left: TypeId, right: TypeId) -> TypeId {
        self.infer_table
            .get_priority_type(&self.type_table.types, left, right)
    }

    pub(crate) fn resolve_all_types(&mut self) {
        macro_rules! resolve {
            ($field:ident, $id:ty) => {
                let cap = self.type_table.$field.cap();
                for i in 1..cap {
                    let type_id = match self.type_table.$field.raw_index(i) {
                        Some(val) => *val,
                        None => continue,
                    };
                    let index = <$id>::new_index(i);

                    let span = match self.hir.spans.$field.get(index).copied() {
                        Some(val) => val,
                        None => Span::default(),
                    };
                    let resolved = self
                        .resolve_type_strict(type_id, span)
                        .unwrap_or(TypeId::error());
                    self.type_table.$field.insert(index, resolved);
                }
            };
        }
        resolve!(locals, LocalId);
        resolve!(blocks, BlockId);
        resolve!(functions, FunctionId);
        resolve!(statements, StatementId);
        resolve!(expressions, ExpressionId);
    }

    pub(crate) fn finalize_types(&mut self) {
        let mut new_types = TypesMap::new();
        let structs = self.type_table.types.get_structs().clone();
        new_types.set_structs(structs);
        
        let mut remap = VecMap::<TypeId, TypeId>::new();

        let ids: Vec<_> = self.collect_root_types();
        for old_id in ids {
            let new_id = match self.finalize_type(old_id, &mut new_types, &mut remap) {
                Ok(val) => val,
                Err(err) => {
                    self.log_error(err);
                    return;
                }
            };

            if let Some(ref_id) = self.type_table.types.id_to_ref(old_id) {
                new_types.force_insert_ref(ref_id, new_id);
            }
            remap.insert(old_id, new_id);
        }

        self.type_table.types = new_types;
        self.type_table.remap_types(&remap);

        self.type_table.none_type = self
            .type_table
            .types
            .type_to_id(&HirType::none_type())
            .expect("should have none type in table");
    }

    pub(crate) fn get_field_access(
        &mut self, 
        object: TypeId,
        field: &str,
        span: Span,
    ) -> Option<FieldId> {
        let object_type = self.resolve_type_strict(object, span)?;
        match &self.id_to_type(object_type).kind {
            HirTypeKind::Struct(struct_id) => self.get_struct_field(*struct_id, field, span),
            other => {
                self.log_error(SoulError::new(
                    format!("typekind '{}' can not do field access", other.display_variant()), 
                    SoulErrorKind::InvalidType, 
                    Some(span)
                ));
                return None
            }
        }
    }

    fn get_struct_field(
        &mut self,
        object: StructId,
        field_ident: &str,
        span: Span
    ) -> Option<FieldId> {
        let object_struct = self.type_table.types.id_to_struct(object)?;
        let field = match object_struct.fields.iter().find(|field| field.name == field_ident) {
            Some(val) => val,
            None => {
                self.log_error(SoulError::new(format!("field '{}' not found", field_ident), SoulErrorKind::FieldNotFound, Some(span)));
                return None
            }
        };

        Some(field.id)
    }

    fn collect_root_types(&self) -> Vec<TypeId> {
        let mut roots = Vec::new();

        let gen_ref_types = self
            .type_table
            .generic_defines
            .entries()
            .flat_map(|(_, types)| types.entries());

        roots.extend(self.type_table.locals.values().copied());
        roots.extend(self.type_table.blocks.values().copied());
        roots.extend(self.type_table.functions.values().copied());
        roots.extend(self.type_table.statements.values().copied());
        roots.extend(self.type_table.expressions.values().copied());
        roots.extend(gen_ref_types.map(|ref_type| self.ref_to_id(ref_type)));
        roots.push(self.type_table.none_type);

        roots
    }

    fn finalize_type(
        &mut self,
        ty: TypeId,
        new_types: &mut TypesMap,
        map: &mut VecMap<TypeId, TypeId>,
    ) -> SoulResult<TypeId> {
        if let Some(&id) = map.get(ty) {
            return Ok(id);
        }

        let resolved = self
            .resolve_type_strict(ty, Span::default())
            .unwrap_or(TypeId::error());
        let typed = match self.resolve_untyped_primitive(resolved, Span::default_const()) {
            Some(val) => self.add_type(val),
            None => resolved,
        };
        let hir_ty = self
            .type_table
            .types
            .id_to_type(typed)
            .expect("should have id");

        debug_assert!(
            !hir_ty.is_infer_type(),
            "InferType leaked into final type graph"
        );

        let modifier = hir_ty.modifier;
        let mut new_ty = match &hir_ty.kind {
            HirTypeKind::Struct(id) => {
                HirType::new(HirTypeKind::Struct(*id))
            }
            HirTypeKind::Optional(id) => {
                let id = self.finalize_type(*id, new_types, map)?;
                HirType::new(HirTypeKind::Optional(id))
            }

            HirTypeKind::Pointer(id) => {
                let id = self.finalize_type(*id, new_types, map)?;
                HirType::new(HirTypeKind::Pointer(id))
            }

            HirTypeKind::Ref { of_type, mutable } => {
                let of_type = *of_type;
                let mutable = *mutable;
                let id = self.finalize_type(of_type, new_types, map)?;
                HirType::new(HirTypeKind::Ref {
                    of_type: id,
                    mutable,
                })
            }

            HirTypeKind::Array { element, kind } => {
                let kind = *kind;
                let element = *element;
                let el = self.finalize_type(element, new_types, map)?;
                HirType::new(HirTypeKind::Array { element: el, kind })
            }
            HirTypeKind::None
            | HirTypeKind::Type
            | HirTypeKind::Error
            | HirTypeKind::Generic(_)
            | HirTypeKind::Primitive(_) => hir_ty.clone(),

            HirTypeKind::InferType(_, _) => unreachable!("resolve_type must eliminate InferType"),
        };

        new_ty.modifier = modifier;
        let new_id = new_types.insert(new_ty);
        map.insert(ty, new_id);

        if let Some(ref_id) = self.type_table.types.id_to_ref(ty) {
            new_types.force_insert_ref(ref_id, new_id);
        }

        Ok(new_id)
    }

    pub(crate) fn resolve_untyped_primitive(
        &mut self,
        base_type: TypeId,
        span: Span,
    ) -> Option<HirType> {
        let base_type = self.id_to_type(base_type);
        let modifier = base_type.modifier;
        let prim = match &base_type.kind {
            HirTypeKind::Primitive(val) => val,
            HirTypeKind::Pointer(type_id) => {
                let id = *type_id;
                let ty = match self.resolve_untyped_primitive(id, span) {
                    Some(val) => self.add_type(val),
                    None => id,
                };
                return Some(HirType {
                    kind: HirTypeKind::Pointer(ty),
                    modifier,
                    generics: vec![],
                });
            }
            HirTypeKind::Optional(type_id) => {
                let id = *type_id;
                let ty = match self.resolve_untyped_primitive(id, span) {
                    Some(val) => self.add_type(val),
                    None => id,
                };
                return Some(HirType {
                    kind: HirTypeKind::Optional(ty),
                    modifier,
                    generics: vec![],
                });
            }
            HirTypeKind::Array { element, kind } => {
                let kind = *kind;
                let id = *element;
                let element = match self.resolve_untyped_primitive(id, span) {
                    Some(val) => self.add_type(val),
                    None => id,
                };
                return Some(HirType {
                    kind: HirTypeKind::Array { element, kind },
                    modifier,
                    generics: vec![],
                });
            }
            HirTypeKind::Ref { of_type, mutable } => {
                let mutable = *mutable;
                let id = *of_type;
                let of_type = match self.resolve_untyped_primitive(id, span) {
                    Some(val) => self.add_type(val),
                    None => id,
                };
                return Some(HirType {
                    kind: HirTypeKind::Ref { of_type, mutable },
                    modifier,
                    generics: vec![],
                });
            }
            _ => return None,
        };

        let typed = match prim {
            PrimitiveTypes::UntypedInt => PrimitiveTypes::Int,
            PrimitiveTypes::UntypedUint => PrimitiveTypes::Int,
            PrimitiveTypes::UntypedFloat => PrimitiveTypes::Float32,
            _ => return None,
        };

        Some(HirType {
            kind: HirTypeKind::Primitive(typed),
            modifier,
            generics: base_type.generics.clone(),
        })
    }

    pub(crate) fn resolve_generic(
        &mut self,
        generic_defines: &VecMap<GenericId, TypeId>,
        ty: TypeId,
    ) -> TypeId {
        let hir_type = self.id_to_type(ty);
        match hir_type.kind {
            HirTypeKind::Generic(generic_id) => match generic_defines.get(generic_id) {
                Some(ty) => *ty,
                None => TypeId::error(),
            },
            _ => return ty,
        }
    }

    fn posion_expression(&mut self, value: ExpressionId) {
        self.type_expression(value, TypeId::error());
    }
}
