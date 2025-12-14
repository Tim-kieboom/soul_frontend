use models::{
    abstract_syntax_tree::{
        AbstractSyntaxTree, block::Block, conditionals::{ForPattern, IfCaseKind}, expression::{Expression, ExpressionKind}, expression_groups::ExpressionGroup, function::LamdbaBodyKind, soul_type::{SoulType, TypeKind}, statment::{Ident, Statement, StatementKind}
    },
    error::{SoulError, SoulErrorKind, Span},
    sementic_models::scope::{NodeId, ScopeTypeEntryKind, ScopeTypeKind, ScopeValueKind}, soul_names::StackArrayKind,
};

use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;

impl NameResolver {
    pub fn resolve_ast(&mut self, ast: &mut AbstractSyntaxTree) {
        self.resolve_block(&mut ast.root);
    }

    fn resolve_block(&mut self, block: &mut Block) {
        self.push_scope();

        for statment in &mut block.statments {
            self.resolve_statement(statment);
        }

        self.pop_scope();
    }

    fn resolve_statement(&mut self, statment: &mut Statement) {
        match &mut statment.node {
            StatementKind::Variable(variable) => {
                let _id = self.declare_value(ScopeValueKind::Variable(variable));

                if let Some(value) = &mut variable.initialize_value {
                    self.resolve_expression(value);
                }
            },
            StatementKind::Assignment(assignment) => {
                self.resolve_expression(&mut assignment.left);
                self.resolve_expression(&mut assignment.right);
            },
            StatementKind::Function(function) => {
                let id = self.declare_value(ScopeValueKind::Funtion(function));

                let prev = self.current_function;
                self.current_function = Some(id);

                self.push_scope();

                self.declare_parameters(&mut function.signature.parameters);
                self.resolve_block(&mut function.block);

                self.pop_scope();
                self.current_function = prev;
            },

            StatementKind::Enum(obj) => {
                self.declare_type(ScopeTypeKind::Enum(obj), statment.span)
            },
            StatementKind::Trait(obj) => {
                self.declare_type(ScopeTypeKind::Trait(obj), statment.span)
            },
            StatementKind::Class(obj) => {
                self.declare_type(ScopeTypeKind::Class(obj), statment.span)
            },
            StatementKind::Union(obj) => {
                self.declare_type(ScopeTypeKind::Union(obj), statment.span)
            },
            StatementKind::Struct(obj) => {
                self.declare_type(ScopeTypeKind::Struct(obj), statment.span)
            },

            StatementKind::Expression(value) => self.resolve_expression(value),
            StatementKind::UseBlock(use_block) => self.resolve_block(&mut use_block.block),

            StatementKind::Import(_) => (), // maybe later track imports
            StatementKind::EndFile | StatementKind::CloseBlock => (),
        }
    }

