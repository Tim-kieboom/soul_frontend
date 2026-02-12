use crate::{CurrentBody, HirContext};
use hir::Assign;
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_names::KeyWord,
    symbool_kind::SymbolKind,
};

pub(crate) mod function;
pub(crate) mod variable;

impl<'a> HirContext<'a> {
    pub(crate) fn lower_global(&mut self, global: &ast::Statement) {
        let id = self.alloc_statement(&global.meta_data, global.span);

        let hir_global = match &global.node {
            ast::StatementKind::Variable(variable) => {
                let hir_variable = self.lower_variable(variable);
                hir::Global::Variable(hir_variable, id)
            }
            ast::StatementKind::Function(function) => {
                let hir_function = self.lower_function(function);
                hir::Global::Function(hir_function, id)
            }
            ast::StatementKind::Import(import) => {
                self.resolve_import(import);
                return;
            }

            ast::StatementKind::Assignment(_) | ast::StatementKind::Expression { .. } => {
                self.log_error(SoulError::new(
                    format!(
                        "{} statement is not allowed as a global",
                        global.node.variant_name()
                    ),
                    SoulErrorKind::InvalidContext,
                    Some(global.span),
                ));
                return;
            }
        };

        self.insert_global(hir_global);
    }

    pub(crate) fn lower_statement(&mut self, statement: &ast::Statement) -> Option<hir::Statement> {
        let id = self.alloc_statement(&statement.meta_data, statement.span);

        let hir_statement = match &statement.node {
            ast::StatementKind::Import(import) => {
                self.resolve_import(import);
                return None;
            }
            ast::StatementKind::Function(function) => {
                let hir_function = self.lower_function(function);
                self.insert_global(hir::Global::Function(hir_function, id));
                return None;
            }
            ast::StatementKind::Variable(variable) => {
                let hir_variable = self.lower_variable(variable);
                hir::Statement::Variable(hir_variable)
            }
            ast::StatementKind::Assignment(assignment) => hir::Statement::Assign(Assign {
                place: self.lower_place(&assignment.left),
                value: self.lower_expression(&assignment.right),
            }),
            ast::StatementKind::Expression {
                id: _,
                expression,
                ends_semicolon,
            } => {
                use ast::ExpressionKind::ReturnLike;

                if let ReturnLike(return_like) = &expression.node {
                    self.lower_return_like(return_like)
                } else {
                    hir::Statement::Expression {
                        value: self.lower_expression(expression),
                        ends_semicolon: *ends_semicolon,
                    }
                }
            }
        };

        Some(hir_statement)
    }

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
        self.hir.blocks.insert(id, block);

        let mut last_expression = None;

        for statement in &body.statements {
            let hir_statement = match self.lower_statement(statement) {
                Some(val) => val,
                None => continue,
            };

            if let hir::Statement::Expression {
                value,
                ends_semicolon,
            } = &hir_statement
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

            self.hir.blocks[id].statements.push(hir_statement);
        }

        self.pop_scope();
        self.current_body = prev_body;

        self.hir.blocks[id].terminator = match last_expression {
            Some((value, ends_semicolon, _span)) if !ends_semicolon => Some(value),
            _ => None,
        };

        id
    }

    fn lower_return_like(&mut self, return_like: &ast::ReturnLike) -> hir::Statement {
        let value = match &return_like.value {
            Some(val) => Some(self.lower_expression(val)),
            None => None,
        };

        match return_like.kind {
            ast::ReturnKind::Return => hir::Statement::Return(value),
            ast::ReturnKind::Break => hir::Statement::Break(value),
            ast::ReturnKind::Continue => {
                if let Some(value) = &return_like.value {
                    self.log_error(SoulError::new(
                        format!("{} can not contain expression", KeyWord::Continue.as_str()),
                        SoulErrorKind::InvalidContext,
                        Some(value.span),
                    ));
                }
                hir::Statement::Continue
            }
        }
    }
}
