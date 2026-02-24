use hir::{HirType, HirTypeKind, InferTypeId, TypeId, TypesMap, UnifyResult};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult}, soul_error_internal, soul_names::TypeModifier, span::Span, vec_map::{VecMap, VecMapIndex}
};

#[derive(Debug, Clone)]
pub(crate) struct InferTable {
    table: VecMap<InferTypeId, InferBinding>,
    id_generator: hir::IdGenerator<InferTypeId>,
}
impl InferTable {
    pub(crate) fn new(types: &TypesMap) -> Self {
        let table = types
            .iter_types()
            .filter_map(|ty| match &ty.kind {
                HirTypeKind::InferType(id, span) => Some((*id, InferBinding::Unbound(*span))),
                _ => None,
            })
            .collect::<VecMap<_, _>>();

        Self {
            table,
            id_generator: hir::IdGenerator::from_id(types.last_infertype()),
        }
    }

    pub(crate) fn alloc(&mut self, span: Span) -> InferTypeId {
        let id = self.id_generator.alloc();
        self.table.insert(id, InferBinding::Unbound(span));
        id
    }

    pub(crate) fn find_root(&mut self, id: InferTypeId) -> SoulResult<InferTypeId> {
        match self.table.get(id) {
            Some(&InferBinding::Alias(parent)) => {
                let root = self.find_root(parent)?;
                self.table.insert(id, InferBinding::Alias(root));
                Ok(root)
            }
            Some(_) => Ok(id),
            None => Err(soul_error_internal!(
                format!("InferVarId({}) not found", id.index()),
                None
            )),
        }
    }

    pub(crate) fn get_binding(&mut self, id: InferTypeId) -> SoulResult<InferBinding> {
        let root = self.find_root(id)?;
        match self.table.get(root) {
            Some(val) => Ok(*val),
            None => Err(soul_error_internal!(
                format!("InferVarId({}) not found", root.index()),
                None
            )),
        }
    }

