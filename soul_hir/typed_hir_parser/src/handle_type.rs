use hir::{ExpressionId, FieldId, HirType, HirTypeKind, LazyTypeId, LocalId, StructId, TypeId};
use soul_utils::{
    error::{SoulError, SoulErrorKind}, soul_names::{PrimitiveTypes, TypeModifier}, span::Span
};

use crate::{TypedHirContext, infer_table::UnifyResult};

impl<'a> TypedHirContext<'a> {
    pub(crate) fn unify(
        &mut self,
        value: ExpressionId,
        expect: LazyTypeId,
        got: LazyTypeId,
        span: Span,
    ) -> bool {
        match self
            .infer_table
            .unify_type_type(&mut self.types, &self.infers, expect, got, span)
        {
            Ok(UnifyResult::Ok) => true,
            Ok(UnifyResult::NeedsAutoCopy) => {
                self.auto_copys.insert(value);
                true
            }
            Err(err) => {
                self.log_error(err);
                self.posion_expression(value);
                false
            }
        }
    }

    pub(crate) fn resolve_type_strict(&mut self, ty: LazyTypeId, span: Span) -> Option<TypeId> {
        if ty == LazyTypeId::error() {
            return None
        }
        
        match self
            .infer_table
            .resolve_type_strict(&mut self.types, ty, Some(span))
        {
            Ok(val) => Some(val),
            Err(err) => {
                self.log_error(err);
                None
            }
        }
    }

    pub(crate) fn resolve_type_lazy(&mut self, ty: LazyTypeId, span: Span) -> LazyTypeId {
        if ty == LazyTypeId::error() {
            return ty
        }
        
        match self
            .infer_table
            .resolve_type_lazy(&mut self.types, &self.infers, ty, span)
        {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                LazyTypeId::error()
            }
        }
    }

    pub(crate) fn get_field_access(
        &mut self,
        object: LazyTypeId,
        field: &str,
        span: Span,
    ) -> Option<FieldId> {
        let object_type = self.resolve_type_strict(object, span)?;
        match &self.id_to_type(object_type).kind {
            HirTypeKind::Struct(struct_id) => self.get_struct_field(*struct_id, field, span),
            HirTypeKind::Ref { of_type, .. } => {
                self.get_field_access(*of_type, field, span)
            }
            HirTypeKind::Array { .. } => {
                let struct_id = self.hir.info.types.array_struct;
                self.get_struct_field(struct_id, field, span)
            }
            other => {
                self.log_error(SoulError::new(
                    format!(
                        "typekind '{}' can not do field access",
                        other.display_variant()
                    ),
                    SoulErrorKind::InvalidType,
                    Some(span),
                ));
                return None;
            }
        }
    }

    pub(crate) fn type_local(
        &mut self,
        id: LocalId,
        type_id: LazyTypeId,
        modifier: TypeModifier,
        span: Span,
    ) -> LazyTypeId {
        let resolved = self.resolve_untyped_primitive(type_id, span);
        let local_type_id = match resolved {
            Some(mut ty) => {
                ty.modifier = Some(modifier);
                self.add_type(ty).to_lazy()
            }
            None => self.lazy_id_insure_modifier(type_id, Some(modifier)),
        };

        match local_type_id {
            LazyTypeId::Known(type_id) => {
                debug_assert_eq!(self.id_to_type(type_id).modifier, Some(modifier))
            }
            LazyTypeId::Infer(infer) => {
                debug_assert_eq!(self.id_to_infer(infer).modifier, Some(modifier))
            }
        }

        self.locals.insert(id, local_type_id);
        local_type_id
    }

    fn resolve_untyped_primitive(&mut self, base_type: LazyTypeId, span: Span) -> Option<HirType> {
        let base_type = match base_type {
            LazyTypeId::Known(val) => val,
            LazyTypeId::Infer(_) => return None,
        };

        let base_type = self.id_to_type(base_type);
        let modifier = base_type.modifier;
        let prim = match &base_type.kind {
            HirTypeKind::Primitive(val) => val,
            HirTypeKind::Pointer(type_id) => {
                let id = *type_id;
                let ty = match self.resolve_untyped_primitive(id, span) {
                    Some(val) => self.add_type(val).to_lazy(),
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
                    Some(val) => self.add_type(val).to_lazy(),
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
                    Some(val) => self.add_type(val).to_lazy(),
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
                    Some(val) => self.add_type(val).to_lazy(),
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

    fn get_struct_field(
        &mut self,
        object: StructId,
        field_ident: &str,
        span: Span,
    ) -> Option<FieldId> {
        let object_struct = self.types.id_to_struct(object)?;
        let field = match object_struct
            .fields
            .iter()
            .find(|field| field.name == field_ident)
        {
            Some(val) => val,
            None => {
                self.log_error(SoulError::new(
                    format!("field '{}' not found", field_ident),
                    SoulErrorKind::FieldNotFound,
                    Some(span),
                ));
                return None;
            }
        };

        Some(field.id)
    }

    pub(crate) fn is_mutable_or_modifier_none(&self, ty: LazyTypeId) -> bool {
        if ty == LazyTypeId::error() {
            return true
        } 
        
        match ty {
            LazyTypeId::Known(type_id) => {
                let ty = self.id_to_type(type_id);
                ty.is_mutable() || ty.is_modifier_none()
            },
            LazyTypeId::Infer(infer_type_id) => {
                let ty = self.id_to_infer(infer_type_id);
                ty.is_mutable() || ty.is_modifier_none()
            },
        }
    }

    fn posion_expression(&mut self, value: ExpressionId) {
        self.type_expression(value, LazyTypeId::error());
    }
}
