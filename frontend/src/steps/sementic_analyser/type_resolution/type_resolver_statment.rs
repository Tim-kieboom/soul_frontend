use models::abstract_syntax_tree::{
    enum_like::UnionVariantKind, function::{Function, FunctionSignature}, objects::{ClassMember, Field}, soul_type::{GenericDeclare, GenericDeclareKind, NamedTupleType}, spanned::Spanned, statment::{Statement, StatementKind, UseBlock}
};

use crate::steps::sementic_analyser::type_resolution::type_resolver::TypeResolver;

impl<'a> TypeResolver<'a> {
    fn resolve_impl_block(&mut self, use_block: &mut UseBlock) {
        self.resolve_type(&mut use_block.ty);
        
        if let Some(ty) = &mut use_block.impl_trait {
            self.resolve_type(ty);
        }
        self.resolve_block(&mut use_block.block);
    }

    pub(super) fn resolve_statement(&mut self, statment: &mut Statement) {
        match &mut statment.node {
            StatementKind::Enum(_) => (),
            StatementKind::Variable(_) => (),
            StatementKind::Assignment(assignment) => {
                self.resolve_expression(&mut assignment.left);
                self.resolve_expression(&mut assignment.right);
            }
            StatementKind::Function(function) => {
                self.resolves_function(function);
            }
            StatementKind::Trait(obj) => for function_signature in &mut obj.methods {
                self.resolves_function_signature(&mut function_signature.node);
            },
            StatementKind::Class(obj) => {
                for Spanned { node: member, .. } in &mut obj.members {
                    match member {
                        ClassMember::Field(field) => self.resolve_field(field),
                        ClassMember::Method(function) => self.resolves_function(function),
                        ClassMember::ImplBlock(use_block) => self.resolve_impl_block(use_block),
                    };
                }
            }
            StatementKind::Union(obj) => for variant in &mut obj.variants {
                match &mut variant.node.field {
                    UnionVariantKind::Tuple(items) => for ty in items {self.resolve_type(ty)},
                    UnionVariantKind::NamedTuple(items) => for (_name, ty) in items {self.resolve_type(ty);},
                }
            }
            StatementKind::Expression(value) => self.resolve_expression(value),
            StatementKind::UseBlock(use_block) => self.resolve_block(&mut use_block.block),
            StatementKind::Struct(obj) => for field in &mut obj.fields {self.resolve_field(&mut field.node)}

            StatementKind::Import(_) => (), // maybe later track imports
            StatementKind::EndFile | StatementKind::CloseBlock => (),
        }
    }

    fn resolve_field(&mut self, field: &mut Field) {
        self.resolve_type(&mut field.ty);
        if let Some(expression) = &mut field.default_value {
            self.resolve_expression(expression);
        }
    }

    fn declare_parameters(&mut self, parameters: &mut NamedTupleType) {
        for (_name, ty, _node_id) in &mut parameters.types {
            self.resolve_type(ty);
        }
    }

    fn resolves_function(&mut self, function: &mut Function) {
        self.resolves_function_signature(&mut function.signature.node);
        self.resolve_block(&mut function.block);
    }

    fn resolves_function_signature(&mut self, signature: &mut FunctionSignature) {
        if let Some(callee) = &mut signature.callee {
            self.resolve_type(&mut callee.node.extention_type);
        }

        self.resolve_type(&mut signature.return_type);
        self.resolve_generic_declares(&mut signature.generics);
        self.declare_parameters(&mut signature.parameters);
    }

    fn resolve_generic_declares(&mut self, generics: &mut Vec<GenericDeclare>) {
        for generic in generics {
            match &mut generic.kind {
                GenericDeclareKind::Lifetime(_) => (),
                GenericDeclareKind::Type {
                    name: _,
                    traits,
                    default,
                } => {
                    for r#trait in traits {
                        self.resolve_type(r#trait);
                    }
                    if let Some(ty) = default {
                        self.resolve_type(ty);
                    }
                }
                GenericDeclareKind::Expression {
                    name: _,
                    for_type,
                    default,
                } => {
                    if let Some(ty) = for_type {
                        self.resolve_type(ty);
                    }
                    if let Some(expression) = default {
                        self.resolve_expression(expression);
                    }
                }
            }
        }
    }
}
