use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;
use models::{
    abstract_syntax_tree::{
        block::Block,
        enum_like::EnumVariantsKind,
        function::Function,
        objects::ClassMember,
        soul_type::GenericDeclare,
        spanned::Spanned,
        statment::{Statement, StatementKind},
    },
    sementic_models::scope::{ScopeTypeKind, ScopeValueKind},
};

impl<'a> NameResolver<'a> {
    pub(super) fn collect_block(&mut self, block: &mut Block) {
        self.push_scope(&mut block.scope_id);

        for statement in &mut block.statments {
            self.collect_statement(statement);
        }

        self.pop_scope();
    }

    fn collect_scopeless_block(&mut self, block: &mut Block) {
        for statement in &mut block.statments {
            self.collect_statement(statement);
        }
    }

    fn collect_statement(&mut self, statment: &mut Statement) {
        match &mut statment.node {
            StatementKind::Import(_) => todo!("impl import trait collection"),
            StatementKind::Variable(variable) => {
                let _ = self.declare_value(ScopeValueKind::Variable(variable));
                if let Some(value) = &mut variable.initialize_value {
                    self.collect_expression(value);
                }
            }
            StatementKind::Function(function) => {
                self.collect_function(function);
            }
            StatementKind::UseBlock(use_block) => {
                self.collect_block(&mut use_block.block);
            }
            StatementKind::Class(obj) => {
                self.declare_type(ScopeTypeKind::Class(obj), statment.span);

                self.push_scope(&mut obj.scope_id);
                self.collect_generic_declares(&mut obj.generics);
                for Spanned { node: member, .. } in &mut obj.members {
                    match member {
                        ClassMember::Field(field) => {
                            self.declare_value(ScopeValueKind::Field(field));
                            if let Some(expression) = &mut field.default_value {
                                self.collect_expression(expression);
                            }
                        }
                        ClassMember::Method(function) => self.collect_function(function),
                        ClassMember::ImplBlock(use_block) => {
                            self.collect_block(&mut use_block.block)
                        }
                    };
                }
                self.pop_scope();
            }
            StatementKind::Union(obj) => {
                self.declare_type(ScopeTypeKind::Union(obj), statment.span);
                let trait_id = obj.node_id.expect("just assigned node_id in declare_type");

                self.push_scope(&mut obj.scope_id);
                self.collect_generic_declares(&mut obj.generics);
                for variant in &mut obj.variants {
                    self.declare_type(
                        ScopeTypeKind::UnionVariant {
                            ty: &mut variant.node,
                            trait_id,
                        },
                        variant.span,
                    );
                }
                self.pop_scope();
            }
            StatementKind::Struct(obj) => {
                self.declare_type(ScopeTypeKind::Struct(obj), statment.span);

                self.push_scope(&mut obj.scope_id);
                self.collect_generic_declares(&mut obj.generics);
                for field in &mut obj.fields {
                    self.declare_value(ScopeValueKind::Field(&mut field.node));
                    if let Some(value) = &mut field.node.default_value {
                        self.collect_expression(value);
                    }
                }
                self.pop_scope();
            }
            StatementKind::Enum(obj) => {
                self.declare_type(ScopeTypeKind::Enum(obj), statment.span);

                for variant in &mut obj.variants {
                    match &mut variant.value {
                        EnumVariantsKind::Int(_) => (),
                        EnumVariantsKind::Expression(spanned) => self.collect_expression(spanned),
                    }
                }
            }
            StatementKind::Trait(obj) => {
                self.push_scope(&mut obj.scope_id);
                self.collect_generic_declares(&mut obj.signature.generics);
                self.declare_type(ScopeTypeKind::Trait(obj), statment.span);
                self.pop_scope();
            }
            StatementKind::Expression(expression) => self.collect_expression(expression),
            StatementKind::Assignment(assignment) => {
                self.collect_expression(&mut assignment.left);
                self.collect_expression(&mut assignment.right);
            }

            StatementKind::EndFile | StatementKind::CloseBlock => (),
        }
    }

    fn collect_function(&mut self, function: &mut Function) {
        let id = self.declare_value(ScopeValueKind::Function(function));
        let prev = self.current_function;
        self.current_function = Some(id);

        self.push_scope(&mut function.block.scope_id);
        self.collect_generic_declares(&mut function.signature.node.generics);
        self.declare_parameters(&mut function.signature.node.parameters);
        self.collect_scopeless_block(&mut function.block);
        self.pop_scope();

        self.current_function = prev;
    }

    fn collect_generic_declares(&mut self, generics: &mut Vec<GenericDeclare>) {
        for generic in generics {
            let span = generic.span;
            self.declare_type(ScopeTypeKind::GenricDeclare(generic), span);
        }
    }
}
