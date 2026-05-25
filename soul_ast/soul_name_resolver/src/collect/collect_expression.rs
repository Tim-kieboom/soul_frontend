use crate::NameResolver;
use ast::scope::ScopeValue;
use ast::{AnyArray, ElseKind, Expression, ExpressionKind, If};
use soul_utils::error::{SoulError, SoulErrorKind};

impl<'a> NameResolver<'a> {
    pub(super) fn collect_expression(&mut self, expression: &mut Expression) {
        match &mut expression.node {
            ExpressionKind::Match(match_expression) => {
                match_expression.id = Some(self.alloc_node());
                self.collect_expression(&mut match_expression.scrutinee);
                for arm in &mut match_expression.arms {
                    self.push_scope(&mut arm.body.scope_id);
                    self.collect_match_pattern(&mut arm.pattern);
                    self.collect_scopeless_block(&mut arm.body);
                    self.pop_scope();
                }
            }
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

                let prev = self.current.in_global;
                self.current.in_global = false;
                self.collect_block(block);
                self.current.in_global = prev;
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
            ExpressionKind::New(expr) => {
                self.collect_expression(expr);
            }
            ExpressionKind::NewArray(array) => match array {
                AnyArray::ArrayLiteral(arr) => {
                    if let Some(ty) = arr.collection_type.as_mut() {
                        self.collect_type(ty)
                    }
                    if let Some(ty) = arr.element_type.as_mut() {
                        self.collect_type(ty)
                    }
                    for value in &mut arr.values {
                        self.collect_expression(value);
                    }
                }
                AnyArray::ArrayConstructor(arr) => {
                    if let Some(ty) = arr.collection_type.as_mut() {
                        self.collect_type(ty)
                    }
                    if let Some(ty) = arr.element_type.as_mut() {
                        self.collect_type(ty)
                    }
                    self.collect_expression(&mut arr.amount);
                    self.collect_expression(&mut arr.element);
                }
            },
            ExpressionKind::ExternalExpression(_) => todo!("impl external expressions"),
            ExpressionKind::Default(id) => *id = Some(self.alloc_node()),
            ExpressionKind::Literal((id, _)) => *id = Some(self.alloc_node()),
            ExpressionKind::TypeOf {
                expr,
                binding,
                binding_id,
                type_name: _,
                variant_name: _,
            } => {
                self.collect_expression(expr);
                if let Some(ident) = binding {
                    if !self.current.in_if_condition {
                        self.log_error(SoulError::new(
                            "typeof with binding can only be used as an if condition".to_string(),
                            SoulErrorKind::InvalidContext,
                            Some(ident.span),
                        ));
                        return;
                    }
                    let id = self.alloc_node();
                    *binding_id = Some(id);
                    if self
                        .insert_value(ident.as_str(), id, ScopeValue::Variable)
                        .is_some()
                    {
                        self.log_error(SoulError::new(
                            format!("name {} already exists in scope", ident.as_str()),
                            SoulErrorKind::AlreadyFoundInScope,
                            Some(ident.span),
                        ));
                    }
                }
            }
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
            ExpressionKind::MatchMethod(mm) => {
                mm.id = Some(self.alloc_node());
                self.collect_expression(&mut mm.expr);
                for arm in &mut mm.arms {
                    self.push_scope(&mut arm.body.scope_id);
                    if let Some((binding_ident, binding_id)) = &mut arm.binding {
                        let id = self.alloc_node();
                        *binding_id = Some(id);
                        if self
                            .insert_value(binding_ident.as_str(), id, ScopeValue::Variable)
                            .is_some()
                        {
                            self.log_error(SoulError::new(
                                format!("name {} already exists in scope", binding_ident.as_str()),
                                SoulErrorKind::AlreadyFoundInScope,
                                Some(binding_ident.span),
                            ));
                        }
                    }
                    self.collect_scopeless_block(&mut arm.body);
                    self.pop_scope();
                }
            }
        }
    }

    fn collect_if(&mut self, r#if: &mut If) {
        self.push_scope(&mut r#if.block.scope_id);
        self.current.in_if_condition = true;
        self.collect_expression(&mut r#if.condition);
        self.current.in_if_condition = false;
        self.collect_scopeless_block(&mut r#if.block);
        self.pop_scope();

        let mut current = r#if.else_branchs.as_mut();

        while let Some(branch) = current {
            match &mut branch.node {
                ElseKind::Else(el) => {
                    self.collect_block(&mut el.node);
                    current = None;
                }
                ElseKind::ElseIf(el) => {
                    let elif = &mut el.node;
                    self.push_scope(&mut elif.block.scope_id);
                    self.current.in_if_condition = true;
                    self.collect_expression(&mut elif.condition);
                    self.current.in_if_condition = false;
                    self.collect_scopeless_block(&mut r#if.block);
                    self.pop_scope();

                    current = el.node.else_branchs.as_mut();
                }
            }
        }
    }

    fn collect_match_pattern(&mut self, pattern: &mut ast::MatchPattern) {
        match pattern {
            ast::MatchPattern::Array(elements) => {
                for elem in elements {
                    self.collect_match_pattern(elem);
                }
            }
            ast::MatchPattern::Binding { ident, id } => {
                let node_id = self.alloc_node();
                *id = Some(node_id);
                if self
                    .insert_value(ident.as_str(), node_id, ScopeValue::Variable)
                    .is_some()
                {
                    self.log_error(SoulError::new(
                        format!("name {} already exists in scope", ident.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(ident.span),
                    ));
                }
            }
            ast::MatchPattern::Constructor {
                binding,
                binding_id,
                ..
            } => {
                if let Some(ident) = binding {
                    let node_id = self.alloc_node();
                    *binding_id = Some(node_id);
                    if self
                        .insert_value(ident.as_str(), node_id, ScopeValue::Variable)
                        .is_some()
                    {
                        self.log_error(SoulError::new(
                            format!("name {} already exists in scope", ident.as_str()),
                            SoulErrorKind::AlreadyFoundInScope,
                            Some(ident.span),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}
