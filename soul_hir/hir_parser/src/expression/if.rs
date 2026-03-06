use hir::BlockId;
use soul_utils::span::{ItemMetaData, Span, Spanned};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_if(
        &mut self,
        id: hir::ExpressionId,
        r#if: &ast::If,
        span: Span,
    ) -> hir::Expression {
        let mut is_else = false;
        self.inner_lower_if(id, r#if, span, &mut is_else)
    }

    fn inner_lower_if(
        &mut self,
        id: hir::ExpressionId,
        r#if: &ast::If,
        span: Span,
        is_else: &mut bool,
    ) -> hir::Expression {
        let condition = self.lower_expression(&r#if.condition);
        let then_block = self.lower_block(&r#if.block);

        let else_block = r#if
            .else_branchs
            .as_ref()
            .map(|arm| self.lower_if_arm(&arm.node, is_else));

        hir::Expression {
            id,
            ty: self.new_infer_type(span),
            kind: hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
                ends_with_else: *is_else,
            },
        }
    }

    fn lower_if_arm(&mut self, arm: &ast::ElseKind, is_else: &mut bool) -> BlockId {
        match arm {
            ast::ElseKind::Else(block) => {
                *is_else = true;
                self.lower_block(&block.node)
            }
            ast::ElseKind::ElseIf(if_expr) => {
                *is_else = false;
                self.lower_else_if(&*if_expr, is_else)
            }
        }
    }

    fn lower_else_if(&mut self, arm: &Spanned<ast::If>, is_else: &mut bool) -> BlockId {
        let id = self.alloc_expression(arm.span);
        let if_expression = self.inner_lower_if(id, &arm.node, arm.span, is_else);

        let block_id = self.id_generator.alloc_body();
        let block = hir::Block::new(block_id);
        self.insert_block(block_id, block, arm.span);

        let expression_id = self.insert_expression(id, if_expression);

        let _ = self.alloc_statement(&ItemMetaData::default_const(), arm.span);
        let if_statement = hir::Statement::Expression {
            id: self.alloc_statement(&ItemMetaData::default_const(), arm.span),
            value: expression_id,
            ends_semicolon: false,
        };

        self.insert_in_block(block_id, if_statement);
        self.insert_block_terminator(block_id, expression_id);
        block_id
    }
}
