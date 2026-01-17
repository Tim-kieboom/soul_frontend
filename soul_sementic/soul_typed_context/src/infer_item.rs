use hir_model::{Body, BodyId, Function, Item, ItemKind, Variable};
use soul_utils::{span::Span};

use crate::{TypedContext, model::InferType, utils::none_ty};

impl<'a> TypedContext<'a> {
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

        let expected_return = InferType::Known(
            signature.return_type.clone(), 
        );

        self.infer_block(function.body, Some(expected_return));
    }

    pub(crate) fn infer_variable(&mut self, variable: &Variable) {
        let mut ty = match &variable.value {
            Some(val) => self.infer_rvalue(*val),
            None => self.environment.alloc_variable(variable.name.get_span()),
        };

        match &variable.ty {
            hir_model::VarTypeKind::NonInveredType(hir_type) => {
                self.unify_ltype(hir_type, &ty, variable.name.get_span());
            }
            hir_model::VarTypeKind::InveredType(type_modifier) => match &mut ty {
                InferType::Known(hir_type) => hir_type.modifier = Some(*type_modifier),
                InferType::Variable(_, _) => (),
            }
        }

        self.try_resolve_untyped(&mut ty, None, variable.name.get_span());
        self.locals.insert(variable.id, ty);
    }

    pub(crate) fn infer_block(
        &mut self, 
        block_id: BodyId, 
        expected_return: Option<InferType>, 
    ) -> InferType {
        let (block, span) = match &self.tree.root.bodies.get(block_id) {
            Some(Body::Block(block, span)) => (block, *span),
            _ => return InferType::Known(
                none_ty(Span::default_const()),
            ),
        };

        let mut last = self.environment.alloc_variable(span);

        let mut last_span = span;
        for statement in block.statements.values() {
            if let Some(ty) = self.infer_statement(statement) {
                last = ty;
            }
            last_span = statement.get_span();
        }

        if let Some(expected) = expected_return {
            self.unify(&last, &expected, last_span);
        }

        last
    }
}