use ast_model::{
    expression::{
        AnyArray, Binary, Constructor, ExpressionId, ExpressionKind, FieldAccess, For,
        ForCondition, If, IfBranch, IfCondition, Lambda, Match, MatchMethod, StringFormat,
        StructConstructor, VariableExpression,
    },
    literal::Literal,
    operators::BinaryOperatorKind,
    scope::ScopeValue,
    soul_type::SoulType,
};
use soul_utils::{fault::Fault, soul_error_internal, soul_names::PrimitiveTypes, span::Span};

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
                self.check_binary_expression(expression_id, expression.span, binary);
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
                self.resolve_variable_expression(variable);
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
                self.resolve_function_call(expression_id, function_call)
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
        self.resolve_block(lambda.body);
    }

    fn resolve_match_method(&mut self, match_method: &MatchMethod) {
        self.resolve_expression(match_method.scrutinee);
        for arm in &match_method.arms {
            self.resolve_block(arm.body);
        }
    }

    fn resolve_field_access(&mut self, field_access: &FieldAccess) {
        let Some(expr) = self.store.expressions.get(field_access.object) else {
            self.resolve_expression(field_access.object);
            return;
        };

        if let ExpressionKind::Variable(VariableExpression { name, .. }) = &expr.node {
            let name_str = name.as_str();
            if name_str == "intrinsic"
                || self.contains_type(name_str)
                || self.lookup_module(name_str).is_some()
                || self.is_crate_name(name_str)
            {
                return;
            }
        }

        self.resolve_expression(field_access.object);
    }

    fn is_crate_name(&mut self, name: &str) -> bool {
        if let Some(modules) = self.scope_info.scopes.iter_modules(self.current.module) {
            for (_, entry) in modules {
                if entry.crate_name.as_deref() == Some(name) {
                    return true;
                }
            }
        }
        false
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
        self.resolve_if_condition(&if_.condition);
        self.resolve_block(if_.block);

        let mut current = if_.branch.as_ref();
        while let Some(branch) = current {
            match branch {
                IfBranch::Else(block_id) => {
                    self.resolve_block(*block_id);
                    current = None;
                }
                IfBranch::If(elif) => {
                    self.resolve_if_condition(&if_.condition);
                    self.resolve_block(elif.block);
                    current = elif.branch.as_ref();
                }
            }
        }
    }

    fn resolve_if_condition(&mut self, condition: &IfCondition) {
        match condition {
            IfCondition::Expression(value) => self.resolve_expression(*value),
            IfCondition::CastType { scrutinee, .. } | IfCondition::MatchType { scrutinee, .. } => {
                self.resolve_expression(*scrutinee);
            }
        }
    }

    fn check_binary_expression(
        &mut self,
        expression_id: ExpressionId,
        span: Span,
        binary: &Binary,
    ) {
        let Some(left_ty) = self.expression_type(binary.left) else {
            return;
        };
        let Some(right_ty) = self.expression_type(binary.right) else {
            return;
        };

        let Some(combined) = combine_operand_types(&left_ty, &right_ty) else {
            self.log_fault(Fault::error(
                format!(
                    "type mismatch in binary expression: left is `{left_ty:?}`, right is `{right_ty:?}`"
                ),
                Some(span),
            ));
            return;
        };

        let result_ty = if is_comparison_operator(binary.operator.value) {
            SoulType::Primitive(PrimitiveTypes::Boolean)
        } else {
            combined
        };
        self.declares
            .insert_expression_type(expression_id, result_ty);
    }

    pub(super) fn expression_type(&self, expression_id: ExpressionId) -> Option<SoulType> {
        if let Some(ty) = self.declares.get_expression_type(expression_id) {
            return Some(ty.clone());
        }

        let expression = self.store.expressions.get(expression_id)?;
        match &expression.node {
            ExpressionKind::Literal((_, literal)) => Some(literal_type(literal)),
            ExpressionKind::Variable(variable) => {
                let resolved = self.declares.get_variable_resolve(variable.id)?;
                let (_, ty, _) = self.declares.get_variable_type(resolved)?;
                ty.clone()
            }
            _ => None,
        }
    }
}

