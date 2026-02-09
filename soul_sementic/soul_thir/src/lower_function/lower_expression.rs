use hir_model as hir;
use soul_utils::soul_error_internal;
use thir_model::{self as thir, TypedExpression};

use crate::lower_function::LowerFunctionContext;

impl<'a> LowerFunctionContext<'a> {
    pub(super) fn lower_expression(&mut self, hir_id: hir::ExpressionId) -> thir::ExpressionId {
        let hir_expression = &self.hir.root.expressions[hir_id];
        let ty = self.typed_context.types[hir_id].clone();

        let kind = match &hir_expression.node {
            hir::ExpressionKind::Literal(literal) => thir::ExpressionKind::Literal(literal.clone()),
            hir::ExpressionKind::ResolvedVariable(node_id) => {
                thir::ExpressionKind::Place(thir::Place::Local {
                    id: *node_id,
                    local: self.id_generator.alloc_local(),
                })
            }
            hir::ExpressionKind::Binary(binary) => thir::ExpressionKind::Binary {
                left: self.lower_expression(binary.left),
                operator: binary.operator.clone(),
                right: self.lower_expression(binary.right),
            },
            hir::ExpressionKind::Unary(unary) => thir::ExpressionKind::Unary {
                operator: unary.operator.clone(),
                value: self.lower_expression(unary.expression),
            },
            hir::ExpressionKind::Ref(r#ref) => thir::ExpressionKind::Ref {
                mutable: r#ref.mutable,
                value: self.lower_expression(r#ref.expression),
            },
            hir::ExpressionKind::DeRef(de_ref) => thir::ExpressionKind::Deref {
                value: self.lower_expression(de_ref.inner),
            },
            hir::ExpressionKind::Index(index) => thir::ExpressionKind::Index {
                base: self.lower_expression(index.collection),
                index: self.lower_expression(index.index),
            },
            hir::ExpressionKind::FunctionCall(function_call) => thir::ExpressionKind::Call {
                function: function_call.resolved,
                arguments: function_call
                    .arguments
                    .iter()
                    .map(|id| self.lower_expression(id.node))
                    .collect(),
            },
            hir::ExpressionKind::Block(node_id) => {
                thir::ExpressionKind::Block(self.lower_body(*node_id))
            }

            hir::ExpressionKind::Null
            | hir::ExpressionKind::If(_)
            | hir::ExpressionKind::Default
            | hir::ExpressionKind::Fall(_)
            | hir::ExpressionKind::While(_)
            | hir::ExpressionKind::Array(_)
            | hir::ExpressionKind::Break(_)
            | hir::ExpressionKind::Return(_)
            | hir::ExpressionKind::Continue(_)
            | hir::ExpressionKind::AsCastType(_) => {
                self.log_error(soul_error_internal!(
                    format!(
                        "expressionKind of NodeId({}) is not impl in thir lowerer",
                        hir_id.display()
                    ),
                    Some(hir_expression.get_span())
                ));
                thir::ExpressionKind::Block(thir::Body::new(
                    thir::BodyKind {
                        statements: vec![],
                        tail: None,
                    },
                    hir_expression.get_span(),
                ))
            }
        };

        let id = self.id_generator.alloc_expression();

        let meta_data = hir_expression.get_meta_data().clone();
        let value = thir::Expression::with_meta_data(kind, meta_data);

        self.expressions.insert(id, TypedExpression { ty, value });
        return id;
    }

    pub(super) fn lower_place(&mut self, hir_id: hir::ExpressionId) -> thir::Place {
        let hir_expression = &self.hir.root.expressions[hir_id];

        match &hir_expression.node {
            hir_model::ExpressionKind::Index(index) => thir::Place::Index {
                base: self.lower_expression(index.collection),
                index: self.lower_expression(index.index),
            },
            hir_model::ExpressionKind::DeRef(de_ref) => {
                thir::Place::Deref(self.lower_expression(de_ref.inner))
            }
            hir_model::ExpressionKind::ResolvedVariable(node_id) => thir::Place::Local {
                id: *node_id,
                local: self.id_generator.alloc_local(),
            },
            _ => thir::Place::Local {
                id: hir_id,
                local: self.id_generator.alloc_local(),
            },
        }
    }
}