    fn resolve_expression(&mut self, expression: &mut Expression) {
        match &mut expression.node {
            ExpressionKind::Variable { ident, resolved } => self.resolve_variable(ident, resolved, expression.span),
            ExpressionKind::FunctionCall(function_call) => {
                function_call.candidates = self.lookup_function_candidates(&function_call.name);

                if function_call.candidates.is_empty() {
                    self.log_error(SoulError::new(
                        format!("function '{}' is undefined in scope", function_call.name),
                        SoulErrorKind::ScopeError,
                        Some(expression.span),
                    ))
                }

                for argument in &mut function_call.arguments.values {
                    self.resolve_expression(argument);
                }
            }
            ExpressionKind::Block(block) => self.resolve_block(block),
            ExpressionKind::Binary(binary) => {
                self.resolve_expression(&mut binary.left);
                self.resolve_expression(&mut binary.right);
            }
            ExpressionKind::Unary(unary) => self.resolve_expression(&mut unary.expression),
            ExpressionKind::ReturnLike(return_like) => {
                if self.current_function.is_none() {
                    let keyword_str = return_like.kind.as_keyword().as_str();

                    self.log_error(SoulError::new(
                        format!("{keyword_str} can not be called while outside of function"),
                        SoulErrorKind::ScopeError,
                        Some(expression.span),
                    ));
                }

                if let Some(value) = &mut return_like.value {
                    self.resolve_expression(value);
                }
            }
            ExpressionKind::Index(index) => {
                self.resolve_expression(&mut index.collection);
                self.resolve_expression(&mut index.index);
            }
            ExpressionKind::Lambda(lambda) => {
                
                for argument in &mut lambda.arguments.values {
                    self.resolve_expression(argument);
                }

                match &mut lambda.body {
                    LamdbaBodyKind::Block(block) => self.resolve_block(block),
                    LamdbaBodyKind::Expression(value) => self.resolve_expression(value),
                }
            },
            ExpressionKind::StructConstructor(struct_constructor) => {
                for (_name, value) in &mut struct_constructor.arguments.values {
                    self.resolve_expression(value);
                }
            },
            ExpressionKind::FieldAccess(field_access) => {
                self.resolve_expression(&mut field_access.object);
            },
            ExpressionKind::If(r#if) => {
                self.resolve_expression(&mut r#if.condition);
                self.resolve_block(&mut r#if.block);
            },
            ExpressionKind::For(r#for) => {
                self.resolve_expression(&mut r#for.collection);
                self.resolve_block(&mut r#for.block);
                if let Some(el) = &mut r#for.element {
                    self.resolve_for_pattern(el);
                }
            },
            ExpressionKind::While(r#while) => {
                if let Some(value) = &mut r#while.condition {
                    self.resolve_expression(value);
                }
                self.resolve_block(&mut r#while.block);
            },
            ExpressionKind::Match(r#match) => {
                self.resolve_expression(&mut r#match.condition);
                for case in &mut r#match.cases {
                    match &mut case.if_kind {
                        IfCaseKind::WildCard(var) => if let Some(variable) = var {
                            let _id = self.declare_value(ScopeValueKind::Variable(variable));
                        },
                        IfCaseKind::Expression(spanned) => self.resolve_expression(spanned),
                        IfCaseKind::Variant{params, ..} => for value in &mut params.values {
                            self.resolve_expression(value);
                        },
                        IfCaseKind::NamedVariant{params, ..} => for (_name, value) in &mut params.values {
                            self.resolve_expression(value);
                        },
                        IfCaseKind::Bind{condition, ..} => self.resolve_expression(condition),
                    }
                }
            },
            ExpressionKind::Ternary(ternary) => {
                self.resolve_expression(&mut ternary.condition);
                self.resolve_expression(&mut ternary.else_branch);
                self.resolve_expression(&mut ternary.if_branch);
            },
            ExpressionKind::Deref(spanned) => self.resolve_expression(spanned),
            ExpressionKind::Ref{expression, ..} => self.resolve_expression(expression),
            ExpressionKind::ExpressionGroup(expression_group) => {

                match expression_group {
                    ExpressionGroup::Tuple(tuple) => for value in &mut tuple.values {
                        self.resolve_expression(value);
                    },
                    ExpressionGroup::Array(array) => for value in &mut array.values {
                        self.resolve_expression(value);
                    },
                    ExpressionGroup::NamedTuple(named_tuple) => for (_name, value) in &mut named_tuple.values {
                        self.resolve_expression(value);
                    },
                    ExpressionGroup::ArrayFiller(array_filler) => {
                        self.resolve_expression(&mut array_filler.amount);
                        self.resolve_expression(&mut array_filler.fill_expr);
                        if let Some(variable) = &mut array_filler.index {
                            self.resolve_variable(&variable.name, &mut variable.node_id, expression.span);
                        }
                    },
                }
            },
            
            ExpressionKind::Empty 
            | ExpressionKind::Default 
            | ExpressionKind::Literal(_)
            | ExpressionKind::StaticFieldAccess(_) 
            | ExpressionKind::ExternalExpression(_) => (),
        }
    }

    fn resolve_type(&mut self, ty: &mut SoulType, span: Span) {

        match &mut ty.kind {
            TypeKind::Stub{ident, ..} => {
                if let Some(entry) = self.lookup_type(&ident) {
                    match entry.kind {
                        ScopeTypeEntryKind::Struct => *ty = SoulType::new(ty.modifier, TypeKind::Struct(entry.node_id), span),
                        ScopeTypeEntryKind::Class => *ty = SoulType::new(ty.modifier, TypeKind::Class(entry.node_id), span),
                        ScopeTypeEntryKind::Trait => *ty = SoulType::new(ty.modifier, TypeKind::Trait(entry.node_id), span),
                        ScopeTypeEntryKind::Union => *ty = SoulType::new(ty.modifier, TypeKind::Union(entry.node_id), span),
                        ScopeTypeEntryKind::Enum => *ty = SoulType::new(ty.modifier, TypeKind::Enum(entry.node_id), span),
                    }
                } else {
                    self.log_error(SoulError::new(
                        format!("type '{ident}' not found"),
                        SoulErrorKind::ScopeError,
                        Some(ty.span)
                    ));
                }
            },
            TypeKind::Generic{ident, resolved} => {
                if let Some(entry) = self.lookup_type(&ident) {
                    *resolved = Some(entry.node_id);
                } else {
                    self.log_error(SoulError::new(
                        format!("generic type '{ident}' not found"),
                        SoulErrorKind::ScopeError,
                        Some(ty.span)
                    ));
                }
            },
            TypeKind::Enum(_) |
            TypeKind::Class(_) |
            TypeKind::Trait(_) |
            TypeKind::Union(_) |
            TypeKind::Struct(_) => (),
            
            TypeKind::Array(array_type) => {
                self.resolve_type(&mut array_type.of_type, span);
                match &mut array_type.size {
                    Some(StackArrayKind::Number(_)) => (),
                    Some(StackArrayKind::Ident{ident, resolved}) => {
                        if let Some(entry) = self.lookup_type(&ident) {
                            *resolved = Some(entry.node_id);
                        } else {
                            self.resolve_variable(ident, resolved, ty.span);
                        }
                    },
                    None => (),
                }
            },
            TypeKind::Tuple(tuple_type) => for ty in &mut tuple_type.types {
                self.resolve_type(ty, span);
            },
            TypeKind::Pointer(soul_type) => self.resolve_type(soul_type, span),
            TypeKind::Optional(soul_type) => self.resolve_type(soul_type, span),
            TypeKind::Function(function_type) => {
                for item in &mut function_type.parameters.types {
                    self.resolve_type(item, span);
                }
                self.resolve_type(&mut function_type.return_type, span);
            },
            TypeKind::Reference(reference_type) => self.resolve_type(&mut reference_type.inner, span),
            TypeKind::NamedTuple(named_tuple_type) => for (_name, ty, _node_id) in &mut named_tuple_type.types {
                self.resolve_type(ty, span);
            },
            
            TypeKind::None |
            TypeKind::Type |
            TypeKind::Primitive(_) |
            TypeKind::InternalComplex(_) => (),
        }
    }

    fn resolve_variable(&mut self, ident: &Ident, resolved: &mut Option<NodeId>, span: Span) {
        match self.lookup_variable(ident) {
            Some(id) => *resolved = Some(id),
            None => self.log_error(SoulError::new(
                format!("variable '{}' is undefined in scope", ident),
                SoulErrorKind::ScopeError,
                Some(span),
            )),
        }
    }

    fn resolve_for_pattern(&mut self, forpattern: &mut ForPattern) {
        match forpattern {
            ForPattern::Ident{ident, resolved, span} => self.resolve_variable(ident, resolved, *span),
            ForPattern::Tuple(items) => {
                for value in items {
                    self.resolve_for_pattern(value);
                }
            },
            ForPattern::NamedTuple(items) => {
                for (_name, value) in items {
                    self.resolve_for_pattern(value);
                }
            },
        }
    }
}
