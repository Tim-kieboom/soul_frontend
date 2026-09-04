use ast_model::{
    FunctionKind,
    expression::ExpressionId,
    statements::{
        Assignment, Enum, EnumVariant, StatementId, StatementKind, Struct, Trait, UseBlock,
        VarPattern, Variable,
    },
};
use soul_utils::{FunctionId, TypeModifier, fault::Fault, soul_error_internal};

use super::function_call::is_generic_parameter;
use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_statement(&mut self, id: StatementId) {
        let Some(statement) = self.store.statements.get(id) else {
            self.log_fault(soul_error_internal!(format!("{id:?} not found"), None));
            return;
        };

        match &statement.node {
            StatementKind::Import(_) | StatementKind::TypeDef(_) => (),
            StatementKind::Enum(enum_) => self.resolve_enum(enum_),
            StatementKind::Union(union_) => self.resolve_enum(union_),
            StatementKind::Function(id) => self.resolve_function(*id),
            StatementKind::Trait(trait_) => self.resolve_trait(trait_),
            StatementKind::Struct(struct_) => self.resolve_struct(struct_),
            StatementKind::Variable(variable) => self.resolve_variable(variable),
            StatementKind::UseBlock(use_block) => self.resolve_use_block(use_block),
            StatementKind::ExternalFunction(id) => self.resolve_function(*id),
            StatementKind::Assignment(assignment) => self.resolve_assignment(assignment),
            StatementKind::Expression {
                expression,
                ends_semicolon: _,
            } => self.resolve_expression(*expression),
        }
    }

    fn resolve_assignment(&mut self, assignment: &Assignment) {
        self.resolve_expression(assignment.left);
        self.resolve_expression(assignment.right);
        self.check_assignment(assignment);
    }

    fn check_assignment(&mut self, assignment: &Assignment) {
        let Some((modifier, left_ty)) = self.variable_lvalue(assignment.left) else {
            return;
        };

        if matches!(modifier, TypeModifier::Immut | TypeModifier::Const) {
            let span = self.get_expression(assignment.left).map(|expr| expr.span);

            self.log_fault(Fault::error(
                "cannot assign to an immutable variable".to_string(),
                span,
            ));
        }

        let empty = vec![];
        let generics = match self.current.function {
            Some(id) => self
                .declares
                .get_function(id)
                .map(|(signature, _)| &signature.generics)
                .unwrap_or(&empty),
            None => &empty,
        };

        if is_generic_parameter(&left_ty, generics) {
            return;
        }

        let Some(right_ty) = self.expression_type(assignment.right) else {
            return;
        };
        if self.combine_operand_types(&right_ty, &left_ty).is_some() {
            return;
        }

        let span = self.get_expression(assignment.right).map(|expr| expr.span);
        self.log_fault(Fault::error(
            format!("assignment type mismatch: expected `{left_ty:?}`, got `{right_ty:?}`"),
            span,
        ));
    }

    fn resolve_use_block(&mut self, use_block: &UseBlock) {
        for method in &use_block.methods {
            self.resolve_function(method.id);
        }

        for impl_block in &use_block.impls {
            for method in &impl_block.methods {
                self.resolve_function(*method);
            }
        }

        for statement in &use_block.statements {
            self.resolve_statement(*statement);
        }
    }

    fn resolve_variable(&mut self, variable: &Variable) {
        if let Some(value) = variable.initialize_value {
            self.resolve_expression(value);
            self.backfill_variable_type(variable, value);
        }
    }

    fn backfill_variable_type(&mut self, variable: &Variable, value: ExpressionId) {
        let type_key = match &variable.pattern {
            VarPattern::Simple { binding, .. } => binding.id,
            _ => variable.id,
        };
        if self
            .declares
            .get_variable_type(type_key)
            .is_some_and(|(_, ty, _)| ty.is_some())
        {
            return;
        }

        let Some(ty) = self.expression_type(value) else {
            return;
        };
        self.declares.insert_variable_type(
            type_key,
            variable.modifier,
            Some(ty),
            self.current.module,
        );
    }

    fn resolve_struct(&mut self, struct_: &Struct) {
        for field in &struct_.fields {
            if let Some(value) = &field.value.initialize_value {
                self.resolve_expression(*value);
            }
        }
        for statement in &struct_.statements {
            self.resolve_statement(*statement);
        }
    }

    fn resolve_trait(&mut self, trait_: &Trait) {
        for method in &trait_.methods {
            self.resolve_function(*method);
        }
    }

    fn resolve_function(&mut self, function_id: FunctionId) {
        let Some(function_kind) = self.store.functions.get(function_id) else {
            self.log_fault(soul_error_internal!(
                format!("{function_id:?} not found"),
                None
            ));
            return;
        };

        let prev = self.current.function;
        let signature = &function_kind.signature();
        self.current.function = Some(signature.id);
        for parameter in &signature.parameters {
            if let Some(default) = &parameter.default {
                self.resolve_expression(*default);
            }
        }

        self.declares
            .insert_functions(signature.id, (*signature).clone(), self.current.module);

        match function_kind {
            FunctionKind::Signature(_) => (),
            FunctionKind::Normal(function) => {
                self.resolve_block(function.block);
                self.check_tail_return_type(
                    function.block,
                    &signature.return_type,
                    &signature.generics,
                );
            }
        };

        self.current.function = prev;
    }

    fn resolve_enum(&mut self, enum_: &Enum) {
        for variant in &enum_.variants {
            match variant {
                EnumVariant::Union(_) | EnumVariant::Normal(_) => (),
                EnumVariant::Assigned { name: _, value } => self.resolve_expression(*value),
            }
        }
    }
}
