use hir::{HirType, HirTypeKind, InferTypeId, TypeId, TypesMap, UnifyResult};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_error_internal,
    span::Span,
    vec_map::{VecMap, VecMapIndex},
};

#[derive(Debug, Clone)]
pub(crate) struct InferTable<'a> {
    types: &'a TypesMap,
    table: VecMap<InferTypeId, InferBinding>,
    id_generator: hir::IdGenerator<InferTypeId>,
}
impl<'a> InferTable<'a> {
    pub(crate) fn new(types: &'a TypesMap) -> Self {
        let table = types.iter_types()
            .filter_map(|ty| {
                match &ty.kind {
                    HirTypeKind::InferType(id) => Some((*id, InferBinding::Unbound)),
                    _ => None,
                }
            })
            .collect::<VecMap<_,_>>();

        Self {
            types,
            table,
            id_generator: hir::IdGenerator::from_id(types.last_infertype()),
        }
    }

    pub(crate) fn alloc(&mut self) -> InferTypeId {
        let id = self.id_generator.alloc();
        self.table.insert(id, InferBinding::Unbound);
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
        expected: TypeId,
        got_type: TypeId,
        span: Span,
    ) -> SoulResult<UnifyResult> {
        let a_ty = self.get_type(expected)?;
        let b_ty = self.get_type(got_type)?;

        match (&a_ty.kind, &b_ty.kind) {
            (HirTypeKind::InferType(a_inf), _) => self.unify_var_type(*a_inf, got_type, span),
            (_, HirTypeKind::InferType(b_inf)) => self.unify_var_type(*b_inf, expected, span),

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
                self.unify_type_type(*a_el, *b_el, span)
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
                            a_ty.display(self.types),
                            b_ty.display(self.types)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    ));
                }
                self.unify_type_type(*a_id, *b_id, span)
            }

            (HirTypeKind::Pointer(a_id), HirTypeKind::Pointer(b_id))
            | (HirTypeKind::Optional(a_id), HirTypeKind::Optional(b_id)) => {
                self.unify_type_type(*a_id, *b_id, span)
            }

            _ => {
                a_ty.compatible_type_kind(b_ty).map_err(|reason| {
                    SoulError::new(
                        format!(
                            "Type mismatch: expected {} got {} because {reason}",
                            a_ty.display(self.types),
                            b_ty.display(self.types)
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
        var: InferTypeId,
        ty: TypeId,
        span: Span,
    ) -> SoulResult<UnifyResult> {
        let root = self.find_root(var)?;
        let ty = self.resolve_type(ty)?;

        match self.get_binding(root)? {
            InferBinding::Unbound => {
                self.table.insert(root, InferBinding::Bound(ty));
                Ok(UnifyResult::Ok)
            }

            InferBinding::Bound(expect_id) => {
                let expect_id = self.resolve_type(expect_id)?;
                
                let expecting = self.get_type(expect_id)?;
                let is_type = self.get_type(ty)?;

                expecting.compatible_type_kind(is_type).map_err(|reason| {
                    SoulError::new(
                        format!(
                            "Type mismatch: expected {} got {} because {reason}",
                            expecting.display(&self.types),
                            is_type.display(&self.types)
                        ),
                        SoulErrorKind::UnifyTypeError,
                        Some(span),
                    )
                })
            }

            InferBinding::Alias(_) => unreachable!("find_root guarantees root is not Alias"),
        }
    }

    pub(crate) fn get_priority_type(&mut self, left: TypeId, right: TypeId) -> TypeId {
        let left_type = self.get_type(left).expect("should have id");
        let right_type = self.get_type(right).expect("should have id");
        match left_type.get_priority(right_type) {
            hir::Priority::Left => left,
            hir::Priority::Right => right,
        }
    }

    fn resolve_type(&mut self, ty: TypeId) -> SoulResult<TypeId> {
        let hir_ty = self.get_type(ty)?;
        match hir_ty.kind {
            HirTypeKind::InferType(inf) => {
                let root = self.find_root(inf)?;
                match self.table.get(root) {
                    Some(InferBinding::Bound(t)) => Ok(*t),
                    Some(InferBinding::Unbound) => Ok(ty),
                    Some(InferBinding::Alias(_)) => unreachable!(),
                    None => Err(soul_error_internal!("InferTypeId not found", None)),
                }
            }
            _ => Ok(ty),
        }
    }
        
    fn get_type(&self, ty: TypeId) -> SoulResult<&HirType> {
        self.types.get_type(ty).ok_or(soul_error_internal!(
            format!("TypeId({}) not found", ty.index()),
            None
        ))
    }
}

/// A binding for an inference variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InferBinding {
    /// Still unbound
    Unbound,

    /// Bound to a concrete type
    Bound(TypeId),

    /// Bound to another inference variable (for union-find / path compression)
    Alias(InferTypeId),
}
