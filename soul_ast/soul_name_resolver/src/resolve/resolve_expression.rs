use ast::{
    DeclareStore, Expression, ExpressionKind, FunctionKind, SoulType, TypeKind, VarTypeKind,
};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc},
    soul_names::PrimitiveTypes,
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_expression(&mut self, expression: &mut Expression) {
        let span = expression.span;
        match &mut expression.node {
            ExpressionKind::Sizeof(_) => (),
            ExpressionKind::ArrayContructor(ctor) => {
                self.resolve_expression(&mut ctor.amount);
                self.resolve_expression(&mut ctor.element);
            }
            ExpressionKind::FieldAccess(_) => (),
            ExpressionKind::StructConstructor(ctor) => {
                for (_, value) in &mut ctor.values {
                    self.resolve_expression(value);
                }
            }
            ExpressionKind::As(type_cast) => {
                self.resolve_expression(&mut type_cast.left);
            }
            ExpressionKind::Index(index) => {
                self.resolve_expression(&mut index.collection);
                self.resolve_expression(&mut index.index);
            }
            ExpressionKind::FunctionCall(function_call) => {
                let callee_ident = function_call.callee.as_ref().and_then(|c| {
                    if let ExpressionKind::Variable { ident, .. } = &c.node {
                        Some(ident.as_str().to_string())
                    } else {
                        None
                    }
                });

                let module_entry = callee_ident
                    .as_ref()
                    .and_then(|name| self.lookup_module(name));

                if module_entry.is_some() {
                    let module_name = callee_ident.clone().unwrap_or_default();
                    let is_allowed = self.is_item_imported(
                        &module_name,
                        function_call.name.as_str(),
                    );
                    if is_allowed {
                        function_call.resolved = self.lookup_module_function(
                            &module_name,
                            self.root,
                            function_call.name.as_str(),
                        );
                    }
                    if function_call.resolved.is_some() {
                        function_call.callee = None;
                    }
                } else {

                    // If still not resolved, try the store lookup
                    if function_call.resolved.is_none() {
                        let func_name = function_call.name.as_str();
                        
                        let mut has_module_with_this = false;
                        let mut can_use_store = false;
                        
                        for (_name, entry) in self.info.scopes.modules() {
                            if let ast::ImportKind::Items { this, .. } = &entry.import_kind {
                                if *this {
                                    has_module_with_this = true;
                                }
                            }
                            
                            if matches!(entry.import_kind, ast::ImportKind::This) {
                                has_module_with_this = true;
                            }
                            
                            for item in &entry.imported_items {
                                match item {
                                    ast::ImportItem::Normal(ident) => {
                                        if ident.as_str() == func_name {
                                            can_use_store = true;
                                        }
                                    }
                                    ast::ImportItem::Alias { name: _, alias } => {
                                        if alias.as_str() == func_name {
                                            can_use_store = true;
                                        }
                                    }
                                }
                            }
                        }
                        
                        if has_module_with_this && !can_use_store {
                            function_call.resolved = self.lookup_function(function_call.name.as_str());
                        } else {
                            let type_qualifier =
                                parse_owner_type(self, function_call.callee.as_deref());
                            let is_type_qualifier = type_qualifier.is_some();

                            if is_type_qualifier {
                                function_call.callee = None;
                            } else if let Some(callee) = &mut function_call.callee {
                                self.resolve_expression(callee);
                            }

                            let owner_kind = type_qualifier.as_ref().map(|t| &t.kind).or_else(|| {
                                function_call.callee.as_ref().and_then(|c| {
                                    receiver_type_kind_for_instance_method(self.store, c.as_ref())
                                })
                            });

                            function_call.resolved = self.store.find_function_by_name_and_owner_kind(
                                function_call.name.as_str(),
                                owner_kind,
                            );
                            if function_call.resolved.is_none() {
                                function_call.resolved = self.lookup_function(function_call.name.as_str());
                            }
                            if function_call.resolved.is_none() && type_qualifier.is_some() {
                                function_call.resolved = self.lookup_function(function_call.name.as_str());
                            }
                        }
                    }
                }

                if function_call.resolved.is_none() {
                    self.log_error(SoulError::new(
                        format!(
                            "function '{}' is undefined in scope",
                            function_call.name.as_str(),
                        ),
                        SoulErrorKind::NotFoundInScope,
                        Some(span),
                    ));

                    function_call.resolved = Some(FunctionId::error());
                };

                if let Some(function_id) = function_call.resolved
                    && let Some((signature, _module)) = self.store.get_function(function_id)
                {
                    let func_name = signature.name.as_str().to_string();
                    let needs_callee = !matches!(signature.function_kind, FunctionKind::Static);

                    let has_callee = function_call.callee.is_some();
                    if has_callee && !needs_callee {
                        self.log_error(SoulError::new(
                            format!(
                                "function '{}' is static and can not be called on an instance",
                                func_name,
                            ),
                            SoulErrorKind::InvalidContext,
                            Some(span),
                        ));
                    } else if !has_callee && needs_callee {
                        self.log_error(SoulError::new(
                            format!(
                                "method '{}' requires a receiver (this/@this/&this)",
                                func_name,
                            ),
                            SoulErrorKind::InvalidContext,
                            Some(span),
                        ));
                    }
                }

                for arg in &mut function_call.arguments {
                    self.resolve_expression(&mut arg.value);
                }
            }
            ExpressionKind::Variable {
                id: _,
                ident,
                resolved,
            } => {
                self.resolve_variable(ident, resolved, span);
            }
            ExpressionKind::Unary(unary) => {
                self.resolve_expression(&mut unary.expression);
            }
            ExpressionKind::Binary(binary) => {
                self.resolve_expression(&mut binary.left);
                self.resolve_expression(&mut binary.right);
            }
            ExpressionKind::If(r#if) => {
                self.resolve_expression(&mut r#if.condition);
                self.resolve_block(&mut r#if.block);
            }
            ExpressionKind::While(r#while) => {
                if let Some(value) = &mut r#while.condition {
                    self.resolve_expression(value);
                }
                self.resolve_block(&mut r#while.block);
            }
            ExpressionKind::Deref { id: _, inner } => {
                self.resolve_expression(inner);
            }
            ExpressionKind::Ref { expression, .. } => {
                self.resolve_expression(expression);
            }
            ExpressionKind::Block(block) => {
                self.resolve_block(block);
            }
            ExpressionKind::ReturnLike(return_like) => {
                if let Some(value) = &mut return_like.value {
                    self.resolve_expression(value);
                }
            }

            ExpressionKind::Array(array) => {
                for value in &mut array.values {
                    self.resolve_expression(value);
                }
            }

            ExpressionKind::Null(_)
            | ExpressionKind::Default(_)
            | ExpressionKind::Literal(_)
            | ExpressionKind::ExternalExpression(_) => (),
        }
    }
}

fn receiver_type_kind_for_instance_method<'a>(
    store: &'a DeclareStore,
    receiver: &Expression,
) -> Option<&'a TypeKind> {
    match &receiver.node {
        ExpressionKind::Variable {
            resolved: Some(node_id),
            ..
        } => {
            match &store.get_variable_type(*node_id)?.0 {
                VarTypeKind::NonInveredType(soul_type) => Some(&soul_type.kind),
                VarTypeKind::InveredType(_) => store.get_variable_owner_hint(*node_id).map(|(ty, _mod)| ty),
            }
        }
        _ => None,
    }
}

fn parse_owner_type(
    resolver: &mut NameResolver<'_>,
    callee: Option<&Expression>,
) -> Option<SoulType> {
    let callee = callee?;
    match &callee.node {
        ExpressionKind::Variable { ident, .. } => {
            if let Some(primitive) = PrimitiveTypes::from_str(ident.as_str()) {
                return Some(SoulType::new(
                    None,
                    TypeKind::Primitive(primitive),
                    ident.span,
                ));
            }

            if resolver.info.scopes.lookup_type(ident).is_some() {
                return Some(SoulType::new(
                    None,
                    TypeKind::Stub(ast::Stub {
                        name: ident.as_str().to_string(),
                        generics: vec![],
                    }),
                    ident.span,
                ));
            }

            if resolver.info.scopes.lookup_module(ident.as_str()).is_some() {
                return Some(SoulType::new(
                    None,
                    TypeKind::Stub(ast::Stub {
                        name: ident.as_str().to_string(),
                        generics: vec![],
                    }),
                    ident.span,
                ));
            }

            None
        }
        _ => None,
    }
}
