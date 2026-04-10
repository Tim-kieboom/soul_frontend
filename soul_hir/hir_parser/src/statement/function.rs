use ast::{FunctionKind, NamedTupleElement, ReferenceType, SoulType, TypeKind};
use hir::TypeId;
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc},
    soul_error_internal,
};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_function(&mut self, function: &ast::Function) -> FunctionId {
        let id = match function.signature.node.id {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "function.id should be Some(_)",
                    Some(function.signature.span)
                ));
                FunctionId::error()
            }
        };

        let signature = &function.signature.node;
        self.insert_function(&signature.name, id);

        self.push_scope();
        let mut generics = vec![];
        for generic in &function.signature.node.generics {
            generics.push(self.insert_generic(generic.name.to_string()));
        }

        let mut parameters = vec![];

        if !matches!(signature.function_kind, FunctionKind::Static) {
            let recv_ty = match signature.function_kind {
                FunctionKind::Consume => {
                    self.lower_type(&signature.methode_type, signature.methode_type.span)
                }
                FunctionKind::MutRef => {
                    let ref_ty = SoulType::new(
                        None,
                        TypeKind::Reference(ReferenceType::new(
                            signature.methode_type.clone(),
                            true,
                        )),
                        signature.methode_type.span,
                    );
                    self.lower_type(&ref_ty, signature.methode_type.span)
                }
                FunctionKind::ConstRef => {
                    let ref_ty = SoulType::new(
                        None,
                        TypeKind::Reference(ReferenceType::new(
                            signature.methode_type.clone(),
                            false,
                        )),
                        signature.methode_type.span,
                    );
                    self.lower_type(&ref_ty, signature.methode_type.span)
                }
                FunctionKind::Static => unreachable!(),
            };
            let local = self.id_generator.alloc_local();
            let this_name = Ident::new("this".to_string(), signature.methode_type.span);
            self.insert_parameter(&this_name, local, recv_ty);
            parameters.push(hir::Parameter {
                local,
                ty: recv_ty,
                default: None,
            });
        }

        for NamedTupleElement {
            name,
            ty,
            default,
            node_id: _,
        } in &signature.parameters
        {
            let ty = self.lower_type(ty, name.span);
            let local = self.id_generator.alloc_local();
            self.insert_parameter(name, local, ty);

            let default = default.as_ref().map(|value| self.lower_expression(value));
            parameters.push(hir::Parameter {
                local,
                ty,
                default,
            });
        }

        let body = match function.signature.node.external {
            Some(language) => hir::FunctionBody::External(language),
            None => hir::FunctionBody::Internal(self.lower_block(&function.block)),
        };

        let return_type = match self.lower_type(&signature.return_type, signature.return_type.span)
        {
            hir::LazyTypeId::Known(type_id) => type_id,
            hir::LazyTypeId::Infer(_) => {
                self.log_error(SoulError::new(
                    "function return type should be known",
                    SoulErrorKind::TypeInferenceError,
                    Some(function.signature.span),
                ));
                TypeId::error()
            }
        };
        let owner_type = match self.lower_type(&signature.methode_type, signature.methode_type.span)
        {
            hir::LazyTypeId::Known(type_id) => type_id,
            hir::LazyTypeId::Infer(_) => {
                self.log_error(SoulError::new(
                    "function owner type should be known",
                    SoulErrorKind::TypeInferenceError,
                    Some(function.signature.span),
                ));
                TypeId::error()
            }
        };

        self.pop_scope();

        let hir_function = hir::Function {
            id,
            body,
            generics,
            parameters,
            owner_type,
            return_type,
            name: signature.name.clone(),
            kind: signature.function_kind,
        };
        self.tree.nodes.functions.insert(id, hir_function);
        id
    }

    fn insert_function(&mut self, name: &Ident, function: FunctionId) {
        let scope = match self.scopes.last_mut() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!(
                    "tryed to insert_local in global scope",
                    Some(name.span)
                ));
                return;
            }
        };

        self.tree.info.spans.functions.insert(function, name.span);
        scope.functions.insert(name.to_string(), function);
    }
}
