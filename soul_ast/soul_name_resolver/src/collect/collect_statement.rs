use std::fs::read_to_string;

use ast::{
    Block, DeclareStore, Expression, ExpressionKind, Function, FunctionSignature, ImportPath, Literal, Statement, StatementKind, TypeKind, UseBlock, VarTypeKind, scope::{ScopeBuilder, ScopeValue, ScopeValueKind}
};
use soul_utils::{
    error::{SoulError, SoulErrorKind}, soul_error_internal, soul_names::PrimitiveTypes, span::{Span, Spanned}
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

                self.store.insert_variable_type(id, variable.ty.clone());

                if matches!(&variable.ty, VarTypeKind::InveredType(_))
                    && let Some(init) = &variable.initialize_value
                    && let Some(hint) =
                        owner_hint_from_initializer_literal(init, &self.info.scopes, self.store)
                {
                    self.store.insert_variable_owner_hint(id, hint);
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
        match signature.function_kind {
            ast::FunctionKind::Static => (),
            ast::FunctionKind::MutRef
            | ast::FunctionKind::Consume
            | ast::FunctionKind::ConstRef => {
                let id = self.alloc_node();
                self.insert_value("this", id, ScopeValue::Variable);
            }
        }
        self.declare_parameters(&mut signature.parameters);
        self.collect_scopeless_block(&mut function.block);
        self.pop_scope();

        self.store.insert_functions(id, signature.clone());
        self.current_function = prev;
    }

    fn collect_import_path(&mut self, path: &ImportPath, span: Span) {
        let module_name = path.module.as_str().to_string();
        let alias = match &path.kind {
            ast::ImportKind::All | ast::ImportKind::This | ast::ImportKind::Items(_) => None,
            ast::ImportKind::Alias(ident) => Some(ident.clone()),
        };

        let import_name = alias.as_ref().map(|a| a.as_str()).unwrap_or(&module_name);
        self.declare_module(import_name, &module_name, path.kind.clone());

        let module_file_path = match self.find_module_file(&module_name) {
            Some(val) => val,
            None => {
                self.log_error(SoulError::new(
                    format!("import '{}': module not found. Make sure the file exists and the path is correct (e.g., './{}' or '{}')", 
                        module_name, module_name, module_name),
                    SoulErrorKind::NotFoundInScope,
                    Some(span),
                ));
                return;
            }
        };

        let module_source = match read_to_string(&module_file_path) {
            Ok(val) => val,
            Err(err) => {
                self.log_error(soul_error_internal!(
                    format!("import '{}': could not read module file '{}': {}", 
                        module_name, module_file_path.display(), err),
                    Some(span)
                ));
                return;
            }
        };

        if let Some(module_ast) = self.parse_module(&module_source) {
            let items_count = module_ast.statements.iter()
                .filter(|s| matches!(s.node, StatementKind::Function(_) | 
                                        StatementKind::ExternalFunction(_) | 
                                        StatementKind::Struct(_)))
                .count();
            
            if items_count == 0 {
                self.log_error(SoulError::new(
                    format!("import '{}': module has no public items to import (functions, extern functions, or structs)", 
                        module_name),
                    SoulErrorKind::NotFoundInScope,
                    Some(span),
                ));
                return;
            }

            for stmt in module_ast.statements {
                match stmt.node {
                    StatementKind::Import(import) => {
                        self.collect_import_path(&import.paths[0], stmt.span);
                    }
                    StatementKind::Function(func) => {
                        self.add_imported_function(func, import_name.to_string());
                    }
                    StatementKind::ExternalFunction(extern_func) => {
                        self.add_imported_function(
                            Function {
                                signature: Spanned::new(
                                    ast::FunctionSignature {
                                        id: None,
                                        name: extern_func.signature.node.name.clone(),
                                        methode_type: extern_func.signature.node.methode_type.clone(),
                                        parameters: extern_func.signature.node.parameters.clone(),
                                        return_type: extern_func.signature.node.return_type.clone(),
                                        function_kind: extern_func.signature.node.function_kind,
                                        generics: extern_func.signature.node.generics.clone(),
                                        external: Some(ast::ExternLanguage::C),
                                    },
                                    Span::default(),
                                ),
                                block: ast::Block {
                                    modifier: soul_utils::soul_names::TypeModifier::Mut,
                                    statements: vec![],
                                    scope_id: None,
                                    node_id: None,
                                    span: Span::default(),
                                },
                            },
                            import_name.to_string(),
                        );
                    }
                    _ => {}
                }
            }
        } else {
            self.log_error(SoulError::new(
                format!("import '{}': failed to parse module", module_name),
                SoulErrorKind::NotFoundInScope,
                Some(span),
            ));
        }
    }
}

fn is_main(signature: &FunctionSignature) -> bool {
    signature.name.as_str() == "main" && matches!(signature.methode_type.kind, TypeKind::None)
}

fn owner_hint_from_initializer_literal(
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
                && let Some(signature) = store.get_function(function_id)
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
