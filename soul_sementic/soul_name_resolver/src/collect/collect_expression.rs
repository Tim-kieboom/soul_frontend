use parser_models::ast::{ElseKind, Expression, ExpressionGroup, ExpressionKind, If};
use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_expression(&mut self, expression: &mut Expression) {

        match &mut expression.node {

            ExpressionKind::If(r#if) => self.collect_if(r#if),
            ExpressionKind::While(r#while) => {
                if let Some(condition) = &mut r#while.condition {
                    self.collect_expression(condition);
                }
                self.collect_block(&mut r#while.block);
            }
            ExpressionKind::Index(index) => {
                self.collect_expression(&mut index.collection);
                self.collect_expression(&mut index.index);
            }
            ExpressionKind::Unary(unary) => {
                self.collect_expression(&mut unary.expression);
            }
            ExpressionKind::Block(block) => {
                self.collect_block(block);
            }
            ExpressionKind::Binary(binary) => {
                self.collect_expression(&mut binary.left);
                self.collect_expression(&mut binary.right);
            }
            ExpressionKind::Deref(spanned) => {
                self.collect_expression(spanned);
            }
            ExpressionKind::ReturnLike(return_like) => {
                if let Some(value) = &mut return_like.value {
                    self.collect_expression(value);
                } 
            }
            ExpressionKind::FieldAccess(field_access) => {
                self.collect_expression(&mut field_access.parent);
            }
            ExpressionKind::FunctionCall(function_call) => {
                for arg in &mut function_call.arguments {
                    self.collect_expression(arg);
                }
                if let Some(callee) = &mut function_call.callee {
                    self.collect_expression(callee);
                }
            }
            ExpressionKind::Ref { expression, .. } => {
                self.collect_expression(expression);
            }
            ExpressionKind::ExpressionGroup(expression_group) => {
                self.collect_expression_group(expression_group);
            }
            ExpressionKind::StructConstructor(struct_constructor) => {
                for (_name, item) in &mut struct_constructor.named_tuple.values {
                    self.collect_expression(item);
                }
            }
            ExpressionKind::TypeNamespace(_) => todo!("typeNamespace"),
            ExpressionKind::ExternalExpression(_) => todo!("impl external expressions"),

            ExpressionKind::Empty 
            | ExpressionKind::Default 
            | ExpressionKind::Literal(_) 
            | ExpressionKind::Variable { .. } => (),
        }
    }

    fn collect_expression_group(&mut self, group: &mut ExpressionGroup) {
        match group {
            ExpressionGroup::Array(array) => {
                for item in &mut array.values {
                    self.collect_expression(item);
                }
            }
            ExpressionGroup::Tuple(spanneds) => {
                for item in spanneds {
                    self.collect_expression(item);
                }
            }
            ExpressionGroup::NamedTuple(named_tuple) => {
                for (_name, item) in &mut named_tuple.values {
                    self.collect_expression(item);
                }
            }
        }
    }

    fn collect_if(&mut self, r#if: &mut If) {
        self.collect_expression(&mut r#if.condition);
        self.collect_block(&mut r#if.block);

        let mut current = r#if.else_branchs.as_mut();
            while let Some(branch) = current {
            match &mut branch.node {
                ElseKind::Else(el) => {
                    self.collect_block(&mut el.node);
                    current = None;
                }
                ElseKind::ElseIf(el) => {
                    self.collect_expression(&mut el.node.condition);
                    self.collect_block(&mut el.node.block);
                    debug_assert!(el.node.else_branchs.is_none());
                    current = el.node.else_branchs.as_mut();
                }
            }
        }
    }   
}