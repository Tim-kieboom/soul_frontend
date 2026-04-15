use std::{path::PathBuf};

use ast::{
    Block, DeclareStore, Expression, ExpressionKind, Function, FunctionSignature,
    ImportPath, Literal, Statement, StatementKind, TypeKind, UseBlock, VarTypeKind, Variable,
    scope::{ScopeBuilder, ScopeValue, ScopeValueKind},
};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_error_internal,
    soul_names::PrimitiveTypes,
    span::{ModuleId, Span},
};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_block(&mut self, block: &mut Block) {
        self.push_scope(&mut block.scope_id);
        self.collect_scopeless_block(block);
        self.pop_scope();
    }

    pub(crate) fn collect_scopeless_block(&mut self, block: &mut Block) {
        block.node_id = Some(self.alloc_node());
        for statement in &mut block.statements {
            self.collect_statement(statement);
        }
    }

    fn collect_statement(&mut self, statement: &mut Statement) {
        match &mut statement.node {
            StatementKind::UseBlock(UseBlock {
                use_type,
                impls,
                generics: _,
                methodes,
            }) => {
                self.collect_type(use_type);
                for methode in methodes {
                    self.check_function_name(&methode.signature.node.name);
                    self.collect_function(methode);
                }

                if !impls.is_empty() {
                    todo!()
                }
            }
            StatementKind::Import(import) => {
                for path in &import.paths {
                    self.collect_import_path(path, statement.span)
                }
            }
            StatementKind::Struct(obj) => {
                self.declare_struct(obj);
            }
            StatementKind::Variable(variable) => {
                self.check_variable_name(&variable.name);
                let id = if let Some(id) = self.flat_check_variable(&variable.name) {
                    self.log_error(SoulError::new(
                        format!(
                            "variable '{}' already defined in scope",
                            variable.name.as_str()
                        ),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(variable.name.span),
                    ));
                    id
                } else {
                    self.declare_value(ScopeValueKind::Variable(variable))
                };

                self.store.insert_variable_type(id, variable.ty.clone(), self.current_module);

                if let Some(hint) = self.try_get_owner_hint(variable) {
                    self.store.insert_variable_owner_hint(id, hint, self.current_module);
                }

                match &mut variable.ty {
                    VarTypeKind::NonInveredType(soul_type) => self.collect_type(soul_type),
                    VarTypeKind::InveredType(_) => (),
                }

                if let Some(value) = &mut variable.initialize_value {
                    self.collect_expression(value);
                }
            }
            StatementKind::ExternalFunction(function) | StatementKind::Function(function) => {
                self.check_function_name(&function.signature.node.name);
                self.collect_function(function);
            }
            StatementKind::Expression {
                id,
                expression,
                ends_semicolon: _,
            } => {
                *id = Some(self.alloc_node());
                self.collect_expression(expression);
            }
            StatementKind::Assignment(assignment) => {
                assignment.node_id = Some(self.alloc_node());
                self.collect_expression(&mut assignment.left);
                self.collect_expression(&mut assignment.right);
            }
        }
    }

    pub(crate) fn collect_function(&mut self, function: &mut Function) {
        let id = self.declare_function(&mut function.signature);
        let prev = self.current_function;
        self.current_function = Some(id);

        if is_main(&function.signature.node) {
            self.store.main_function = Some(id);
        }

        let signature = &mut function.signature.node;
        self.collect_type(&mut signature.methode_type);
        self.collect_type(&mut signature.return_type);

        self.push_scope(&mut function.block.scope_id);

        if signature.function_kind != ast::FunctionKind::Static {
            let id = self.alloc_node();
            self.insert_value("this", id, ScopeValue::Variable);
        }

        self.declare_parameters(&mut signature.parameters);
        self.collect_scopeless_block(&mut function.block);
        self.pop_scope();

        self.store.insert_functions(id, signature.clone(), self.current_module);
        self.current_function = prev;
    }

    fn collect_import_path(&mut self, path: &ImportPath, span: Span) {
        let module_name = match path.module.get_module_name() {
            Some(val) => val,
            None => {
                self.log_error(soul_error_internal!("could not get module name", None));
                return;
            }
        };

        let alias = match &path.kind {
            ast::ImportKind::Alias(ident) => Some(ident.as_str()),
            _ => None,
        };

        let imported_items = match &path.kind {
            ast::ImportKind::Items{ this, this_alias, items } => items.clone(),
            _ => vec![],
        };

        let Some(module_file_path) = self.find_module_file(&module_name, span) else {
            return
        };

        let import_name = alias.unwrap_or(module_name);
        self.declare_module(import_name, &module_name, path.kind.clone(), imported_items.clone());

        let module_id = self.import_module(module_file_path, module_name, span);

        for item in imported_items {
            match item {
                ast::ImportItem::Alias { name, alias: alias_name } => {
                    let Some(func_id) = self.store.find_function_in_module(name.as_str(), module_id) else {
                        continue;
                    };
                    self.insert_function_alias(alias_name.as_str(), func_id);
                }
                ast::ImportItem::Normal(ident) => {
                    let Some(func_id) = self.store.find_function_in_module(ident.as_str(), module_id) else {
                        continue;
                    };
                    self.insert_function_alias(ident.as_str(), func_id);
                }
            }
        }
    }

    fn try_get_owner_hint(&self, variable: &Variable) -> Option<TypeKind> {
        if !matches!(&variable.ty, VarTypeKind::InveredType(_)) {
            return None;
        }

        let init = variable.initialize_value.as_ref()?;
        owner_hint_from_expression(init, &self.info.scopes, self.store)
    }

    fn import_module(
        &mut self,
        module_file_path: PathBuf,
        module_name: &str,
        span: Span,
    ) -> ModuleId {

        let Some(module_source) = self.read_module(&module_file_path, module_name, span) else {
            return self.root;
        };

        let dir = module_file_path.parent().unwrap_or(&module_file_path);
        self.context.push_current_path(dir.to_path_buf());
        let module_id = self.context.module_store.get_or_insert(module_file_path);
        if self.modules.get(module_id).is_some() {
            self.context.pop_current_path();
            return module_id
        }

        self.parse_module(&module_source, module_id, module_name.to_string());
        self.context.pop_current_path();
        module_id
    }

    fn read_module(&mut self, path: &PathBuf, module_name: &str, span: Span) -> Option<String> {
        match std::fs::read_to_string(path) {
            Ok(val) => Some(val),
            Err(err) => {
                self.log_error(soul_error_internal!(
                    format!(
                        "import '{}': could not read module file '{}': {}",
                        module_name,
                        path.display(),
                        err,
                    ),
                    Some(span)
                ));
                None
            }
        }
    }
}

