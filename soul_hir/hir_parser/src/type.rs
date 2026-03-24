use ast::{Stub};
use hir::{GenericId, HirType, HirTypeKind, RefTypeId, TypeId, TypesMap};
use soul_utils::{
    error::{SoulError, SoulResult},
    ids::IdAlloc,
    soul_error_internal,
    soul_names::{PrimitiveTypes, TypeModifier},
    span::Span,
};

use crate::{HirContext, Scope};

const CHAR: HirType = HirType::new(hir::HirTypeKind::Primitive(PrimitiveTypes::Char));

impl<'a> HirContext<'a> {
    /// this function is needed is the borrow checker does not validate self.lower_type
    pub(crate) fn convert_type(
        ty: &ast::SoulType,
        scopes: &Vec<Scope>,
        call_generics: &Vec<(String, RefTypeId)>,
        types: &mut TypesMap,
    ) -> SoulResult<hir::TypeId> {
        let mut generics = vec![];
        let modifier = ty.modifier; 
        let kind = match &ty.kind {
            ast::TypeKind::None => HirTypeKind::None,
            ast::TypeKind::Type => HirTypeKind::Type,
            ast::TypeKind::Stub(Stub{ name, generics: stub_generics }) => {
                for generic in stub_generics {
                    generics.push(
                        Self::convert_type(generic, scopes, call_generics, types)?
                    )
                }
                
                resolve_stub(scopes, types, call_generics, name).ok_or(SoulError::new(
                    format!("type '{}' not found", name),
                    soul_utils::error::SoulErrorKind::TypeNotFound,
                    Some(ty.span),
                ))?
            }
            ast::TypeKind::Pointer(inner) => {
                HirTypeKind::Pointer(Self::convert_type(inner, scopes, call_generics, types)?)
            }
            ast::TypeKind::Optional(inner) => {
                HirTypeKind::Optional(Self::convert_type(inner, scopes, call_generics, types)?)
            }
            ast::TypeKind::Array(array) => HirTypeKind::Array {
                element: Self::convert_type(&array.of_type, scopes, call_generics, types)?,
                kind: array.kind,
            },
            ast::TypeKind::Reference(reference) => HirTypeKind::Ref {
                of_type: Self::convert_type(&reference.inner, scopes, call_generics, types)?,
                mutable: reference.mutable,
            },
            ast::TypeKind::Primitive(prim) => HirTypeKind::Primitive(*prim),
        };

        Ok(types.insert(HirType{kind, modifier, generics}))
    }

    pub(crate) fn lower_type(&mut self, ty: &ast::SoulType) -> hir::TypeId {
        match Self::convert_type(ty, &self.scopes, &vec![], &mut self.hir.types) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                TypeId::error()
            }
        }
    }

    pub(crate) fn type_from_literal(&mut self, literal: &ast::Literal) -> TypeId {
        let ty = match literal.get_literal_type().to_internal_primitive_type() {
            ast::TypeResult::Primitive(val) => HirTypeKind::Primitive(val),
            ast::TypeResult::Str => HirTypeKind::Array {
                element: self.add_type(CHAR),
                kind: ast::ArrayKind::ConstSlice,
            },
        };

        self.add_type(HirType::new(ty))
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
            None => self.new_infer_type(span),
        };

        let array_ty = self.add_type(HirType::new(HirTypeKind::Array { element, kind }));
        array_ty
    }

    pub(crate) fn new_infer_type(&mut self, span: Span) -> TypeId {
        self.hir.types.new_infertype(None, span)
    }

    pub(crate) fn new_infer_with_modifier(&mut self, modifier: TypeModifier, span: Span) -> TypeId {
        self.hir.types.new_infertype(Some(modifier), span)
    }

    pub(crate) fn add_type(&mut self, ty: HirType) -> TypeId {
        self.hir.types.insert(ty)
    }
}

enum GenericKind {
    Generic(GenericId),
    Resolved(RefTypeId),
}

fn resolve_stub(
    scopes: &Vec<Scope>,
    types: &TypesMap,
    call_generics: &Vec<(String, RefTypeId)>,
    name: &str,
) -> Option<HirTypeKind> {
    if let Some(ty) = find_created_type(scopes, name) {
        return Some(ty)
    }

    let id = find_generic(scopes, call_generics, name)?;
    Some(match id {
        GenericKind::Generic(generic) => HirTypeKind::Generic(generic),
        GenericKind::Resolved(ref_type) => types.ref_to_type(ref_type)?.kind,
    })
}

fn find_created_type(
    scopes: &Vec<Scope>,
    name: &str,
) -> Option<HirTypeKind> {
    
    for store in scopes.iter().rev() {
        if let Some(ty) = store.created_type.get(name).copied() {
            return Some(ty);
        }
    }

    None
}

fn find_generic(
    scopes: &Vec<Scope>,
    call_generics: &Vec<(String, RefTypeId)>,
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
