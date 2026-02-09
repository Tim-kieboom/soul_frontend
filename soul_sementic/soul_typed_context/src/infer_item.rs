use hir_model::{Body, BodyId, Function, HirType, HirTypeKind, Item, ItemKind, Variable};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    span::Span,
};

use crate::{TypedContextAnalyser, model::InferType, utils::none_ty};

impl<'a> TypedContextAnalyser<'a> {
    pub(crate) fn infer_item(&mut self, item: &Item) {
        match &item.node {
            ItemKind::Function(function) => self.infer_function(function),
            ItemKind::Variable(variable) => self.infer_variable(variable),
            ItemKind::Import(_) => todo!("infer import item"),
        }
    }

    pub(crate) fn infer_function(&mut self, function: &Function) {
        let signature = &function.signature;

        for parameter in &signature.parameters {
            let ty = InferType::Known(parameter.ty.clone());
            self.locals.insert(parameter.id, ty);
        }

        self.infer_typed_block(
            function.body,
            signature.return_type.clone(),
            signature.name.get_span(),
        );
    }

    pub(crate) fn infer_variable(&mut self, variable: &Variable) {
        let mut ty = match &variable.value {
            Some(val) => self.infer_rvalue(*val),
            None => self.environment.alloc_variable(variable.name.get_span()),
        };

        match &variable.ty {
            hir_model::VarTypeKind::NonInveredType(hir_type) => {
                self.unify_rtype(variable.id, &ty, hir_type, variable.name.get_span());
                ty = InferType::Known(hir_type.clone());
            }
            hir_model::VarTypeKind::InveredType(type_modifier) => match &mut ty {
                InferType::Known(hir_type) => hir_type.modifier = Some(*type_modifier),
                InferType::Variable(_, _) => (),
            },
        }

        self.try_resolve_untyped_number(&mut ty, None, variable.name.get_span());
        self.locals.insert(variable.id, ty);
    }

    pub(crate) fn infer_typed_block(
        &mut self,
        body_id: BodyId,
        expected_return: HirType,
        span: Span,
    ) -> InferType {
        let is_none = matches!(expected_return.kind, HirTypeKind::None);

        let return_count = self.current_return_count;
        self.current_return_count = 0;
        let return_type = self.current_return_type.take();
        self.current_return_type = Some(InferType::Known(expected_return));

        let ty = self.infer_block(body_id);

        if self.current_return_count == 0 && !is_none {
            self.log_error(SoulError::new(
                "missing return statement in body",
                SoulErrorKind::NotFoundInScope,
                Some(span),
            ));
        }

        self.current_return_type = return_type;
        self.current_return_count = return_count;
        ty
    }

    pub(crate) fn infer_block(&mut self, block_id: BodyId) -> InferType {
        
        let (block, span) = match &self.tree.root.bodies.get(block_id) {
            Some(Body::Block(block, span)) => (block, *span),
            _ => return InferType::Known(none_ty(Span::default_const())),
        };
        
        let mut last_ty = InferType::Known(none_ty(span));
        for statement in block.statements.values() {
            if let Some(ty) = self.infer_statement(statement) {
                last_ty = ty;
            }
        }

        last_ty
    }
}
