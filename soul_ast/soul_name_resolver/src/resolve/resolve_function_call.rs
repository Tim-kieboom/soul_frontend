use ast::{
    Expression, ExpressionKind, FunctionCall, FunctionKind, SoulType, TypeKind, VarTypeKind,
};
use soul_utils::{
    error::{SoulError, SoulErrorKind}, ids::{FunctionId, IdAlloc}, soul_error_internal, soul_names::PrimitiveTypes, span::Span
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_function_call(&mut self, function_call: &mut FunctionCall, span: Span) {
        self.get_resolve(function_call);
        self.check_if_valid(function_call, span);

        for arg in &mut function_call.arguments {
            self.resolve_expression(&mut arg.value);
        }
    }

    fn check_if_valid(&mut self, function_call: &mut FunctionCall, span: Span) {
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

        let Some(function_id) = function_call.resolved else {
            return;
        };

        let Some((signature, _)) = self.store.get_function(function_id) else {
            return;
        };

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

    fn get_resolve(&mut self, function_call: &mut FunctionCall) {
        let callee_ident = match function_call.callee.as_ref().map(|f| &f.node) {
            Some(ExpressionKind::Variable { ident, .. }) => Some(ident.to_string()),
            _ => None,
        };

        let module_entry = callee_ident
            .as_ref()
            .and_then(|name| self.lookup_module(name));

        if module_entry.is_some() {
            let module_name = callee_ident.clone().unwrap_or_default();

            function_call.resolved = self.lookup_module_function(
                &module_name,
                function_call.name.as_str(),
                function_call.name.span,
            );

            if function_call.resolved.is_some() {
                function_call.callee = None;
            }

            return;
        }

        if function_call.resolved.is_some() {
            return;
        }

        let func_name = function_call.name.as_str();
        let mut has_module_with_this = false;
        let mut can_use_store = false;

        let Some(modules_iter) = self.info.scopes.iter_modules(self.current.module) else {
            self.log_error(soul_error_internal!(
                format!("{:?} not found", self.current.module),
                Some(function_call.name.span)
            ));
            return
        };

        for (_name, entry) in modules_iter {
            if let ast::ImportKind::Items { this, .. } = &entry.import_kind {
                if *this {
                    has_module_with_this = true;
                }
            }

            if matches!(entry.import_kind, ast::ImportKind::This) {
                has_module_with_this = true;
            }

            if matches!(entry.import_kind, ast::ImportKind::Module) {
                continue;
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
            return;
        }

        let type_qualifier = self.parse_owner_type(function_call.callee.as_deref());
        let is_type_qualifier = type_qualifier.is_some();

        if is_type_qualifier {
            function_call.callee = None;
        } else if let Some(callee) = &mut function_call.callee {
            self.resolve_expression(callee);
        }

        let owner_kind = self.get_owner_kind(&type_qualifier, function_call);

        function_call.resolved = self
            .store
            .find_function(function_call.name.as_str(), owner_kind);

        if function_call.resolved.is_none() {
            function_call.resolved = self.lookup_function(function_call.name.as_str());
        }
    }

    fn get_owner_kind(
        &'a self,
        type_qualifier: &'a Option<SoulType>,
        function_call: &mut FunctionCall,
    ) -> Option<&'a TypeKind> {
        if let Some(ty) = &type_qualifier {
            return Some(&ty.kind);
        };

        let callee = match &function_call.callee {
            Some(val) => val,
            None => return None,
        };

        match &callee.node {
            ExpressionKind::Variable {
                resolved: Some(node_id),
                ..
            } => match &self.store.get_variable_type(*node_id)?.0 {
                VarTypeKind::NonInveredType(soul_type) => Some(&soul_type.kind),
                VarTypeKind::InveredType(_) => self
                    .store
                    .get_variable_owner_hint(*node_id)
                    .map(|(ty, _mod)| ty),
            },
            _ => None,
        }
    }

    fn parse_owner_type(&mut self, callee: Option<&Expression>) -> Option<SoulType> {
        let callee = callee?;
        let ident = match &callee.node {
            ExpressionKind::Variable { ident, .. } => ident,
            _ => return None,
        };

        if let Some(primitive) = PrimitiveTypes::from_str(ident.as_str()) {
            return Some(SoulType::new(
                None,
                TypeKind::Primitive(primitive),
                ident.span,
            ));
        }

        if self.info.scopes.lookup_type(ident, self.current.module).is_some() {
            return Some(SoulType::new(
                None,
                TypeKind::Stub(ast::Stub {
                    name: ident.as_str().to_string(),
                    generics: vec![],
                }),
                ident.span,
            ));
        }

        if self.info.scopes.lookup_module(ident.as_str(), self.current.module).is_some() {
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
}
