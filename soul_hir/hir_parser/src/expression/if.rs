use hir::BlockId;
use soul_utils::span::{ItemMetaData, Spanned};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_if(&mut self, id: hir::ExpressionId, r#if: &ast::If) -> hir::Expression {
        let condition = self.lower_expression(&r#if.condition);
        let then_block = self.lower_block(&r#if.block);

        let else_block = r#if
            .else_branchs
            .as_ref()
            .map(|arm| self.lower_if_arm(&arm.node));

        hir::Expression {
            id,
            ty: self.new_infer_type(),
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
        let if_expression = self.lower_if(id, &arm.node);

        let block_id = self.id_generator.alloc_body();
        let mut block = hir::Block::new(block_id);

        let expression_id = self.insert_expression(id, if_expression);

        let _ = self.alloc_statement(&ItemMetaData::default_const(), arm.span);
        let if_statement = hir::Statement::Expression {
            value: expression_id,
            ends_semicolon: false,
        };

        block.statements.push(if_statement);
        self.hir.blocks.insert(block_id, block);
        block_id
    }
}