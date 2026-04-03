use ast::Stub;
use hir::{GenericId, HirType, HirTypeKind, InferTypesMap, LazyTypeId, StructId, TypeId, TypesMap};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    ids::IdAlloc,
    soul_error_internal,
    soul_names::{PrimitiveTypes, TypeModifier},
    span::Span,
};

use crate::{HirContext, Scope};
const CHAR: HirType = HirType::new(hir::HirTypeKind::Primitive(PrimitiveTypes::Char));

impl<'a> HirContext<'a> {
    pub(crate) fn lower_type(&mut self, ty: &ast::SoulType) -> hir::LazyTypeId {
        match Self::convert_type(
            ty,
            &self.scopes,
            &vec![],
            &mut self.tree.info.types,
            &mut self.tree.info.infers,
        ) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                LazyTypeId::error()
            }
        }
    }

    pub(crate) fn type_from_array(&mut self, array: &ast::Array, span: Span) -> TypeId {
        if array.collection_type.is_some() {
            self.log_error(soul_error_internal!(
                "collection type in array is unstable",
                Some(span)
            ));
            return TypeId::error();
        }

        let kind = ast::ArrayKind::StackArray(array.values.len() as u64);
        let element = match &array.element_type {
            Some(val) => self.lower_type(val),
            None => self.new_infer_type(vec![], None, span),
        };

        let array_ty = self.add_type(HirType::new(HirTypeKind::Array { element, kind }));
        array_ty
    }

    pub(crate) fn add_type(&mut self, ty: HirType) -> TypeId {
        self.tree.info.types.insert_type(ty)
    }

    /// this function is needed is the borrow checker does not validate self.lower_type
    pub(crate) fn convert_type(
        ty: &ast::SoulType,
        scopes: &Vec<Scope>,
        call_generics: &Vec<(String, TypeId)>,
        types: &mut TypesMap,
        infers: &mut InferTypesMap,
    ) -> SoulResult<hir::LazyTypeId> {
        let mut generics = vec![];
        let modifier = ty.modifier;
        let kind = match &ty.kind {
            ast::TypeKind::None => HirTypeKind::None,
            ast::TypeKind::Type => HirTypeKind::Type,
            ast::TypeKind::Stub(Stub {
                name,
                generics: stub_generics,
            }) => {
                for generic in stub_generics {
                    let ty = Self::convert_type(generic, scopes, call_generics, types, infers)?;

                    match ty {
                        LazyTypeId::Known(type_id) => generics.push(type_id),
                        LazyTypeId::Infer(_) => {
                            return Err(SoulError::new(
                                "type should be known at this time",
                                SoulErrorKind::TypeInferenceError,
                                Some(generic.span),
                            ));
                        }
                    }
                }

                resolve_stub(scopes, types, call_generics, name).ok_or(SoulError::new(
                    format!("type '{}' not found", name),
                    soul_utils::error::SoulErrorKind::TypeNotFound,
                    Some(ty.span),
                ))?
            }
            ast::TypeKind::Pointer(inner) => HirTypeKind::Pointer(Self::convert_type(
                inner,
                scopes,
                call_generics,
                types,
                infers,
            )?),
            ast::TypeKind::Optional(inner) => HirTypeKind::Optional(Self::convert_type(
                inner,
                scopes,
                call_generics,
                types,
                infers,
            )?),
            ast::TypeKind::Array(array) => HirTypeKind::Array {
                element: Self::convert_type(&array.of_type, scopes, call_generics, types, infers)?,
                kind: array.kind,
            },
            ast::TypeKind::Reference(reference) => HirTypeKind::Ref {
                of_type: Self::convert_type(
                    &reference.inner,
                    scopes,
                    call_generics,
                    types,
                    infers,
                )?,
                mutable: reference.mutable,
            },
            ast::TypeKind::Primitive(prim) => HirTypeKind::Primitive(*prim),
        };

        let ty = types.insert_type(HirType {
            kind,
            modifier,
            generics,
        });
        Ok(LazyTypeId::Known(ty))
    }

    pub(crate) fn insert_generic(&mut self, name: String) -> GenericId {
        let id = self.tree.info.types.insert_generic(name.clone());
        self.scopes
            .last_mut()
            .expect("should have scope")
            .generics
            .insert(name, id);
        
        id
    }

    pub(crate) fn insert_struct(&mut self, id: StructId, obj: hir::Struct) {
        let name = obj.name.to_string();
        self.tree.info.types.insert_struct(id, obj);
        self.scopes
            .last_mut()
            .expect("should have scope")
            .created_type
            .insert(name, hir::CreatedTypes::Struct(id));
    }

    pub(crate) fn new_infer_type(
        &mut self,
        generics: Vec<TypeId>,
        modifier: Option<TypeModifier>,
        span: Span,
    ) -> LazyTypeId {
        LazyTypeId::Infer(self.tree.info.infers.insert_infer(generics, modifier, span))
    }

    pub(crate) fn new_null_infer(&mut self, span: Span) -> LazyTypeId {
        let infer = self.new_infer_type(vec![], None, span);
        LazyTypeId::Known(self.add_type(HirType {
            kind: HirTypeKind::Optional(infer),
            modifier: None,
            generics: vec![],
        }))
    }

    pub(crate) fn type_from_literal(&mut self, literal: &ast::Literal) -> TypeId {
        let ty = match literal.get_literal_type().to_internal_primitive_type() {
            ast::TypeResult::Primitive(val) => HirTypeKind::Primitive(val),
            ast::TypeResult::Str => HirTypeKind::Array {
                element: LazyTypeId::Known(self.add_type(CHAR)),
                kind: ast::ArrayKind::ConstSlice,
            },
        };

        self.add_type(HirType::new(ty))
    }
}

enum GenericKind {
    Generic(GenericId),
    Resolved(TypeId),
}

fn resolve_stub(
    scopes: &Vec<Scope>,
    types: &TypesMap,
    call_generics: &Vec<(String, TypeId)>,
    name: &str,
) -> Option<HirTypeKind> {
    if let Some(ty) = find_created_type(scopes, name) {
        return Some(ty);
    }

    let id = find_generic(scopes, call_generics, name)?;
    Some(match id {
        GenericKind::Generic(generic) => HirTypeKind::Generic(generic),
        GenericKind::Resolved(ref_type) => types.id_to_type(ref_type)?.kind,
    })
}

fn find_created_type(scopes: &Vec<Scope>, name: &str) -> Option<HirTypeKind> {
    for store in scopes.iter().rev() {
        if let Some(ty) = store.created_type.get(name).copied() {
            return Some(ty.to_hir_kind());
        }
    }

    None
}

fn find_generic(
    scopes: &Vec<Scope>,
    call_generics: &Vec<(String, TypeId)>,
    name: &str,
) -> Option<GenericKind> {
    for (generic_name, ref_type) in call_generics {
        if generic_name == name {
            return Some(GenericKind::Resolved(*ref_type));
        }
    }

    for store in scopes.iter().rev() {
        if let Some(id) = store.generics.get(name).copied() {
            return Some(GenericKind::Generic(id));
        }
    }

    None
}
