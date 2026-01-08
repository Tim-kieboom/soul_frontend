use parser_models::{ast::{Block, Function, Statement, StatementKind}, scope::{NodeId, ScopeValueEntryKind}};
use soul_utils::{Ident, error::{SoulError, SoulErrorKind}, span::Span};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub fn resolve_block(&mut self, block: &mut Block) {
        self.try_go_to(block.scope_id);

        for statement in &mut block.statements {
            self.resolve_statement(statement);
        }
    }

    fn resolve_statement(&mut self, statment: &mut Statement) {
        match &mut statment.node {
            StatementKind::Variable(variable) => {
                if let Some(value) = &mut variable.initialize_value {
                    self.resolve_expression(value);
                }
            }
            StatementKind::Assignment(assignment) => {
                self.resolve_expression(&mut assignment.left);
                self.resolve_expression(&mut assignment.right);
            }
            StatementKind::Function(function) => {
                self.resolves_function(function);
            }
            StatementKind::Expression(value) => self.resolve_expression(value),
            StatementKind::Import(_) => (), // maybe later track imports
        }
    }

    fn resolves_function(&mut self, function: &mut Function) {
        let prev = self.current_function;
        self.current_function = function.node_id;

        self.try_go_to(function.block.scope_id);
        self.resolve_block(&mut function.block);
        self.current_function = prev;
    }

    pub(super) fn resolve_variable(
        &mut self,
        name: &Ident,
        resolved: &mut Option<NodeId>,
        span: Span,
    ) {
        match self.info.scopes.lookup_value(name, ScopeValueEntryKind::Variable) {
            Some(id) => *resolved = Some(id),
            None => self.log_error(SoulError::new(
                format!("variable '{}' is undefined in scope", name.as_str()),
                SoulErrorKind::NotFoundInScope,
                Some(span),
            )),
        }
    }
}