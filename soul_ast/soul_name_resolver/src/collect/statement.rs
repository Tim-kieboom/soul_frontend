use ast_model::{
    CustomType, FunctionKind,
    scope::{ScopeValue, ScopeValueKind},
    soul_type::SoulType,
    statements::{
        Function, FunctionSignature, FunctionThisKind, StatementId, StatementKind, UseBlock,
        Variable,
    },
};
use soul_utils::{FunctionId, Ident, error::SoulResult, fault::Fault, soul_error_internal};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_statement(&mut self, id: StatementId) {
        let Some(statement) = self.store.statements.get(id) else {
            self.log_fault(soul_error_internal!(format!("{id:?} not found"), None));
            return;
        };

        match &statement.node {
            StatementKind::Enum(enum_) => {
                self.declare_enum(enum_);
                if self.current.in_global {
                    let ty = CustomType::Enum(enum_.clone());
                    self.header_insert_custom_type(id, ty);
                }
            }
            StatementKind::Trait(trait_) => {
                self.declare_trait(trait_);
                if self.current.in_global {
                    let ty = CustomType::Trait(trait_.clone());
                    self.header_insert_custom_type(id, ty);
                }
            }
            StatementKind::Struct(struct_) => {
                self.declare_struct(struct_);
                if self.current.in_global {
                    let ty = CustomType::Struct(struct_.clone());
                    self.header_insert_custom_type(id, ty);
                }
            }
            StatementKind::Import(import) => {
                let span = statement.span;
                for path in &import.paths {
                    self.collect_import_path(path, span);
                }
            }
            StatementKind::TypeDef(type_def) => {
                self.collect_type(&type_def.new_type);
                self.collect_type(&type_def.old_type);
            }
            StatementKind::Variable(variable) => self.collect_variable(variable),
            StatementKind::UseBlock(use_block) => self.collect_use_block(use_block),
            StatementKind::Assignment(assignment) => {
                self.collect_expression(assignment.left);
                self.collect_expression(assignment.right);
            }
            StatementKind::Expression {
                expression,
                ends_semicolon: _,
            } => {
                self.collect_expression(*expression);
            }
            StatementKind::Function(id) | StatementKind::ExternalFunction(id) => {
                self.collect_function_id(*id)
            }
        }
    }

    fn collect_use_block(&mut self, use_block: &UseBlock) {
        let UseBlock {
            use_generics: _,
            ty,
            impls,
            methods,
            statements,
        } = use_block;

        let prev = self.current.in_global;
        self.current.in_global = false;
        self.collect_type(ty);
        for method in methods {
            self.collect_function_id(method.id);
        }

        for impl_block in impls {
            self.collect_type(&impl_block.impl_trait);
            for method in &impl_block.methods {
                self.collect_function_id(*method);
            }
        }

        for method in methods {
            self.collect_function_id(method.id)
        }

        for statement in statements {
            self.collect_statement(*statement);
        }

        self.current.in_global = prev;
    }

    fn collect_function_id(&mut self, function_id: FunctionId) {
        let Some(function_kind) = self.store.functions.get(function_id) else {
            return;
        };
        let signature = &function_kind.signature().value;
        self.check_function_name(&signature.name);

        if let FunctionKind::Normal(function) = function_kind {
            self.collect_function(function);
        }

        if self.current.in_global {
            self.header_insert_function_id(function_id);
        }
    }

    fn collect_function(&mut self, function: &Function) {
        let prev_in_global = self.current.in_global;
        let prev_function = self.current.function;

        let id = self.declare_function(&function.signature);
        self.current.function = Some(id);

        if is_main(&function.signature.value) {
            self.declares.main_function = Some(id);
        }

        let signature = &function.signature.value;
        self.collect_type(&signature.method_type);
        self.collect_type(&signature.return_type);

        self.declares
            .insert_functions(id, signature.clone(), self.current.module);

        for parameter in signature.parameters.iter() {
            self.collect_type(&parameter.ty);
            let span = parameter.name.span();
            let name = parameter.name.as_str();
            self.insert_value(name, parameter.id, span, ScopeValue::Variable);
            if let Some(value) = parameter.default {
                self.collect_expression(value);
            }
        }

        self.push_scope(function.block);
        if signature.function_kind != FunctionThisKind::Static {
            let id = self.node_generator.alloc();
            self.insert_value("this", id, signature.name.span(), ScopeValue::Variable);
        }
        self.collect_scopeless_block(function.block);
        self.pop_scope();

        self.current.function = prev_function;
        self.current.in_global = prev_in_global;
    }

    fn collect_variable(&mut self, variable: &Variable) {
        if let Err(err) = check_variable_name(&variable.name) {
            self.log_fault(err);
        }

        self.declare_value(ScopeValueKind::Variable(variable));

        let id = variable.id;
        self.declares.insert_variable_type(
            id,
            variable.modifier,
            variable.ty.clone(),
            self.current.module,
        );

        if let Some(value) = variable.initialize_value {
            self.collect_expression(value);
        }

        if let Some(ty) = &variable.ty {
            self.collect_type(ty);
        }

        if self.current.in_global {
            self.header_insert_variable(variable);
        }
    }

    fn check_function_name(&mut self, name: &Ident) {
        if let Err(err) = check_function_name(name) {
            self.log_fault(err);
        }

        let Some(id) = self.current.function else {
            return;
        };

        let Some(FunctionKind::Normal(function)) = self.store.functions.get(id) else {
            self.log_fault(soul_error_internal!(
                format!("parent function {id:?} not found"),
                Some(name.span())
            ));
            return;
        };
        let signature = &function.signature.value;

        if signature.name.as_str() == name.as_str() {
            self.log_fault(Fault::error(
                "parent and child function can not have the same name",
                Some(name.span()),
            ));
        }
    }
}

fn is_main(signature: &FunctionSignature) -> bool {
    signature.name.as_str() == "main" && matches!(signature.method_type, SoulType::None)
}

fn check_function_name(name: &Ident) -> SoulResult<()> {
    let mut chars = name.as_str().chars();
    let first = chars.next().ok_or(Fault::error(
        "function name can not be empty",
        Some(name.span()),
    ))?;

    if !first.is_alphabetic() && first != '_' {
        return Err(Fault::error(
            format!("function name should not start with '{first}' (start with letter or '_')"),
            Some(name.span()),
        ));
    }

    if name.as_str().contains("___") {
        return Err(Fault::error(
            "function name should not have '___' in the name",
            Some(name.span()),
        ));
    }

    Ok(())
}

fn check_variable_name(name: &Ident) -> SoulResult<()> {
    let mut chars = name.as_str().chars();
    let first = chars.next().ok_or(Fault::error(
        "variable name can not be empty",
        Some(name.span()),
    ))?;

    if !first.is_alphabetic() && first != '_' {
        return Err(Fault::error(
            format!("variable name should not start with '{first}' (start with letter or '_')"),
            Some(name.span()),
        ));
    }

    Ok(())
}
