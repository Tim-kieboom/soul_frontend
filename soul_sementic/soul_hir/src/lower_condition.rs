use crate::HirLowerer;
use hir_model as hir;
use parser_models::{ast, scope::NodeId};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    span::{Span, Spanned},
};

impl HirLowerer {
    pub(crate) fn lower_while(
        &mut self,
        r#while: &ast::While,
        span: Span,
    ) -> Option<(NodeId, hir::ExpressionKind)> {
        let id = self.expect_node_id(r#while.id, span)?;
        let condition = match &r#while.condition {
            Some(val) => Some(self.lower_expression(val)?),
            None => None,
        };

        let prev_scope = self.push_scope();
        let body = self.lower_block(&r#while.block, span)?;
        self.pop_scope(prev_scope);

        let body_id = body.id;
        self.push_block(body, r#while.block.span);

        Some((
            id,
            hir::ExpressionKind::While(hir::While {
                body: body_id,
                condition,
            }),
        ))
    }

    pub(crate) fn lower_if(
        &mut self,
        r#if: &ast::If,
        span: Span,
    ) -> Option<(NodeId, hir::ExpressionKind)> {
        let id = self.expect_node_id(r#if.id, span)?;
        let condition = self.lower_expression(&r#if.condition)?;

        let prev_scope = self.push_scope();
        let body = self.lower_block(&r#if.block, span)?;
        self.pop_scope(prev_scope);

        let body_id = body.id;
        self.push_block(body, r#if.block.span);

        Some((
            id,
            hir::ExpressionKind::If(hir::If {
                condition,
                body: body_id,
                else_arm: self.lower_if_arms(&r#if.else_branchs),
            }),
        ))
    }

    fn lower_if_arms(&mut self, arm: &Option<ast::IfArm>) -> Option<Box<hir::IfArm>> {
        let mut head = None;
        let mut tail = None;
        let mut current = arm.as_ref();

        while let Some(arm) = current {
            let hir_arm = match &arm.node {
                ast::ElseKind::ElseIf(elif) => {
                    let arm = self.lower_elseif(elif)?;
                    current = elif.node.else_branchs.as_ref();
                    arm
                }
                ast::ElseKind::Else(el) => {
                    let arm = self.lower_else(el)?;
                    current = None;
                    arm
                }
            };

            let tail_arm = match tail {
                Some(val) => val,
                None => {
                    head = Some(hir_arm);
                    tail = head.as_mut();
                    continue;
                }
            };

            tail = match &mut **tail_arm {
                hir::IfArm::Else(_) => {
                    self.log_error(SoulError::new(
                        "tryed to add ifArm after Else",
                        SoulErrorKind::InternalError,
                        Some(arm.get_span()),
                    ));
                    return None;
                }
                hir::IfArm::ElseIf(elif) => {
                    elif.else_arm = Some(hir_arm);
                    elif.else_arm.as_mut()
                }
            };
        }

        head
    }

    fn lower_elseif(&mut self, elif: &Box<Spanned<ast::If>>) -> Option<Box<hir::IfArm>> {
        let prev_body = self.push_scope();
        let body = self.lower_block(&elif.node.block, elif.get_span())?;
        self.pop_scope(prev_body);
        let body_id = body.id;
        self.push_block(body, elif.get_span());

        Some(Box::new(hir::IfArm::ElseIf(hir::If {
            body: body_id,
            else_arm: None,
            condition: self.lower_expression(&elif.node.condition)?,
        })))
    }

    fn lower_else(&mut self, el: &Spanned<ast::Block>) -> Option<Box<hir::IfArm>> {
        let prev_body = self.push_scope();
        let body = self.lower_block(&el.node, el.get_span())?;
        self.pop_scope(prev_body);
        let body_id = body.id;
        self.push_block(body, el.get_span());

        Some(Box::new(hir::IfArm::Else(body_id)))
    }
}
