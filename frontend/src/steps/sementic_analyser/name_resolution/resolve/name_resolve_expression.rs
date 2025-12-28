use soul_ast::{
    abstract_syntax_tree::{
        conditionals::{ForPattern, IfCaseKind},
        expression::{Expression, ExpressionKind},
        expression_groups::ExpressionGroup,
        function::LamdbaBodyKind,
    },
    error::{SoulError, SoulErrorKind},
};

use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_expression(&mut self, expression: &mut Expression) {
        match &mut expression.node {
            ExpressionKind::Variable { ident, resolved } => {
                self.resolve_variable(ident, resolved, expression.span)
            }
            ExpressionKind::FunctionCall(function_call) => {
                function_call.candidates = self.lookup_function_candidates(&function_call.name);

                if function_call.candidates.is_empty() {
                    self.log_error(SoulError::new(
                        format!(
                            "function '{}' is undefined in scope",
                            function_call.name.as_str()
                        ),
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
            }
            ExpressionKind::StructConstructor(struct_constructor) => {
                for (_name, value) in &mut struct_constructor.arguments.values {
                    self.resolve_expression(value);
                }
            }
            ExpressionKind::FieldAccess(field_access) => {
                self.resolve_expression(&mut field_access.object);
            }
            ExpressionKind::If(r#if) => {
                self.resolve_expression(&mut r#if.condition);
                self.resolve_block(&mut r#if.block);
            }
            ExpressionKind::For(r#for) => {
                self.resolve_expression(&mut r#for.collection);
                self.resolve_block(&mut r#for.block);
                if let Some(el) = &mut r#for.element {
                    self.resolve_for_pattern(el);
                }
            }
            ExpressionKind::While(r#while) => {
                if let Some(value) = &mut r#while.condition {
                    self.resolve_expression(value);
                }
                self.resolve_block(&mut r#while.block);
            }
            ExpressionKind::Match(r#match) => {
                self.resolve_expression(&mut r#match.condition);
                for case in &mut r#match.cases {
                    match &mut case.if_kind {
                        IfCaseKind::WildCard(_) => (),
                        IfCaseKind::Expression(spanned) => self.resolve_expression(spanned),
                        IfCaseKind::Variant { params, .. } => {
                            for value in params {
                                self.resolve_variable(
                                    &value.name,
                                    &mut value.node_id,
                                    expression.span,
                                );
                            }
                        }
                        IfCaseKind::NamedVariant { params, .. } => {
                            for (_name, value) in params {
                                self.resolve_variable(
                                    &value.name,
                                    &mut value.node_id,
                                    expression.span,
                                );
                            }
                        }
                        IfCaseKind::Bind { condition, .. } => self.resolve_expression(condition),
                    }
                }
            }
            ExpressionKind::Ternary(ternary) => {
                self.resolve_expression(&mut ternary.condition);
                self.resolve_expression(&mut ternary.else_branch);
                self.resolve_expression(&mut ternary.if_branch);
            }
            ExpressionKind::Deref(spanned) => self.resolve_expression(spanned),
            ExpressionKind::Ref { expression, .. } => self.resolve_expression(expression),
            ExpressionKind::ExpressionGroup(expression_group) => match expression_group {
                ExpressionGroup::Tuple(tuple) => {
                    for value in &mut tuple.values {
                        self.resolve_expression(value);
                    }
                }
                ExpressionGroup::Array(array) => {
                    for value in &mut array.values {
                        self.resolve_expression(value);
                    }
                }
                ExpressionGroup::NamedTuple(named_tuple) => {
                    for (_name, value) in &mut named_tuple.values {
                        self.resolve_expression(value);
                    }
                }
                ExpressionGroup::ArrayFiller(array_filler) => {
                    self.resolve_expression(&mut array_filler.amount);
                    self.resolve_expression(&mut array_filler.fill_expr);
                    if let Some(variable) = &mut array_filler.index {
                        self.resolve_variable(
                            &variable.name,
                            &mut variable.node_id,
                            expression.span,
                        );
                    }
                }
            },

            ExpressionKind::Empty
            | ExpressionKind::Default
            | ExpressionKind::Literal(_)
            | ExpressionKind::StaticFieldAccess(_)
            | ExpressionKind::ExternalExpression(_) => (),
        }
    }

    fn resolve_for_pattern(&mut self, forpattern: &mut ForPattern) {
        match forpattern {
            ForPattern::Ident {
                ident,
                resolved,
                span,
            } => self.resolve_variable(ident, resolved, *span),
            ForPattern::Tuple(items) => {
                for value in items {
                    self.resolve_for_pattern(value);
                }
            }
            ForPattern::NamedTuple(items) => {
                for (_name, value) in items {
                    self.resolve_for_pattern(value);
                }
            }
        }
    }
}
