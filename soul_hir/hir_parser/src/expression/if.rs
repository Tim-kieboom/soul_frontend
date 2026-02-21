use hir::BlockId;
use soul_utils::span::{ItemMetaData, Span, Spanned};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_if(&mut self, id: hir::ExpressionId, r#if: &ast::If, span: Span) -> hir::Expression {
        let condition = self.lower_expression(&r#if.condition);
        let then_block = self.lower_block(&r#if.block);

        let else_block = r#if
            .else_branchs
            .as_ref()
            .map(|arm| self.lower_if_arm(&arm.node));

        hir::Expression {
            id,
            ty: self.new_infer_type(span),
            kind: hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
            },
        }
    }

    fn lower_if_arm(&mut self, arm: &ast::ElseKind) -> BlockId {
        match arm {
            ast::ElseKind::Else(block) => self.lower_block(&block.node),
            ast::ElseKind::ElseIf(if_expr) => self.lower_else_if(&*if_expr),
        }
    }

    fn lower_else_if(&mut self, arm: &Spanned<ast::If>) -> BlockId {
        let id = self.alloc_expression(arm.span);
        let if_expression = self.lower_if(id, &arm.node, arm.span);

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
