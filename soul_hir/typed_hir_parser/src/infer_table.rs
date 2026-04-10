use hir::{
    DisplayType, HirType, HirTypeKind, InferType, InferTypeId, InferTypesMap, LazyTypeId, TypeId,
    TypesMap,
};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_error_internal,
    soul_names::TypeModifier,
    span::Span,
    vec_map::VecMap,
};

use crate::type_helpers::{ArrayKindCompatible, GetPriority, TypeCompatible};

pub enum UnifyResult {
    /// fully unifyable
    Ok,
    /// error if auto copy not impl
    NeedsAutoCopy,
}

#[derive(Debug, Clone)]
pub(crate) struct InferTable {
    pub(crate) table: VecMap<InferTypeId, InferBinding>,
}
impl InferTable {
    pub(crate) fn new(infers: &InferTypesMap) -> Self {
        let table = infers
            .entries()
            .map(|(id, (_, span))| (id, InferBinding::Unbound(*span)))
            .collect::<VecMap<_, _>>();

        Self { table }
    }

    pub(crate) fn alloc(&mut self, infer: InferTypeId, span: Span) {
        if self.table.get(infer).is_some() {
            #[cfg(debug_assertions)]
            panic!("already has {:?} in InferTable", infer);
            #[cfg(not(debug_assertions))]
            return;
        }

        self.table.insert(infer, InferBinding::Unbound(span));
    }

    pub(crate) fn add_infer_binding(&mut self, id: InferTypeId, kown: LazyTypeId) {
        let binding = match kown {
            LazyTypeId::Known(type_id) => InferBinding::Bound(type_id),
            LazyTypeId::Infer(infer) => InferBinding::Alias(infer),
        };
        self.table.insert(id, binding);
    }

    pub(crate) fn get_priority_type(
        &mut self,
        types: &TypesMap,
        left: TypeId,
        right: TypeId,
    ) -> TypeId {
        let left_type = match self.get_type(types, left) {
            Ok(val) => val,
            Err(_) => return right,
        };
        let right_type = match self.get_type(types, right) {
            Ok(val) => val,
            Err(_) => return left,
        };

        match left_type.get_priority(right_type) {
            crate::type_helpers::Priority::Left => left,
            crate::type_helpers::Priority::Right => right,
        }
    }

