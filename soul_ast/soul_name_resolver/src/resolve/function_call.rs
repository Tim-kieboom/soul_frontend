use ast_model::{
    declare_store::{FunctionResolve, IntrinsicResolve},
    expression::{
        ExpressionId, ExpressionKind, FieldAccess, FunctionCall, FunctionCallee,
        FunctionCalleeKind, VariableExpression,
    },
    scope::{ScopeModuleEntry, ScopeTypeEntryKind, ScopeValue},
    soul_type::{Generic, SoulType, Stub},
    statements::{ImportItem, ImportKind},
};
use soul_utils::soul_names::PrimitiveTypes;
use soul_utils::{
    FunctionId,
    fault::Fault,
    intrinsics::IntrinsicFunction,
    soul_error_internal,
    span::{ModuleId, Span},
};
use std::str::FromStr;

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_function_call(
        &mut self,
        expression_id: ExpressionId,
        call: &FunctionCall,
    ) {
        for argument in &call.arguments {
            self.resolve_expression(argument.value);
        }

        if let Some(path) = self.try_get_intrinsic_path(call) {
            self.resolve_intrinsic_call(&path, call);
            return;
        }

        self.resolve_call(expression_id, call);
    }

    fn finish_call_resolution(
        &mut self,
        expression_id: ExpressionId,
        call: &FunctionCall,
        function_id: FunctionId,
    ) {
        if function_id == FunctionId::ERROR {
            return;
        }
        let Some((signature, _)) = self.declares.get_function(function_id) else {
            return;
        };
        let return_type = signature.return_type.clone();
        let parameters = signature.parameters.clone();
        let generics = signature.generics.clone();

        // Only checkable when arity matches positionally and no argument is named.
        let checkable = call.arguments.len() == parameters.len()
            && call.arguments.iter().all(|arg| arg.name.is_none());

        if checkable {
            let mut generic_bindings: Vec<(&str, SoulType)> = Vec::new();

            for (argument, parameter) in call.arguments.iter().zip(&parameters) {
                if matches!(parameter.ty, SoulType::ImplTrait(_)) {
                    continue;
                }

                let Some(arg_ty) = self.expression_type(argument.value) else {
                    continue;
                };

                let span = self
                    .store
                    .expressions
                    .get(argument.value)
                    .map(|expr| expr.span)
                    .unwrap_or(call.name.span());

                if let Some(generic_name) = generic_name_of(&parameter.ty, &generics) {
                    match generic_bindings
                        .iter()
                        .find(|(name, _)| *name == generic_name)
                    {
                        Some((_, bound_ty)) => {
                            if self.combine_operand_types(&arg_ty, bound_ty).is_none() {
                                self.log_fault(Fault::error(
                                    format!(
                                        "generic parameter `{generic_name}` inferred as both `{bound_ty:?}` and `{arg_ty:?}`"
                                    ),
                                    Some(span),
                                ));
                            }
                        }
                        None => generic_bindings.push((generic_name, arg_ty)),
                    }
                    continue;
                }

                if self.combine_operand_types(&arg_ty, &parameter.ty).is_some() {
                    continue;
                }

                self.log_fault(Fault::error(
                    format!(
                        "argument type mismatch: expected `{:?}`, got `{arg_ty:?}`",
                        parameter.ty
                    ),
                    Some(span),
                ));
            }
        }

        self.declares
            .insert_expression_type(expression_id, return_type);
    }

    fn resolve_variable_callable(&mut self, expression_id: ExpressionId, name: &str) -> bool {
        let Some(var_id) =
            self.scope_info
                .scopes
                .lookup_value(name, ScopeValue::Variable, self.current.module)
        else {
            return false;
        };
        let Some((_, Some(SoulType::Function { return_type, .. }), _)) =
            self.declares.get_variable_type(var_id)
        else {
            return false;
        };
        self.declares
            .insert_expression_type(expression_id, (**return_type).clone());
        true
    }

    fn try_get_intrinsic_path(&mut self, call: &FunctionCall) -> Option<String> {
        let callee = call.callee.as_ref()?;
        let value = match &callee.kind {
            FunctionCalleeKind::Type(_) => return None,
            FunctionCalleeKind::Expression(id) => *id,
        };

        let mut segments = self.collect_intrinsic_path_segments(value)?;
        segments.push(call.name.to_string());
        Some(segments.join("."))
    }

    fn collect_intrinsic_path_segments(&mut self, id: ExpressionId) -> Option<Vec<String>> {
        let expr = self.store.expressions.get(id)?;
        match &expr.node {
            ExpressionKind::Variable(VariableExpression { name, .. })
                if name.as_str() == "intrinsic" =>
            {
                Some(Vec::new())
            }
            ExpressionKind::FieldAccess(field_access) => {
                let mut segments = self.collect_intrinsic_path_segments(field_access.object)?;
                segments.push(field_access.field.to_string());
                Some(segments)
            }
            _ => None,
        }
    }

    fn resolve_intrinsic_call(&mut self, path: &str, call: &FunctionCall) {
        let Ok(kind) = IntrinsicFunction::from_str(path) else {
            self.log_fault(Fault::error(
                format!("unknown intrinsic 'intrinsic.{path}'"),
                Some(call.name.span()),
            ));
            return;
        };

        if call.arguments.len() != kind.arity() {
            self.log_fault(Fault::error(
                format!(
                    "'intrinsic.{path}' expects {} argument(s), got {}",
                    kind.arity(),
                    call.arguments.len()
                ),
                Some(call.name.span()),
            ));
        }

        self.declares
            .insert_intrinsic_resolve(call.id, IntrinsicResolve { kind });
    }

    fn resolve_call(&mut self, expression_id: ExpressionId, call: &FunctionCall) {
        let module_entry = match &self.try_get_callee_string(call) {
            Some(string) => self.lookup_module(string),
            None => None,
        };

        if let Some(module) = module_entry {
            self.resolve_module_call(expression_id, module, call);
            return;
        }

        if !self.is_function_imported(call) {
            let name = call.name.as_str();
            match self.lookup_function(name) {
                Some(id) => {
                    self.declares.insert_function_resolve(
                        call.id,
                        FunctionResolve {
                            id,
                            is_defer: false,
                            ignore_callee: false,
                        },
                    );
                    self.finish_call_resolution(expression_id, call, id);
                }
                None if self.resolve_variable_callable(expression_id, name) => {}
                None => {
                    self.declares.insert_function_resolve(
                        call.id,
                        FunctionResolve {
                            id: FunctionId::ERROR,
                            is_defer: false,
                            ignore_callee: false,
                        },
                    );
                }
            }
            return;
        }

        let type_qualifier = self.parse_owner_type(call.callee.as_ref());
        let ignore_callee = type_qualifier.is_some();
        if let Some(callee) = &call.callee {
            let is_union = self.is_callee_union(callee).unwrap_or(false);
            if type_qualifier.is_some() && is_union {
                self.declares.insert_function_resolve(
                    call.id,
                    FunctionResolve {
                        id: FunctionId::ERROR,
                        is_defer: false,
                        ignore_callee,
                    },
                );
                return;
            }

            match &callee.kind {
                FunctionCalleeKind::Type(_) => (),
                FunctionCalleeKind::Expression(id) => {
                    if !ignore_callee {
                        self.resolve_expression(*id);
                    }
                }
            }
        }

        let name = call.name.as_str();
        let owner_type = self.get_owner_kind(type_qualifier.as_ref(), call);
        let has_owner_type = owner_type.is_some();
        let resolved = if has_owner_type {
            self.declares.find_function(name, owner_type)
        } else {
            self.lookup_function(name)
        };

        let Some(id) = resolved else {
            if !has_owner_type {
                self.resolve_variable_callable(expression_id, name);
            }
            return;
        };
        self.declares.insert_function_resolve(
            call.id,
            FunctionResolve {
                id,
                is_defer: false,
                ignore_callee: false,
            },
        );
        self.finish_call_resolution(expression_id, call, id);
    }

    fn get_owner_kind(
        &'a self,
        type_qualifier: Option<&'a SoulType>,
        call: &'a FunctionCall,
    ) -> Option<&'a SoulType> {
        if let Some(ty) = type_qualifier {
            return Some(ty);
        }

        let callee = call.callee.as_ref()?;
        let value = match &callee.kind {
            FunctionCalleeKind::Type(soul_type) => return Some(soul_type),
            FunctionCalleeKind::Expression(id) => self.store.expressions.get(*id)?,
        };

        match &value.node {
            ExpressionKind::Variable(VariableExpression { id, .. }) => {
                let resolved = self.declares.get_variable_resolve(*id)?;
                match self.declares.get_variable_type(resolved)? {
                    (_, Some(ty), _) => Some(ty),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn is_callee_union(&self, callee: &FunctionCallee) -> Option<bool> {
        let ident = match &callee.kind {
            FunctionCalleeKind::Expression(id) => match &self.store.expressions.get(*id)?.node {
                ExpressionKind::Variable(VariableExpression { name, .. }) => name.as_str(),
                _ => return None,
            },
            FunctionCalleeKind::Type(soul_type) => match soul_type {
                SoulType::Stub(stub) => &stub.name,
                SoulType::Res { .. } => return Some(true),
                _ => return None,
            },
        };

        let entry = self
            .scope_info
            .scopes
            .lookup_type(ident, self.current.module)?;
        Some(matches!(entry.kind, ScopeTypeEntryKind::Union))
    }

    fn is_function_imported(&mut self, call: &FunctionCall) -> bool {
        let function_name = call.name.as_str();
        let mut has_module_with_this = false;
        let mut matches_imported_item = false;

        let Some(modules) = self.scope_info.scopes.iter_modules(self.current.module) else {
            self.log_fault(soul_error_internal!(
                format!("{:?} not found", self.current.module),
                Some(call.name.span())
            ));
            return false;
        };

        for (_name, entry) in modules {
            match &entry.import_kind {
                ImportKind::Module => continue,
                ImportKind::This => has_module_with_this = true,
                ImportKind::Items { has_this, .. } if *has_this => {
                    has_module_with_this = true;
                }
                _ => (),
            }

            for item in &entry.imported_items {
                match item {
                    ImportItem::Normal(ident) => {
                        if ident.as_str() == function_name {
                            matches_imported_item = true;
                        }
                    }
                    ImportItem::Alias { alias, .. } => {
                        if alias.as_str() == function_name {
                            matches_imported_item = true;
                        }
                    }
                }
            }
        }

        !has_module_with_this || matches_imported_item
    }

    fn parse_owner_type(&mut self, callee: Option<&FunctionCallee>) -> Option<SoulType> {
        let callee = callee?;
        let value = match &callee.kind {
            FunctionCalleeKind::Type(_) => return None,
            FunctionCalleeKind::Expression(val) => *val,
        };

        let expr_node = &self.store.expressions.get(value)?.node;
        match expr_node {
            ExpressionKind::Variable(VariableExpression { name, .. }) => {
                if self.contains_type(name.as_str()) {
                    return Some(SoulType::Stub(Stub::new(name.as_str())));
                }
                if let Ok(prim) = PrimitiveTypes::from_str(name.as_str()) {
                    return Some(SoulType::Primitive(prim));
                }
            }
            ExpressionKind::FieldAccess(field_access) => {
                return self.parse_owner_from_field_access(field_access);
            }
            _ => {}
        }

        None
    }

    fn parse_owner_from_field_access(&mut self, field_access: &FieldAccess) -> Option<SoulType> {
        let (module_entry, field_name) = self.follow_field_access_to_module(field_access)?;
        let ast_module = self.ast_modules.get(module_entry.module_id)?;
        let header_entry = ast_module.header.get(&field_name)?;
        let custom_type = header_entry.custom_type.as_ref()?;
        Some(SoulType::Stub(Stub::new(custom_type.value.name().as_str())))
    }

    /// Walk a FieldAccess chain like `Std.Io.Stdout` to find the innermost
    /// module entry and the final field name.
    fn follow_field_access_to_module(
        &mut self,
        field_access: &FieldAccess,
    ) -> Option<(ScopeModuleEntry, String)> {
        let obj_expr = self.store.expressions.get(field_access.object)?;
        match &obj_expr.node {
            ExpressionKind::Variable(VariableExpression { name, .. }) => {
                let name_str = name.as_str();
                if let Some(module_entry) = self.lookup_module(name_str) {
                    return Some((module_entry, field_access.field.to_string()));
                }
                let module_entry = self.find_module_by_crate_name(name_str)?;
                Some((module_entry, field_access.field.to_string()))
            }
            ExpressionKind::FieldAccess(inner_fa) => {
                let (module_entry, _) = self.follow_field_access_to_module(inner_fa)?;
                let module_id = module_entry.module_id;
                let field_name = field_access.field.to_string();
                let inner_module = self.ast_modules.get(module_id)?;
                let header_entry = inner_module.header.get(&field_name)?;
                header_entry.custom_type.as_ref()?;
                Some((module_entry, field_name))
            }
            _ => None,
        }
    }

    fn find_module_by_crate_name(&mut self, name: &str) -> Option<ScopeModuleEntry> {
        if let Some(modules) = self.scope_info.scopes.iter_modules(self.current.module) {
            for (_, entry) in modules {
                if entry.crate_name.as_deref() == Some(name) {
                    return Some(entry.clone());
                }
            }
        }
        None
    }

    fn resolve_module_call(
        &mut self,
        expression_id: ExpressionId,
        module_entry: ScopeModuleEntry,
        call: &FunctionCall,
    ) {
        let resolve =
            self.lookup_module_function(&module_entry, call.name.as_str(), call.name.span());
        let id = match resolve {
            Some(id) => id,
            None => self.resolve_external_function(&module_entry, call),
        };

        let ignore_callee = id != FunctionId::ERROR;
        self.declares.insert_function_resolve(
            call.id,
            FunctionResolve {
                id,
                ignore_callee,
                is_defer: false,
            },
        );
        self.finish_call_resolution(expression_id, call, id);
    }

    fn resolve_external_function(
        &mut self,
        module_entry: &ScopeModuleEntry,
        call: &FunctionCall,
    ) -> FunctionId {
        let function_name = call.name.as_str();
        let location = match &module_entry.crate_name {
            Some(crate_name) => format!("crate '{crate_name}'"),
            None => format!("module '{}'", module_entry.module_name),
        };
        self.log_fault(Fault::error(
            format!("'{function_name}' not found in {location}"),
            Some(call.name.span()),
        ));

        FunctionId::ERROR
    }

    fn lookup_module_function(
        &mut self,
        module_entry: &ScopeModuleEntry,
        function_name: &str,
        span: Span,
    ) -> Option<FunctionId> {
        if let Some(function_name) = self.lookup_function_import(module_entry, function_name) {
            return self.lookup_function(&function_name);
        }

        let module_id = module_entry.module_id;
        debug_assert!(module_id != ModuleId::ERROR);
        debug_assert!(self.ast_modules.contains(module_id));

        let header = &self.ast_modules.get(module_id)?.header;
        let entry = header.get(function_name)?.function?;
        if !entry.is_public {
            self.log_fault(Fault::error(
                format!("'{function_name}' is private"),
                Some(span),
            ));
        }

        Some(entry.value)
    }

    fn lookup_function_import(
        &mut self,
        module_entry: &ScopeModuleEntry,
        function_name: &str,
    ) -> Option<String> {
        for item in &module_entry.imported_items {
            match item {
                ast_model::statements::ImportItem::Normal(name) => {
                    if name.as_str() == function_name {
                        return Some(name.to_string());
                    }
                }
                ast_model::statements::ImportItem::Alias { alias, name } => {
                    if alias.as_str() == function_name {
                        return Some(name.to_string());
                    }
                }
            }
        }

        None
    }

    fn try_get_callee_string(&mut self, call: &FunctionCall) -> Option<String> {
        let callee = call.callee.as_ref()?;
        let value = match &callee.kind {
            FunctionCalleeKind::Type(_) => return None,
            FunctionCalleeKind::Expression(val) => *val,
        };

        let Some(value) = self.store.expressions.get(value) else {
            self.log_fault(soul_error_internal!(
                format!("{value:?} not found"),
                Some(call.name.span())
            ));
            return None;
        };

        match &value.node {
            ExpressionKind::Variable(var) => Some(var.name.to_string()),
            _ => None,
        }
    }

    pub(super) fn contains_type(&mut self, ident: &str) -> bool {
        self.scope_info
            .scopes
            .lookup_type(ident, self.current.module)
            .is_some()
    }

    pub(super) fn lookup_module(&mut self, string: &str) -> Option<ScopeModuleEntry> {
        self.scope_info
            .scopes
            .lookup_module(string, self.current.module)
    }

    fn lookup_function(&mut self, string: &str) -> Option<FunctionId> {
        self.scope_info
            .scopes
            .lookup_function(string, self.current.module)
    }
}

pub(super) fn is_generic_parameter(ty: &SoulType, generics: &[Generic]) -> bool {
    match ty {
        SoulType::ImplTrait(_) => true,
        SoulType::Stub(stub) => generics
            .iter()
            .any(|generic| generic.name.as_str() == stub.name.as_str()),
        _ => false,
    }
}

/// The declared generic's name if `ty` is a bare reference to it (e.g. `T`
/// in `foo<T>(a: T)`), so repeated uses of the same generic within one call
/// can be checked against each other.
fn generic_name_of<'g>(ty: &SoulType, generics: &'g [Generic]) -> Option<&'g str> {
    let SoulType::Stub(stub) = ty else {
        return None;
    };
    generics
        .iter()
        .find(|generic| generic.name.as_str() == stub.name.as_str())
        .map(|generic| generic.name.as_str())
}
