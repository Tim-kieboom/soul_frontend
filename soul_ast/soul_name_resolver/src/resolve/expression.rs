use ast_model::{
    expression::{
        AnyArray, Constructor, ExpressionId, ExpressionKind, FieldAccess, For, ForCondition, If,
        IfBranch, Lambda, Match, MatchMethod, StringFormat, StructConstructor, VariableExpression,
    },
    scope::ScopeValue,
};
use soul_utils::{fault::Fault, soul_error_internal};

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn resolve_expression(&mut self, expression_id: ExpressionId) {
        let Some(expression) = self.store.expressions.get(expression_id) else {
            self.log_fault(soul_error_internal!(
                format!("{expression_id:?} not found"),
                None
            ));
            return;
        };

        match &expression.node {
            ExpressionKind::Break
            | ExpressionKind::Null(_)
            | ExpressionKind::None(_)
            | ExpressionKind::Continue
            | ExpressionKind::Literal(_)
            | ExpressionKind::Undefined(_) => (),

            ExpressionKind::New(value)
            | ExpressionKind::Pass(value)
            | ExpressionKind::Copy(value)
            | ExpressionKind::Sizeof(value) => self.resolve_expression(*value),

            ExpressionKind::If(if_) => self.resolve_if(if_),
            ExpressionKind::Ref(ref_) => self.resolve_expression(ref_.value),
            ExpressionKind::For(for_) => self.resolve_for(for_),
            ExpressionKind::Unary(unary) => self.resolve_expression(unary.value),
            ExpressionKind::Deref(deref) => self.resolve_expression(deref.value),
            ExpressionKind::Index(index) => {
                self.resolve_expression(index.index);
                self.resolve_expression(index.collection);
            }
            ExpressionKind::Match(match_) => self.resolve_match(match_),
            ExpressionKind::Binary(binary) => {
                self.resolve_expression(binary.left);
                self.resolve_expression(binary.right);
            }

            ExpressionKind::Tuple(values) => {
                for value in values {
                    self.resolve_expression(*value);
                }
            }
            ExpressionKind::NamedTuple(values) => {
                for (_, value) in values {
                    self.resolve_expression(*value);
                }
            }

            ExpressionKind::Lambda(lambda) => self.resolve_lambda(lambda),
            ExpressionKind::Block(block_id) => self.resolve_block(*block_id),
            ExpressionKind::TypeOf(type_of) => self.resolve_expression(type_of.value),
            ExpressionKind::Array(any_array) => self.resolve_any_array(any_array),
            ExpressionKind::Variable(variable) => {
                self.resolve_variable_expression(&variable);
            }
            ExpressionKind::NewArray(any_array) => self.resolve_any_array(any_array),
            ExpressionKind::Return(expression_id) => {
                if let Some(value) = expression_id {
                    self.resolve_expression(*value);
                }
            }
            ExpressionKind::Constructor(constructor) => self.resolve_contructor(constructor),
            ExpressionKind::FieldAccess(field_access) => self.resolve_field_access(field_access),
            ExpressionKind::MatchMethod(match_method) => self.resolve_match_method(match_method),
            ExpressionKind::StringFormat(string_format) => {
                self.resolve_string_format(string_format)
            }
            ExpressionKind::FunctionCall(function_call) => {
                self.resolve_function_call(function_call)
            }
            ExpressionKind::StructConstructor(struct_constructor) => {
                self.resolve_struct_contructor(struct_constructor)
            }
        }
    }

    fn resolve_struct_contructor(&mut self, struct_constructor: &StructConstructor) {
        for (_, value) in &struct_constructor.values {
            self.resolve_expression(*value);
        }
    }

    fn resolve_string_format(&mut self, string_format: &StringFormat) {
        for (_, value) in &string_format.parts {
            self.resolve_expression(*value);
        }
    }

    fn resolve_variable_expression(&mut self, variable: &VariableExpression) {
        let name = &variable.name;
        match self.scope_info.scopes.lookup_value(
            name.as_str(),
            ScopeValue::Variable,
            self.current.module,
        ) {
            Some(resolved) => _ = self.declares.insert_variable_resolve(variable.id, resolved),
            None => self.log_fault(Fault::error(
                format!("variable '{}' is undefined in scope", name.as_str()),
                Some(name.span()),
            )),
        }
    }

    fn resolve_lambda(&mut self, lambda: &Lambda) {
        self.resolve_expression(lambda.body);
    }

    fn resolve_match_method(&mut self, match_method: &MatchMethod) {
        self.resolve_expression(match_method.scrutinee);
        for arm in &match_method.arms {
            self.resolve_block(arm.body);
        }
    }

    fn resolve_field_access(&mut self, field_access: &FieldAccess) {
        self.resolve_expression(field_access.object);
    }

    fn resolve_contructor(&mut self, constructor: &Constructor) {
        for arg in &constructor.arguments {
            self.resolve_expression(arg.value);
        }
    }

    fn resolve_any_array(&mut self, any_array: &AnyArray) {
        match any_array {
            AnyArray::Array(array) => {
                for value in &array.values {
                    self.resolve_expression(*value);
                }
            }
            AnyArray::ArrayFiller(array_filler) => {
                self.resolve_expression(array_filler.amount);
                self.resolve_expression(array_filler.element);
            }
        }
    }

    fn resolve_match(&mut self, match_: &Match) {
        self.resolve_expression(match_.scrutinee);
        for arm in &match_.arms {
            self.resolve_block(arm.body);
        }
    }

    fn resolve_for(&mut self, for_: &For) {
        match &for_.condition {
            ForCondition::Loop => (),
            ForCondition::While(expression_id) => self.resolve_expression(*expression_id),
            ForCondition::Foreach { collection, .. } => {
                self.resolve_expression(*collection);
            }
        }

        self.resolve_block(for_.block);
    }

    fn resolve_if(&mut self, if_: &If) {
        self.resolve_expression(if_.condition);
        self.resolve_block(if_.block);

        let mut current = if_.branch.as_ref();
        while let Some(branch) = current {
            match branch.as_ref() {
                IfBranch::Else(block_id) => {
                    self.resolve_block(*block_id);
                    current = None;
                }
                IfBranch::If(elif) => {
                    self.resolve_expression(if_.condition);
                    self.resolve_block(elif.block);
                    current = elif.branch.as_ref();
                }
            }
        }
    }
}
