use crate::{ThirContext, lower_function::LowerFunctionContext};
use hir_model as hir;
use soul_utils::{error::{SoulError, SoulErrorKind}, soul_error_internal, span::Span};
use thir_model::{self as thir, TypedExpression};

impl<'a> ThirContext<'a> {
    pub(crate) fn lower_item(&mut self, item: &hir::Item) {
        match &item.node {
            hir::ItemKind::Import(_) => (),
            hir::ItemKind::Function(function) => {
                let id = self.item_id_generator.alloc();
                let func = self.lower_function(function);
                self.thir.items.insert(id, thir::Item::Function(func));
            }
            hir::ItemKind::Variable(variable) => {
                let id = self.item_id_generator.alloc();
                let global = self.lower_global(variable);
                self.thir.items.insert(id, thir::Item::Global(global));
            }
        }
    }

    pub(crate) fn lower_function(&mut self, function: &hir::Function) -> thir::Function {
        let mut context = LowerFunctionContext::new(self.hir, self.typed_context, &mut self.faults);

        let body = context.lower_body(function.body);

        thir::Function {
            owner: function.id,
            body,
            locals: context.locals,
            expressions: context.expressions,
        }
    }

    pub(crate) fn lower_global(&mut self, variable: &hir::Variable) -> thir::Global {
        thir::Global {
            owner: variable.id,
            local: self.id_generator.alloc_local(),
            value: variable.value.map(|id| self.lower_expression(id)),
        }
    }

    fn lower_expression(&mut self, hir_id: hir::ExpressionId) -> thir::ExpressionId {
        let hir_expression = &self.hir.root.expressions[hir_id];
        
        let span = hir_expression.get_span();
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
            hir::ExpressionKind::Block(_) => {
                self.log_error(SoulError::new(
                    "block not allowed in global scope", 
                    SoulErrorKind::Empty, 
                    Some(span),
                ));
                thir::ExpressionKind::Block(empty_body(span))
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

        self.thir.global_expressions.insert(id, TypedExpression { ty, value });
        return id;
    }
}

fn empty_body(span: Span) -> thir::Body {
    thir::Body::new(
        thir::BodyKind{statements: vec![], tail: None}, 
        span,
    )
}