fn is_main(signature: &FunctionSignature) -> bool {
    signature.name.as_str() == "main" && matches!(signature.methode_type.kind, TypeKind::None)
}

fn owner_hint_from_expression(
    init: &Expression,
    scopes: &ScopeBuilder,
    store: &DeclareStore,
) -> Option<TypeKind> {
    match &init.node {
        ExpressionKind::Literal((_, lit)) => Some(TypeKind::Primitive(match lit {
            Literal::Int(_) | Literal::Uint(_) => PrimitiveTypes::Int,
            Literal::Float(_) => PrimitiveTypes::Float64,
            Literal::Bool(_) => PrimitiveTypes::Boolean,
            Literal::Char(_) => PrimitiveTypes::Char,
            Literal::Str(_) => return None,
        })),
        ExpressionKind::FunctionCall(function_call) => {
            let owner_kind = function_call
                .callee
                .as_ref()
                .and_then(|callee| parse_callee_type(callee, scopes));

            let function_name = function_call.name.as_str();

            if let Some(owner_kind) = owner_kind
                && let Some(function_id) =
                    store.find_function_by_name_and_owner_kind(function_name, Some(&owner_kind))
                && let Some((signature, _module)) = store.get_function(function_id)
            {
                return Some(signature.return_type.kind.clone());
            }
            None
        }
        _ => None,
    }
}

fn parse_callee_type(callee: &Expression, scopes: &ScopeBuilder) -> Option<TypeKind> {
    match &callee.node {
        ExpressionKind::Variable { ident, .. } => {
            if let Some(primitive) = PrimitiveTypes::from_str(ident.as_str()) {
                return Some(TypeKind::Primitive(primitive));
            }
            if scopes.lookup_type(ident).is_some() {
                return Some(TypeKind::Stub(ast::Stub {
                    name: ident.as_str().to_string(),
                    generics: vec![],
                }));
            }
            None
        }
        _ => None,
    }
}