    pub(crate) fn unify_type_type(
        &mut self,
        types: &mut TypesMap,
        infers: &InferTypesMap,
        expected: LazyTypeId,
        got_type: LazyTypeId,
        span: Span,
    ) -> SoulResult<UnifyResult> {
        let a_id = self.resolve_type_lazy(types, infers, expected, span)?;
        let b_id = self.resolve_type_lazy(types, infers, got_type, span)?;
        if a_id == LazyTypeId::error() || b_id == LazyTypeId::error() {
            return Ok(UnifyResult::Ok);
        }

        let (a_id, b_id) = match (a_id, b_id) {
            (LazyTypeId::Infer(a_infer), LazyTypeId::Infer(b_infer)) => {
                return self.unify_var_var(types, infers, a_infer, b_infer, span);
            }
            (LazyTypeId::Infer(a_infer), _) => {
                return self.unify_var_type(types, infers, a_infer, got_type, span);
            }
            (_, LazyTypeId::Infer(b_infer)) => {
                return self.unify_var_type(types, infers, b_infer, expected, span);
            }
            (LazyTypeId::Known(a), LazyTypeId::Known(b)) => (a, b),
        };

        let a_ty = self.get_type(types, a_id)?;
        let b_ty = self.get_type(types, b_id)?;

        match (&a_ty.kind, &b_ty.kind) {
            (
                HirTypeKind::Array {
                    element: a_el,
                    kind: a_kind,
                },
                HirTypeKind::Array {
                    element: b_el,
                    kind: b_kind,
                },
            ) => {
                if let Err(msg) = HirTypeKind::arraykind_compatible(a_kind, b_kind) {
                    return Err(SoulError::new(
                        msg,
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                }
                self.unify_type_type(types, infers, *a_el, *b_el, span)
            }

            (
                HirTypeKind::Ref {
                    of_type: a_id,
                    mutable: a_mut,
                },
                HirTypeKind::Ref {
                    of_type: b_id,
                    mutable: b_mut,
                },
            ) => {
                if a_mut != b_mut {
                    return Err(SoulError::new(
                        format!(
                            "Type mismatch: expected {} got {}",
                            a_ty.display(types, infers),
                            b_ty.display(types, infers)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                }
                self.unify_type_type(types, infers, *a_id, *b_id, span)
            }

            (HirTypeKind::Pointer(a_id), HirTypeKind::Pointer(b_id))
            | (HirTypeKind::Optional(a_id), HirTypeKind::Optional(b_id)) => {
                self.unify_type_type(types, infers, *a_id, *b_id, span)
            }

            (HirTypeKind::Error, _) | (_, HirTypeKind::Error) => Ok(UnifyResult::Ok),

            _ => a_ty.compatible_type_kind(b_ty).map_err(|reason| {
                SoulError::new(
                    format!(
                        "Type mismatch: expected '{}' got '{}' because {reason}",
                        a_ty.display(types, infers),
                        b_ty.display(types, infers)
                    ),
                    SoulErrorKind::UnifyTypeError,
                    Some(span),
                )
            }),
        }
    }

    pub(crate) fn resolve_type_lazy(
        &mut self,
        types: &mut TypesMap,
        infers: &InferTypesMap,
        ty: LazyTypeId,
        span: Span,
    ) -> SoulResult<LazyTypeId> {
        if ty == LazyTypeId::error() {
            return Ok(ty);
        }

        let ty = match ty {
            LazyTypeId::Known(val) => val,
            LazyTypeId::Infer(infer) => return self.resolve_infer_lazy(types, infers, infer),
        };

        let hir_type = self.get_type(types, ty)?;
        let modifier = hir_type.modifier;

        let resolved = match &hir_type.kind {
            HirTypeKind::None
            | HirTypeKind::Type
            | HirTypeKind::Error
            | HirTypeKind::Struct(_)
            | HirTypeKind::Generic(_)
            | HirTypeKind::Primitive(_) => return Ok(hir::LazyTypeId::Known(ty)),

            HirTypeKind::Pointer(id) => {
                let generics = hir_type.generics.clone();
                HirType {
                    kind: HirTypeKind::Pointer(self.resolve_type_lazy(types, infers, *id, span)?),
                    modifier,
                    generics,
                }
            }
            HirTypeKind::Optional(id) => {
                let generics = hir_type.generics.clone();
                HirType {
                    kind: HirTypeKind::Optional(self.resolve_type_lazy(types, infers, *id, span)?),
                    modifier,
                    generics,
                }
            }
            HirTypeKind::Ref { of_type, mutable } => {
                let generics = hir_type.generics.clone();
                let of_type = *of_type;
                let mutable = *mutable;
                HirType {
                    kind: HirTypeKind::Ref {
                        of_type: self.resolve_type_lazy(types, infers, of_type, span)?,
                        mutable,
                    },
                    modifier,
                    generics,
                }
            }
            HirTypeKind::Array { element, kind } => {
                let generics = hir_type.generics.clone();
                let element = *element;
                let kind = *kind;
                HirType {
                    kind: HirTypeKind::Array {
                        element: self.resolve_type_lazy(types, infers, element, span)?,
                        kind,
                    },
                    modifier,
                    generics,
                }
            }
        };

        Ok(hir::LazyTypeId::Known(types.insert_type(resolved)))
    }

    pub(crate) fn resolve_type_strict(
        &mut self,
        types: &mut TypesMap,
        ty: LazyTypeId,
        span: Option<Span>,
    ) -> SoulResult<TypeId> {
        let ty = match ty {
            LazyTypeId::Known(val) => val,
            LazyTypeId::Infer(infer) => return self.resolve_infer_strict(infer, span),
        };

        let hir_ty = self.get_type(types, ty)?;
        let modifier = hir_ty.modifier;
        let generics = hir_ty.generics.clone();

        match &hir_ty.kind {
            HirTypeKind::Pointer(inner) => {
                let inner_resolved = self.resolve_type_strict(types, *inner, span)?;
                Ok(types.insert_type(HirType {
                    kind: HirTypeKind::Pointer(inner_resolved.to_lazy()),
                    modifier,
                    generics,
                }))
            }

            HirTypeKind::Optional(inner) => {
                let inner_resolved = self.resolve_type_strict(types, *inner, span)?;
                Ok(types.insert_type(HirType {
                    kind: HirTypeKind::Optional(inner_resolved.to_lazy()),
                    modifier,
                    generics,
                }))
            }

            HirTypeKind::Ref { of_type, mutable } => {
                let mutable = *mutable;
                let inner_resolved = self.resolve_type_strict(types, *of_type, span)?;
                Ok(types.insert_type(HirType {
                    kind: HirTypeKind::Ref {
                        of_type: inner_resolved.to_lazy(),
                        mutable,
                    },
                    modifier,
                    generics,
                }))
            }

            HirTypeKind::Array { element, kind } => {
                let kind = *kind;
                let resolved = self.resolve_type_strict(types, *element, span)?;
                let element = if self.get_type(types, resolved)?.modifier == None {
                    resolved
                } else {
                    let mut new = self.get_type(types, resolved)?.clone();
                    new.modifier = None;
                    types.insert_type(new)
                }
                .to_lazy();

                Ok(types.insert_type(HirType {
                    kind: HirTypeKind::Array { element, kind },
                    modifier,
                    generics,
                }))
            }

            _ => Ok(ty),
        }
    }

    fn unify_var_type(
        &mut self,
        types: &mut TypesMap,
        infers: &InferTypesMap,
        var: InferTypeId,
        ty: LazyTypeId,
        span: Span,
    ) -> SoulResult<UnifyResult> {
        let root = self.find_root(var)?;
        let ty = self.resolve_type_lazy(types, infers, ty, span)?;

        if self.occurs_in(types, root, ty) {
            return Err(SoulError::new(
                "infinite type: inference variable occurs in its own definition",
                SoulErrorKind::UnifyTypeError,
                Some(span),
            ));
        }

        match self.get_binding(root)? {
            InferBinding::Unbound(_) => {
                let known = self.resolve_type_strict(types, ty, Some(span))?;
                self.table.insert(root, InferBinding::Bound(known));
                Ok(UnifyResult::Ok)
            }

            InferBinding::Bound(expect_id) => {
                let ty = self.resolve_type_strict(types, ty, Some(span))?;
                let expecting = self.get_type(types, expect_id)?;
                let is_type = self.get_type(types, ty)?;

                expecting.compatible_type_kind(is_type).map_err(|reason| {
                    SoulError::new(
                        format!(
                            "Type mismatch: expected {} got {} because {reason}",
                            expecting.display(types, infers),
                            is_type.display(types, infers)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    )
                })
            }

            InferBinding::Alias(_) => unreachable!("find_root guarantees root is not Alias"),
        }
    }

    fn unify_var_var(
        &mut self,
        types: &mut TypesMap,
        infers: &InferTypesMap,
        a: InferTypeId,
        b: InferTypeId,
        span: Span,
    ) -> SoulResult<UnifyResult> {
        let a_root = self.find_root(a)?;
        let b_root = self.find_root(b)?;

        if a_root == b_root {
            return Ok(UnifyResult::Ok);
        }

        match (self.table[a_root], self.table[b_root]) {
            (InferBinding::Unbound(_), _) => {
                self.table.insert(a_root, InferBinding::Alias(b_root));
            }
            (_, InferBinding::Unbound(_)) => {
                self.table.insert(b_root, InferBinding::Alias(a_root));
            }
            (InferBinding::Bound(a_ty), InferBinding::Bound(b_ty)) => {
                self.unify_type_type(types, infers, a_ty.to_lazy(), b_ty.to_lazy(), span)?;
                self.table.insert(a_root, InferBinding::Alias(b_root));
            }
            _ => unreachable!(),
        }

        Ok(UnifyResult::Ok)
    }

    fn resolve_infer_strict(
        &mut self,
        infer: InferTypeId,
        span: Option<Span>,
    ) -> SoulResult<TypeId> {
        let root = self.find_root(infer)?;
        match self.table.get(root) {
            Some(InferBinding::Bound(ty)) => Ok(*ty),
            Some(InferBinding::Unbound(span)) => {
                #[cfg(debug_assertions)]
                return Err(SoulError::new(
                    format!("type {:?} could not be inferd", infer),
                    SoulErrorKind::UnifyTypeError,
                    Some(*span),
                ));
                #[cfg(not(debug_assertions))]
                return Err(SoulError::new(
                    "type could not be inferd",
                    SoulErrorKind::UnifyTypeError,
                    Some(*span),
                ));
            }
            Some(_) => unreachable!(),
            None => {
                return Err(soul_error_internal!(format!("{:?} not found", root), span));
            }
        }
    }

    fn resolve_infer_lazy(
        &mut self,
        types: &mut TypesMap,
        infers: &InferTypesMap,
        infer: InferTypeId,
    ) -> SoulResult<LazyTypeId> {
        let modifier = self.get_infer(infers, infer)?.modifier;
        let root = self.find_root(infer)?;
        match self.table.get(root) {
            Some(InferBinding::Bound(known)) => {
                let type_id = self.insure_modifier(types, *known, modifier)?;
                Ok(LazyTypeId::Known(type_id))
            }
            Some(InferBinding::Alias(_)) => Ok(LazyTypeId::Infer(infer)),
            Some(InferBinding::Unbound(_)) => Ok(LazyTypeId::Infer(root)),
            None => Err(soul_error_internal!("InferTypeId not found", None)),
        }
    }

    pub(crate) fn get_binding(&mut self, id: InferTypeId) -> SoulResult<InferBinding> {
        let root = self.find_root(id)?;
        match self.table.get(root) {
            Some(val) => Ok(*val),
            None => Err(soul_error_internal!(format!("{:?} not found", root), None)),
        }
    }

    pub(crate) fn find_root(&mut self, id: InferTypeId) -> SoulResult<InferTypeId> {
        match self.table.get(id) {
            Some(&InferBinding::Alias(parent)) => {
                let root = self.find_root(parent)?;
                self.table.insert(id, InferBinding::Alias(root));
                Ok(root)
            }
            Some(_) => Ok(id),
            None => Err(soul_error_internal!(format!("{:?} not found", id), None)),
        }
    }

    pub(crate) fn insure_modifier(
        &mut self,
        types: &mut TypesMap,
        ty: TypeId,
        modifier: Option<TypeModifier>,
    ) -> SoulResult<TypeId> {
        let hir_type = self.get_type(types, ty)?;
        if hir_type.modifier == modifier {
            return Ok(ty);
        }

        let mut new = hir_type.clone();
        new.modifier = modifier;
        Ok(types.insert_type(new))
    }

    fn occurs_in(&mut self, types: &TypesMap, var: InferTypeId, ty: LazyTypeId) -> bool {
        let ty = match ty {
            LazyTypeId::Known(val) => val,
            LazyTypeId::Infer(infer) => {
                return self.find_root(infer).ok() == self.find_root(var).ok();
            }
        };

        let hir_ty = match types.id_to_type(ty) {
            Some(t) => t,
            None => return false,
        };

        match &hir_ty.kind {
            HirTypeKind::Pointer(id) | HirTypeKind::Optional(id) => self.occurs_in(types, var, *id),
            HirTypeKind::Ref { of_type, .. } => self.occurs_in(types, var, *of_type),
            HirTypeKind::Array { element, .. } => self.occurs_in(types, var, *element),
            _ => false,
        }
    }

    fn get_type<'b>(&self, types: &'b TypesMap, ty: TypeId) -> SoulResult<&'b HirType> {
        match types.id_to_type(ty) {
            Some(val) => Ok(val),
            None => Err(soul_error_internal!(format!("{:?} not found", ty), None)),
        }
    }

    fn get_infer<'d>(
        &self,
        infers: &'d InferTypesMap,
        infer: InferTypeId,
    ) -> SoulResult<&'d InferType> {
        match infers.get_infer(infer) {
            Some(val) => Ok(val),
            None => Err(soul_error_internal!(format!("{:?} not found", infer), None)),
        }
    }
}

/// A binding for an inference variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InferBinding {
    /// Still unbound
    Unbound(Span),

    /// Bound to a concrete type
    Bound(TypeId),

    /// Bound to another inference variable (for union-find / path compression)
    Alias(InferTypeId),
}
