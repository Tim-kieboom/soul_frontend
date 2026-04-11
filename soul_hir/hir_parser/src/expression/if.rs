use hir::BlockId;
use soul_utils::span::{ItemMetaData, Span, Spanned};

use crate::HirContext;

impl<'a> HirContext<'a> {
    pub(super) fn lower_if(
        &mut self,
        id: hir::ExpressionId,
        ast_if: &ast::If,
        span: Span,
    ) -> hir::Expression {
        let mut is_else = false;
        self.inner_lower_if(id, ast_if, &mut is_else, span)
    }

    fn inner_lower_if(
        &mut self,
        id: hir::ExpressionId,
        ast_if: &ast::If,
        is_else: &mut bool,
        span: Span,
    ) -> hir::Expression {
        let condition = self.lower_expression(&ast_if.condition);
        let then_block = self.lower_block(&ast_if.block);

        let else_block = ast_if
            .else_branchs
            .as_ref()
            .map(|arm| self.lower_if_arm(&arm.node, is_else, span));

        hir::Expression {
            id,
            ty: self.new_infer_type(vec![], None, span),
            kind: hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
                ends_with_else: *is_else,
            },
        }
    }

    fn lower_if_arm(&mut self, arm: &ast::ElseKind, is_else: &mut bool, span: Span) -> BlockId {
        match arm {
            ast::ElseKind::Else(block) => {
                *is_else = true;
                self.lower_block(&block.node)
            }
            ast::ElseKind::ElseIf(if_expr) => {
                *is_else = false;
                self.lower_else_if(if_expr, is_else, span)
            }
        }
    }

    fn lower_else_if(&mut self, arm: &Spanned<ast::If>, is_else: &mut bool, span: Span) -> BlockId {
        let id = self.alloc_expression(arm.span);
        let if_expression = self.inner_lower_if(id, &arm.node, is_else, span);

        let block_id = self.id_generator.alloc_body();
        let block = hir::Block::new(block_id);
        self.insert_block(block_id, block, arm.span);

        let expression_id = self.insert_expression(id, if_expression);

        let _ = self.alloc_statement(&ItemMetaData::default_const(), arm.span);
        let kind = hir::StatementKind::Expression {
            value: expression_id,
            ends_semicolon: false,
        };

        let id = self.alloc_statement(&ItemMetaData::default_const(), arm.span);
        let if_statement = hir::Statement::new(kind, id);

        self.insert_in_block(block_id, if_statement);
        self.insert_block_terminator(block_id, hir::Terminator::Expression(expression_id));
        block_id
    }
}
