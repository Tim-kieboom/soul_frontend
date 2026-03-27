use hir::BlockId;
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    span::Span,
    symbool_kind::SymbolKind,
};

use crate::{CurrentBody, HirContext, Scope};

impl<'a> HirContext<'a> {
    pub(crate) fn lower_block(&mut self, body: &ast::Block) -> hir::BlockId {
        let id = self.id_generator.alloc_body();

        let prev_body = self.current_body;
        self.current_body = CurrentBody::Block(id);
        self.push_scope();

        let block = hir::Block {
            id,
            statements: vec![],
            terminator: None,
            imports: vec![],
        };
        self.insert_block(id, block, body.span);

        let mut last_expression = None;

        for statement in &body.statements {
            use hir::StatementKind::Expression;

            let hir_statement = match self.lower_statement(statement) {
                Some(val) => val,
                None => continue,
            };

            if let Expression {
                value,
                ends_semicolon,
            } = &hir_statement.kind
            {
                if let Some((_value, ends_semicolon, span)) = last_expression {
                    if ends_semicolon {
                        self.log_error(SoulError::new(
                            format!("'{}' at the end of a line can only be used for expressions at the end of a block", SymbolKind::SemiColon.as_str()), 
                            SoulErrorKind::InvalidEscapeSequence,
                            Some(span),
                        ));
                    }
                }

                last_expression = Some((*value, *ends_semicolon, statement.span));
            }

            self.insert_in_block(id, hir_statement);
        }

        self.pop_scope();
        self.current_body = prev_body;

        match last_expression {
            Some((value, ends_semicolon, _span)) if !ends_semicolon => {
                self.insert_block_terminator(id, value);
            }
            _ => (),
        };

        id
    }

    pub(crate) fn insert_block(&mut self, id: BlockId, block: hir::Block, span: Span) {
        self.tree.nodes.blocks.insert(id, block);
        self.tree.info.spans.blocks.insert(id, span);
    }

    pub(crate) fn insert_in_block(&mut self, id: BlockId, statement: hir::Statement) {
        self.tree.nodes.blocks[id].statements.push(statement);
    }

    pub(crate) fn insert_block_terminator(&mut self, id: BlockId, terminator: hir::ExpressionId) {
        self.tree.nodes.blocks[id].terminator = Some(terminator)
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub(super) fn pop_scope(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }
}
