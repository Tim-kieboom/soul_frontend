use models::{
    abstract_syntax_tree::{
        block::Block,
        function::Function,
        objects::ClassMember,
        spanned::Spanned,
        statment::{Ident, Statement, StatementKind, UseBlock},
    },
    error::{SoulError, SoulErrorKind, Span},
    sementic_models::scope::{NodeId, ScopeTypeKind, ScopeValueKind},
};

use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;

impl NameResolver {
    pub(super) fn resolve_block(&mut self, block: &mut Block) {
        self.push_scope();

        for statment in &mut block.statments {
            self.resolve_statement(statment);
        }

        self.pop_scope();
    }

    fn resolve_impl_block(&mut self, _block: &mut UseBlock) {
        todo!()
    }

    fn resolve_statement(&mut self, statment: &mut Statement) {
        match &mut statment.node {
            StatementKind::Variable(variable) => {
                let _id = self.declare_value(ScopeValueKind::Variable(variable));
                self.resolve_type(&mut variable.ty, statment.span);

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
            StatementKind::Enum(obj) => self.declare_type(ScopeTypeKind::Enum(obj), statment.span),
            StatementKind::Trait(obj) => {
                self.resolve_generic_declares(&mut obj.signature.generics);
                self.declare_type(ScopeTypeKind::Trait(obj), statment.span);
            }
            StatementKind::Class(obj) => {
                self.push_scope();
                self.resolve_generic_declares(&mut obj.generics);
                for Spanned { node: member, .. } in &mut obj.members {
                    match member {
                        ClassMember::Field(field) => {
                            _ = self.declare_value(ScopeValueKind::Field(field))
                        }
                        ClassMember::Method(function) => self.resolves_function(function),
                        ClassMember::ImplBlock(use_block) => self.resolve_impl_block(use_block),
                    };
                }

                self.declare_type(ScopeTypeKind::Class(obj), statment.span);
                self.pop_scope();
            }
            StatementKind::Union(obj) => {
                self.resolve_generic_declares(&mut obj.generics);
                self.declare_type(ScopeTypeKind::Union(obj), statment.span);
            }
            StatementKind::Struct(obj) => {
                self.push_scope();
                self.resolve_generic_declares(&mut obj.generics);
                for Spanned { node: field, .. } in &mut obj.fields {
                    self.declare_value(ScopeValueKind::Field(field));
                }

                self.declare_type(ScopeTypeKind::Struct(obj), statment.span);
                self.pop_scope();
            }

            StatementKind::Expression(value) => self.resolve_expression(value),
            StatementKind::UseBlock(use_block) => self.resolve_block(&mut use_block.block),

            StatementKind::Import(_) => (), // maybe later track imports
            StatementKind::EndFile | StatementKind::CloseBlock => (),
        }
    }

    fn resolves_function(&mut self, function: &mut Function) {
        let id = self.declare_value(ScopeValueKind::Function(function));
        let signature = &mut function.signature;

        let prev = self.current_function;
        self.current_function = Some(id);

        self.push_scope();
        self.resolve_generic_declares(&mut signature.node.generics);
        self.resolve_type(&mut signature.node.return_type, signature.span);

        self.declare_parameters(&mut signature.node.parameters, signature.span);
        self.resolve_block(&mut function.block);

        self.pop_scope();
        self.current_function = prev;
    }

    pub(super) fn resolve_variable(
        &mut self,
        ident: &Ident,
        resolved: &mut Option<NodeId>,
        span: Span,
    ) {
        match self.lookup_variable(ident) {
            Some(id) => *resolved = Some(id),
            None => self.log_error(SoulError::new(
                format!("variable '{}' is undefined in scope", ident),
                SoulErrorKind::ScopeError,
                Some(span),
            )),
        }
    }
}
