use crate::NameResolver;
use ast::{ElseKind, Expression, ExpressionKind, If};

impl<'a> NameResolver<'a> {
    pub(super) fn collect_expression(&mut self, expression: &mut Expression) {
        match &mut expression.node {
            ExpressionKind::Sizeof(ty) => self.collect_type(ty),
            ExpressionKind::ArrayContructor(ctor) => {
                ctor.id = Some(self.alloc_node());
                if let Some(ty) = ctor.collection_type.as_mut() {
                    self.collect_type(ty)
                }
                if let Some(ty) = ctor.element_type.as_mut() {
                    self.collect_type(ty)
                }
                self.collect_expression(&mut ctor.amount);
                self.collect_expression(&mut ctor.element);
            }
            ExpressionKind::FieldAccess(field) => {
                self.collect_expression(&mut field.object);
            }
            ExpressionKind::StructConstructor(ctor) => {
                self.collect_type(&mut ctor.struct_type);
                for (_, value) in &mut ctor.values {
                    self.collect_expression(value);
                }
            }
            ExpressionKind::Null(node_id) => {
                *node_id = Some(self.alloc_node());
            }
            ExpressionKind::As(type_cast) => {
                type_cast.id = Some(self.alloc_node());
                self.collect_expression(&mut type_cast.left);
                self.collect_type(&mut type_cast.type_cast);
            }
            ExpressionKind::If(r#if) => {
                r#if.id = Some(self.alloc_node());
                self.collect_if(r#if);
            }
            ExpressionKind::While(r#while) => {
                r#while.id = Some(self.alloc_node());
                if let Some(condition) = &mut r#while.condition {
                    self.collect_expression(condition);
                }
                self.collect_block(&mut r#while.block);
            }
            ExpressionKind::Index(index) => {
                index.id = Some(self.alloc_node());
                self.collect_expression(&mut index.collection);
                self.collect_expression(&mut index.index);
            }
            ExpressionKind::Unary(unary) => {
                unary.id = Some(self.alloc_node());
                self.collect_expression(&mut unary.expression);
            }
            ExpressionKind::Block(block) => {
                block.node_id = Some(self.alloc_node());
                self.collect_block(block);
            }
            ExpressionKind::Binary(binary) => {
                binary.id = Some(self.alloc_node());
                self.collect_expression(&mut binary.left);
                self.collect_expression(&mut binary.right);
            }
            ExpressionKind::Deref { inner, id } => {
                *id = Some(self.alloc_node());
                self.collect_expression(inner);
            }
            ExpressionKind::ReturnLike(return_like) => {
                return_like.id = Some(self.alloc_node());
                if let Some(value) = &mut return_like.value {
                    self.collect_expression(value);
                }
            }
            ExpressionKind::FunctionCall(function_call) => {
                function_call.id = Some(self.alloc_node());
                for arg in &mut function_call.arguments {
                    self.collect_expression(&mut arg.value);
                }
                if let Some(callee) = &mut function_call.callee {
                    self.collect_expression(callee);
                }
            }
            ExpressionKind::Ref { expression, id, .. } => {
                *id = Some(self.alloc_node());
                self.collect_expression(expression);
            }
            ExpressionKind::ExternalExpression(_) => todo!("impl external expressions"),
            ExpressionKind::Default(id) => *id = Some(self.alloc_node()),
            ExpressionKind::Literal((id, _)) => *id = Some(self.alloc_node()),
            ExpressionKind::Variable { id, .. } => {
                *id = Some(self.alloc_node());
            }
            ExpressionKind::Array(array) => {
                array.id = Some(self.alloc_node());
                if let Some(ty) = array.collection_type.as_mut() {
                    self.collect_type(ty)
                }

                if let Some(ty) = array.element_type.as_mut() {
                    self.collect_type(ty)
                }

                for value in &mut array.values {
                    self.collect_expression(value);
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
                    current = el.node.else_branchs.as_mut();
                }
            }
        }
    }
}
