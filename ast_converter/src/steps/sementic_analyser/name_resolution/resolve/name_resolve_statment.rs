use soul_ast::{
    abstract_syntax_tree::{
        block::Block,
        enum_like::EnumVariantsKind,
        function::Function,
        objects::ClassMember,
        spanned::Spanned,
        statment::{Ident, Statement, StatementKind, UseBlock},
    },
    sementic_models::scope::NodeId,
};
use soul_utils::{SoulError, SoulErrorKind, Span};

use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_block(&mut self, block: &mut Block) {
        self.try_go_to(block.scope_id);

        for statment in &mut block.statements {
            self.resolve_statement(statment);
        }
    }

    fn resolve_impl_block(&mut self, use_block: &mut UseBlock) {
        if let Some(impl_trait) = &mut use_block.impl_trait {
            self.insert_trait_impl(impl_trait.clone(), use_block.ty.clone());
        }
        self.resolve_block(&mut use_block.block);
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
            StatementKind::Enum(obj) => {
                for variant in &mut obj.variants {
                    match &mut variant.value {
                        EnumVariantsKind::Int(_) => (),
                        EnumVariantsKind::Expression(expression) => {
                            self.resolve_expression(expression)
                        }
                    }
                }
            }
            StatementKind::Class(obj) => {
                self.try_go_to(obj.scope_id);
                for Spanned { node: member, .. } in &mut obj.members {
                    match member {
                        ClassMember::Field(field) => {
                            if let Some(expression) = &mut field.default_value {
                                self.resolve_expression(expression);
                            }
                        }
                        ClassMember::Method(function) => self.resolves_function(function),
                        ClassMember::ImplBlock(use_block) => self.resolve_impl_block(use_block),
                    };
                }
            }
            StatementKind::Struct(obj) => {
                self.try_go_to(obj.scope_id);
                for Spanned { node: field, .. } in &mut obj.fields {
                    if let Some(expression) = &mut field.default_value {
                        self.resolve_expression(expression);
                    }
                }
            }

            StatementKind::Expression(value) => self.resolve_expression(value),
            StatementKind::UseBlock(use_block) => self.resolve_block(&mut use_block.block),

            StatementKind::Import(_) => (), // maybe later track imports

            StatementKind::Trait(_)
            | StatementKind::EndFile
            | StatementKind::Union(_)
            | StatementKind::CloseBlock => (),
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
        ident: &Ident,
        resolved: &mut Option<NodeId>,
        span: Span,
    ) {
        match self.info.scopes.lookup_variable(ident) {
            Some(id) => *resolved = Some(id),
            None => self.log_error(SoulError::new(
                format!("variable '{}' is undefined in scope", ident.as_str()),
                SoulErrorKind::ScopeError,
                Some(span),
            )),
        }
    }
}
