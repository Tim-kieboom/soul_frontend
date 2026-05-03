use ast::{Block, Function, Statement, StatementKind, UseBlock, scope::NodeId};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    span::Span,
};

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
            StatementKind::UseBlock(UseBlock {
                use_type: _,
                generics: _,
                impls,
                methodes,
            }) => {
                for methode in methodes {
                    self.resolves_function(methode);
                }

                if !impls.is_empty() {
                    todo!()
                }
            }
            StatementKind::Struct(obj) => {
                Self::resolve_struct(self.context, self.store, &self.current, obj);
            }
            StatementKind::Enum(obj) => {
                Self::resolve_enum(self.context, self.store, &self.current, obj);
            }
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
            StatementKind::Expression {
                id: _,
                expression,
                ends_semicolon: _,
            } => self.resolve_expression(expression),
            StatementKind::Import(_) => (), // maybe later track imports
            StatementKind::ExternalFunction(_) => (), // maybe later track imports
        }
    }

    fn resolves_function(&mut self, function: &mut Function) {
        let prev = self.current.function;
        self.current.function = function.signature.node.id;

        self.try_go_to(function.block.scope_id);
        self.resolve_block(&mut function.block);
        self.current.function = prev;
    }

    pub(super) fn resolve_variable(
        &mut self,
        name: &Ident,
        resolved: &mut Option<NodeId>,
        span: Span,
    ) {
        match self.check_variable(name) {
            Some(id) => *resolved = Some(id),
            None => self.log_error(SoulError::new(
                format!("variable '{}' is undefined in scope", name.as_str()),
                SoulErrorKind::NotFoundInScope,
                Some(span),
            )),
        }
    }
}
