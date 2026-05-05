use ast::{
    Block, DeclareStore, Expression, ExpressionKind, Function, FunctionSignature, Literal,
    Statement, StatementKind, TypeKind, UseBlock, VarTypeKind, Variable,
    scope::{ScopeBuilder, ScopeValue, ScopeValueKind},
};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_names::PrimitiveTypes,
    span::ModuleId,
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
            StatementKind::Enum(obj) => {
                self.declare_enum(obj);

                if self.current.in_global {
                    self.header_insert_enum(obj.clone());
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
        let prev_in_global = self.current.in_global;
        let prev_function = self.current.function;

        let id = self.declare_function(&mut function.signature);

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
        self.current.function = prev_function;
        self.current.in_global = prev_in_global;
    }

    fn try_get_owner_hint(&self, variable: &Variable) -> Option<TypeKind> {
        if !matches!(&variable.ty, VarTypeKind::InveredType(_)) {
            return None;
        }

        let init = variable.initialize_value.as_ref()?;
        owner_hint_from_expression(init, &self.info.scopes, self.store, self.current.module)
    }
}

fn is_main(signature: &FunctionSignature) -> bool {
    signature.name.as_str() == "main" && matches!(signature.methode_type.kind, TypeKind::None)
}

fn owner_hint_from_expression(
    init: &Expression,
    scopes: &ScopeBuilder,
    store: &DeclareStore,
    module: ModuleId,
) -> Option<TypeKind> {
    // Provides a "hint" for the type of an expression when inferring an owner's type.
    //
    // WHY LITERALS GET DEFAULT TYPES:
    // This is where integer literals first get typed. All integer literals (42, 1, 0, etc.)
    // are initially given type `Int` (which becomes i64). This is by design - it provides a
    // default type so literals can participate in type inference.
    //
    // The actual required type is determined later in MIR when the literal is used in:
    // - Binary expressions: cast to match the other operand's type
    // - Function calls: cast to match the parameter type
    //
    // Without this default, literals would have no type and couldn't participate in
    // type inference at all.
    match &init.node {
        ExpressionKind::Literal((_, lit)) => Some(TypeKind::Primitive(match lit {
            Literal::Int(_) | Literal::Uint(_) => PrimitiveTypes::Int,
            Literal::Float(_) => PrimitiveTypes::Float64,
            Literal::Bool(_) => PrimitiveTypes::Boolean,
            Literal::Char(_) => PrimitiveTypes::Char,
            Literal::Str(_) | Literal::Cstr(_) => return None,
        })),
        ExpressionKind::FunctionCall(function_call) => {
            if let Some(intrinsic) = function_call.intrinsic {
                return match intrinsic {
                    ast::Intrinsic::InFile => Some(TypeKind::Primitive(PrimitiveTypes::CStr)),
                    ast::Intrinsic::InLine => Some(TypeKind::Primitive(PrimitiveTypes::Int)),
                };
            }

            let owner_kind = function_call
                .callee
                .as_ref()
                .and_then(|callee| parse_callee_type(callee, scopes, module));

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

fn parse_callee_type(
    callee: &Expression,
    scopes: &ScopeBuilder,
    module: ModuleId,
) -> Option<TypeKind> {
    match &callee.node {
        ExpressionKind::Variable { ident, .. } => {
            if let Some(primitive) = PrimitiveTypes::from_str(ident.as_str()) {
                return Some(TypeKind::Primitive(primitive));
            }
            if scopes.lookup_type(ident, module).is_some() {
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