    pub(crate) fn unify_type_type(
        &mut self,
        types: &mut TypesMap,
        expected: TypeId,
        got_type: TypeId,
        span: Span,
    ) -> SoulResult<UnifyResult> {
        let a_id = self.resolve_type_lazy(types, expected, span)?;
        let b_id = self.resolve_type_lazy(types, got_type, span)?;
        let a_ty = self.get_type(types, a_id)?;
        let b_ty = self.get_type(types, b_id)?;

        match (&a_ty.kind, &b_ty.kind) {
            (HirTypeKind::InferType(a_inf, _), _) => {
                self.unify_var_type(types, *a_inf, got_type, span)
            }
            (_, HirTypeKind::InferType(b_inf, _)) => {
                self.unify_var_type(types, *b_inf, expected, span)
            }

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
                if let Some(msg) = HirTypeKind::arraykind_compatible(a_kind, b_kind) {
                    return Err(SoulError::new(
                        msg,
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                }
                self.unify_type_type(types, *a_el, *b_el, span)
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
                            a_ty.display(types),
                            b_ty.display(types)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                }
                self.unify_type_type(types, *a_id, *b_id, span)
            }

            (HirTypeKind::Pointer(a_id), HirTypeKind::Pointer(b_id))
            | (HirTypeKind::Optional(a_id), HirTypeKind::Optional(b_id)) => {
                self.unify_type_type(types, *a_id, *b_id, span)
            }

            (HirTypeKind::Error, _) | (_, HirTypeKind::Error) => Ok(UnifyResult::Ok),

            _ => {
                a_ty.compatible_type_kind(b_ty).map_err(|reason| {
                    SoulError::new(
                        format!(
                            "Type mismatch: expected {} got {} because {reason}",
                            a_ty.display(types),
                            b_ty.display(types)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    )
                })?;
                Ok(UnifyResult::Ok)
            }
        }
    }

    pub(crate) fn unify_var_type(
        &mut self,
        types: &mut TypesMap,
        var: InferTypeId,
        ty: TypeId,
        span: Span,
    ) -> SoulResult<UnifyResult> {
        let root = self.find_root(var)?;
        let ty = self.resolve_type_lazy(types, ty, span)?;

        if self.occurs_in(types, root, ty) {
            return Err(SoulError::new(
                "infinite type: inference variable occurs in its own definition",
                SoulErrorKind::UnifyTypeError,
                Some(span),
            ));
        }

        match self.get_binding(root)? {
            InferBinding::Unbound(_) => {
                self.table.insert(root, InferBinding::Bound(ty));
                Ok(UnifyResult::Ok)
            }

            InferBinding::Bound(expect_id) => {
                let expect_id = self.resolve_type_lazy(types, expect_id, span)?;

                let expecting = self.get_type(types, expect_id)?;
                let is_type = self.get_type(types, ty)?;

                expecting.compatible_type_kind(is_type).map_err(|reason| {
                    SoulError::new(
                        format!(
                            "Type mismatch: expected {} got {} because {reason}",
                            expecting.display(types),
                            is_type.display(types)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    )
                })
            }

            InferBinding::Alias(_) => unreachable!("find_root guarantees root is not Alias"),
        }
    }

    pub(crate) fn get_priority_type(
        &mut self,
        types: &TypesMap,
        left: TypeId,
        right: TypeId,
    ) -> TypeId {
        let left_type = self.get_type(types, left).expect("should have id");
        let right_type = self.get_type(types, right).expect("should have id");
        match left_type.get_priority(right_type) {
            hir::Priority::Left => left,
            hir::Priority::Right => right,
        }
    }

    pub(crate) fn add_infer_binding(&mut self, id: InferTypeId, kown: TypeId) {
        self.table.insert(id, InferBinding::Bound(kown));
    }

    pub(crate) fn resolve_type_strict(
        &mut self,
        types: &mut TypesMap,
        ty: TypeId,
        span: Span,
    ) -> SoulResult<TypeId> {
        let hir_ty = self.get_type(types, ty)?;
        let modifier = hir_ty.modifier;

        match &hir_ty.kind {
            HirTypeKind::InferType(inf, _) => {
                let root = self.find_root(*inf)?;
                match self.table.get(root) {
                    Some(InferBinding::Bound(t)) => self.resolve_type_strict(types, *t, span),
                    Some(InferBinding::Unbound(span)) => {
                        #[cfg(debug_assertions)]
                        return Err(SoulError::new(
                            format!("type {:?} could not be inferd", ty),
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
                        return Err(soul_error_internal!(
                            format!("{:?} not found", root),
                            Some(span)
                        ));
                    }
                }
            }

            HirTypeKind::Pointer(inner) => {
                let inner_resolved = self.resolve_type_strict(types, *inner, span)?;
                Ok(types.insert(HirType {
                    kind: HirTypeKind::Pointer(inner_resolved),
                    modifier,
                }))
            }

            HirTypeKind::Optional(inner) => {
                let inner_resolved = self.resolve_type_strict(types, *inner, span)?;
                Ok(types.insert(HirType {
                    kind: HirTypeKind::Optional(inner_resolved),
                    modifier,
                }))
            }

            HirTypeKind::Ref { of_type, mutable } => {
                let mutable = *mutable;
                let inner_resolved = self.resolve_type_strict(types, *of_type, span)?;
                Ok(types.insert(HirType {
                    kind: HirTypeKind::Ref {
                        of_type: inner_resolved,
                        mutable,
                    },
                    modifier,
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
                    types.insert(new)
                };

                Ok(types.insert(HirType {
                    kind: HirTypeKind::Array {
                        element,
                        kind,
                    },
                    modifier,
                }))
            }

            _ => Ok(ty),
        }
    }

    pub(crate) fn resolve_type_lazy(
        &mut self,
        types: &mut TypesMap,
        ty: TypeId,
        span: Span,
    ) -> SoulResult<TypeId> {
        let hir_ty = self.get_type(types, ty)?;
        let modifier = hir_ty.modifier;
        let resolved = match &hir_ty.kind {
            HirTypeKind::InferType(inf, _) => {
                let root = self.find_root(*inf)?;
                return match self.table.get(root) {
                    Some(InferBinding::Bound(t)) => self.insure_modifier(types, *t, modifier),
                    Some(InferBinding::Unbound(_)) => Ok(ty),
                    Some(InferBinding::Alias(_)) => unreachable!(),
                    None => Err(soul_error_internal!("InferTypeId not found", None)),
                };
            }
            HirTypeKind::None
            | HirTypeKind::Type
            | HirTypeKind::Error
            | HirTypeKind::Primitive(_) => return Ok(ty),

            HirTypeKind::Pointer(id) => HirType {
                kind: HirTypeKind::Pointer(self.resolve_type_lazy(types, *id, span)?),
                modifier,
            },
            HirTypeKind::Optional(id) => HirType {
                kind: HirTypeKind::Optional(self.resolve_type_lazy(types, *id, span)?),
                modifier,
            },
            HirTypeKind::Ref { of_type, mutable } => {
                let of_type = *of_type;
                let mutable = *mutable;
                HirType {
                    kind: HirTypeKind::Ref {
                        of_type: self.resolve_type_lazy(types, of_type, span)?,
                        mutable,
                    },
                    modifier,
                }
            }
            HirTypeKind::Array { element, kind } => {
                let element = *element;
                let kind = *kind;
                HirType {
                    kind: HirTypeKind::Array {
                        element: self.resolve_type_lazy(types, element, span)?,
                        kind,
                    },
                    modifier,
                }
            }
        };

        Ok(types.insert(resolved))
    }

    pub(crate) fn insure_modifier(&mut self, types: &mut TypesMap, ty: TypeId, modifier: Option<TypeModifier>) -> SoulResult<TypeId> {
        let hir_type = self.get_type(types, ty)?;
        if hir_type.modifier == modifier {
            return Ok(ty)
        }

        let mut new = hir_type.clone();
        new.modifier = modifier;
        Ok(
            types.insert(new)
        )
    }

    fn occurs_in(&mut self, types: &TypesMap, var: InferTypeId, ty: TypeId) -> bool {
        let hir_ty = match types.get_type(ty) {
            Some(t) => t,
            None => return false,
        };

        match &hir_ty.kind {
            HirTypeKind::InferType(inf, _) => self.find_root(*inf).ok() == self.find_root(var).ok(),

            HirTypeKind::Pointer(id) | HirTypeKind::Optional(id) => self.occurs_in(types, var, *id),

            HirTypeKind::Ref { of_type, .. } => self.occurs_in(types, var, *of_type),

            HirTypeKind::Array { element, .. } => self.occurs_in(types, var, *element),

            _ => false,
        }
    }

    fn get_type<'a>(&self, types: &'a TypesMap, ty: TypeId) -> SoulResult<&'a HirType> {
        match types.get_type(ty) {
            Some(val) => Ok(val),
            None => Err(soul_error_internal!(
                format!("TypeId({}) not found", ty.index()),
                None
            )),
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