fn literal_type(literal: &Literal) -> SoulType {
    match literal {
        Literal::Int(_) => SoulType::Primitive(PrimitiveTypes::UntypedInt),
        Literal::Uint(_) => SoulType::Primitive(PrimitiveTypes::UntypedUint),
        Literal::Float(_) => SoulType::Primitive(PrimitiveTypes::UntypedFloat),
        Literal::Bool(_) => SoulType::Primitive(PrimitiveTypes::Boolean),
        Literal::Char(_) => SoulType::Primitive(PrimitiveTypes::Char),
        Literal::Str(_) => SoulType::String,
        Literal::Cstr(_) => SoulType::Primitive(PrimitiveTypes::CStr),
    }
}

fn is_comparison_operator(operator: BinaryOperatorKind) -> bool {
    matches!(
        operator,
        BinaryOperatorKind::Eq
            | BinaryOperatorKind::NotEq
            | BinaryOperatorKind::Lt
            | BinaryOperatorKind::Gt
            | BinaryOperatorKind::Le
            | BinaryOperatorKind::Ge
            | BinaryOperatorKind::LogAnd
            | BinaryOperatorKind::LogOr
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NumCategory {
    Int,
    Uint,
    Float,
}

fn concrete_num_category(prim: PrimitiveTypes) -> Option<NumCategory> {
    match prim {
        PrimitiveTypes::CInt
        | PrimitiveTypes::Int
        | PrimitiveTypes::Int8
        | PrimitiveTypes::Int16
        | PrimitiveTypes::Int32
        | PrimitiveTypes::Int64
        | PrimitiveTypes::Int128 => Some(NumCategory::Int),
        PrimitiveTypes::CUint
        | PrimitiveTypes::Uint
        | PrimitiveTypes::Uint8
        | PrimitiveTypes::Uint16
        | PrimitiveTypes::Uint32
        | PrimitiveTypes::Uint64
        | PrimitiveTypes::Uint128 => Some(NumCategory::Uint),
        PrimitiveTypes::Float16 | PrimitiveTypes::Float32 | PrimitiveTypes::Float64 => {
            Some(NumCategory::Float)
        }
        _ => None,
    }
}

fn untyped_targets(kind: PrimitiveTypes) -> &'static [NumCategory] {
    match kind {
        PrimitiveTypes::UntypedInt => &[NumCategory::Int, NumCategory::Float],
        PrimitiveTypes::UntypedUint => &[NumCategory::Int, NumCategory::Uint, NumCategory::Float],
        PrimitiveTypes::UntypedFloat => &[NumCategory::Float],
        _ => &[],
    }
}

fn untyped_kind_of(ty: &SoulType) -> Option<PrimitiveTypes> {
    match ty {
        SoulType::Primitive(
            kind @ (PrimitiveTypes::UntypedInt
            | PrimitiveTypes::UntypedUint
            | PrimitiveTypes::UntypedFloat),
        ) => Some(*kind),
        _ => None,
    }
}

fn combine_untyped_kinds(a: PrimitiveTypes, b: PrimitiveTypes) -> PrimitiveTypes {
    if a == PrimitiveTypes::UntypedFloat || b == PrimitiveTypes::UntypedFloat {
        PrimitiveTypes::UntypedFloat
    } else {
        PrimitiveTypes::UntypedInt
    }
}

pub(super) fn combine_operand_types(left: &SoulType, right: &SoulType) -> Option<SoulType> {
    match (untyped_kind_of(left), untyped_kind_of(right)) {
        (None, None) if left == right => Some(left.clone()),
        (None, None) => None,
        (Some(kind), None) => coerce_untyped_to_concrete(kind, right),
        (None, Some(kind)) => coerce_untyped_to_concrete(kind, left),
        (Some(a), Some(b)) => Some(SoulType::Primitive(combine_untyped_kinds(a, b))),
    }
}

fn coerce_untyped_to_concrete(kind: PrimitiveTypes, concrete: &SoulType) -> Option<SoulType> {
    let SoulType::Primitive(prim) = concrete else {
        return None;
    };

    let category = concrete_num_category(*prim)?;
    if untyped_targets(kind).contains(&category) {
        Some(concrete.clone())
    } else {
        None
    }
}
