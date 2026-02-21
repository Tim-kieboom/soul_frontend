use hir::{
    BlockId, ExpressionId, FunctionId, HirType, HirTypeKind, IdAlloc, LocalId, StatementId, TypeId, TypesMap, UnifyResult
};
use soul_utils::{
    error::SoulResult, soul_names::PrimitiveTypes, span::Span, vec_map::{VecMap, VecMapIndex}
};

use crate::HirTypedContext;

impl<'a> HirTypedContext<'a> {
    pub(crate) fn unify(&mut self, value: ExpressionId, expect: TypeId, got: TypeId, span: Span) {

        match self
            .infer_table
            .unify_type_type(&mut self.type_table.types, expect, got, span)
        {
            Ok(UnifyResult::Ok) => (),
            Ok(UnifyResult::NeedsAutoCopy) => self.add_autocopy(value),
            Err(err) => {
                self.log_error(err);
                self.posion_expression(value);
            }
        }
    }

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

    pub(crate) fn type_local(&mut self, id: LocalId, type_id: TypeId, span: Span) -> TypeId {
        let ty = self.hir.locals[id];
        let modifier = self.get_type(ty).modifier;

        let resolved = self.resolve_untyped_primitive(type_id, span);
        let local_type_id = match resolved {
            Some(mut ty) => {
                ty.modifier = modifier;
                self.add_type(ty)
            }
            None => {
                let mut ty = self.get_type(type_id).clone();
                ty.modifier = modifier;
                self.add_type(ty)
            }
        };
        self.type_table.locals.insert(id, local_type_id);
        local_type_id
    }

    pub(crate) fn add_type(&mut self, ty: HirType) -> TypeId {
        self.type_table.types.insert(ty)
    }

    pub(crate) fn get_priority_type(&mut self, left: TypeId, right: TypeId) -> TypeId {
        self.infer_table
            .get_priority_type(&self.type_table.types, left, right)
    }

    pub(crate) fn resolve_all_types(&mut self) {
        macro_rules! resolve {
            ($field:ident, $id:ty) => {
                let cap = self.type_table.$field.cap();
                for i in 0..cap {
                    let type_id = match self.type_table.$field.raw_index(i) {
                        Some(val) => *val,
                        None => continue,
                    };
                    let index = <$id>::new_index(i);

                    let span = match self.hir.spans.$field.get(index).copied() {
                        Some(val) => val,
                        None => {
                            debug_assert!(false, "span of {:?} not found", index);
                            Span::default()
                        }
                    };
                    let resolved = self.resolve_type_strict(type_id, span).unwrap_or(TypeId::error());
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
            remap.insert(old_id, new_id);
        }

        self.type_table.types = new_types;
        self.type_table.remap_types(&remap);
    }

    fn collect_root_types(&self) -> Vec<TypeId> {
        let mut roots = Vec::new();

        roots.extend(self.type_table.locals.values().copied());
        roots.extend(self.type_table.blocks.values().copied());
        roots.extend(self.type_table.functions.values().copied());
        roots.extend(self.type_table.statements.values().copied());
        roots.extend(self.type_table.expressions.values().copied());

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

        let resolved = self.resolve_type_strict(ty, Span::default()).unwrap_or(TypeId::error());
        let typed = match self.resolve_untyped_primitive(resolved, Span::default_const()) {
            Some(val) => self.add_type(val),
            None => resolved,
        };
        let hir_ty = self.type_table.types.get_type(typed).expect("should have id");

        debug_assert!(
            !hir_ty.is_infertype(),
            "InferType leaked into final type graph"
        );

        let new_ty = match &hir_ty.kind {
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
            | HirTypeKind::Primitive(_) => hir_ty.clone(),

            HirTypeKind::InferType(_, _) => unreachable!("resolve_type must eliminate InferType"),
        };

        let new_id = new_types.insert(new_ty);
        map.insert(ty, new_id);
        Ok(new_id)
    }

    pub(crate) fn resolve_untyped_primitive(
        &mut self,
        base_type: TypeId,
        span: Span,
    ) -> Option<HirType> {
        let ty = self.get_type(base_type);
        let modifier = ty.modifier;
        let prim = match &ty.kind {
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
        })
    }

    fn posion_expression(&mut self, value: ExpressionId) {
        self.type_expression(value, TypeId::error());
    }
}
