use std::path::PathBuf;

use ast::{
    Block, DeclareStore, EntryKind, Expression, ExpressionKind, Function, FunctionSignature,
    ImportItem, ImportPath, Literal, Statement, StatementKind, TypeKind, UseBlock, VarTypeKind,
    Variable,
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
                let prev = self.current.in_global;
                self.current.in_global = false;
                self.collect_type(use_type);
                for methode in methodes {
                    self.check_function_name(&methode.signature.node.name);
                    self.collect_function(methode);
                }

                if !impls.is_empty() {
                    todo!()
                }
                self.current.in_global = prev;
            }
            StatementKind::Import(import) => {
                for path in &import.paths {
                    self.collect_import_path(path, statement.span)
                }
            }
            StatementKind::Struct(obj) => {
                self.declare_struct(obj);

                if self.current.in_global {
                    self.header_insert_struct(obj.clone());
                }
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

                self.store
                    .insert_variable_type(id, variable.ty.clone(), self.current.module);

                if let Some(hint) = self.try_get_owner_hint(variable) {
                    self.store
                        .insert_variable_owner_hint(id, hint, self.current.module);
                }

                match &mut variable.ty {
                    VarTypeKind::NonInveredType(soul_type) => self.collect_type(soul_type),
                    VarTypeKind::InveredType(_) => (),
                }

                if let Some(value) = &mut variable.initialize_value {
                    self.collect_expression(value);
                }

                if self.current.in_global {
                    self.header_insert_variable(variable);
                }
            }
            StatementKind::ExternalFunction(function) | StatementKind::Function(function) => {
                self.check_function_name(&function.signature.node.name);

                let prev = self.current.in_global;
                self.current.in_global = false;
                self.collect_function(function);
                self.current.in_global = prev;

                if self.current.in_global {
                    self.header_insert_function(function);
                }
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
        let prev = self.current.function;
        self.current.function = Some(id);

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

        self.store
            .insert_functions(id, signature.clone(), self.current.module);
        self.current.function = prev;
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
            ast::ImportKind::Items { items, .. } => items,
            _ => &vec![],
        };

        let Some(module_file_path) = self.find_module_file(&module_name, span) else {
            return;
        };

        let import_name = alias.unwrap_or(module_name);

        let module_id = self.import_module(module_file_path, module_name, span);

        self.declare_module(
            import_name,
            &module_name,
            module_id,
            path.kind.clone(),
            imported_items.clone(),
        );

        self.collect_items(module_id, module_name, &imported_items, span)
    }

    fn collect_items(
        &mut self,
        module_id: ModuleId,
        module_name: &str,
        imported_items: &Vec<ImportItem>,
        span: Span,
    ) {
        for item in imported_items {
            let (name, alias_name) = match &item {
                ast::ImportItem::Alias { name, alias } => (name.as_str(), alias),
                ast::ImportItem::Normal(name) => (name.as_str(), name),
            };

            let Some(entry) = self.modules[module_id].header.get(name) else {
                self.log_error(SoulError::new(
                    format!("module '{}' does not export '{}'", module_name, name),
                    SoulErrorKind::NotFoundInScope,
                    Some(span),
                ));
                continue;
            };

            let entry_variable = entry.variable;
            let entry_function = entry.function;
            if let Some(EntryKind {
                value: obj,
                is_public,
            }) = &entry.struct_type
            {
                if !is_public {
                    Self::static_log_error(
                        self.context,
                        SoulError::new(
                            format!("struct {} is private", alias_name.as_str()),
                            SoulErrorKind::AlreadyFoundInScope,
                            Some(alias_name.span),
                        ),
                    );
                }

                let id = match obj.id {
                    Some(val) => val,
                    None => {
                        self.log_error(soul_error_internal!(
                            format!("Struct: '{}' node_id is None", obj.name.as_str()),
                            None
                        ));
                        return;
                    }
                };

                if !Self::insert_struct_alias(&mut self.info.scopes, alias_name, span, id) {
                    Self::static_log_error(
                        self.context,
                        SoulError::new(
                            format!("struct {} already exists", alias_name.as_str()),
                            SoulErrorKind::AlreadyFoundInScope,
                            Some(alias_name.span),
                        ),
                    );
                }

                Self::resolve_struct(self.context, self.store, &self.current, obj);
            }

            if let Some(EntryKind {
                value: id,
                is_public,
            }) = entry_variable
            {
                if !is_public {
                    self.log_error(SoulError::new(
                        format!("variable '{}' is private", alias_name.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(alias_name.span),
                    ));
                }

                if !self.insert_variable_alias(alias_name, id) {
                    self.log_error(SoulError::new(
                        format!("variable '{}' already exists", alias_name.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(alias_name.span),
                    ));
                }
            }

            if let Some(EntryKind {
                value: id,
                is_public,
            }) = entry_function
            {
                if !is_public {
                    self.log_error(SoulError::new(
                        format!("function '{}' is private", alias_name.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(alias_name.span),
                    ));
                }

                if !self.insert_function_alias(alias_name, id) {
                    self.log_error(SoulError::new(
                        format!("function '{}' already exists", alias_name.as_str()),
                        SoulErrorKind::AlreadyFoundInScope,
                        Some(alias_name.span),
                    ));
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
            return module_id;
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
                && let Some(function_id) = store.find_function(function_name, Some(&owner_kind))
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
