use ast_model::{
    AstStore,
    block::BlockId,
    expression::{
        AnyArray, Binding, Constructor, ExpressionId, ExpressionKind, For, ForCondition,
        FunctionCall, FunctionCalleeKind, If, IfBranch, Lambda, Match, MatchMethod, MatchPattern,
        StringFormat, StructConstructor, TypeOf, VariableExpression,
    },
};
use soul_utils::span::Span;

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(super) fn collect_expression(&mut self, expression_id: ExpressionId) {
        self.inner_collect_expression(expression_id);
    }

    fn inner_collect_expression(&mut self, expression_id: ExpressionId) {
        let Some(expression) = self.store.expressions.get(expression_id) else {
            return;
        };

        match &expression.node {
            ExpressionKind::Break
            | ExpressionKind::Null(_)
            | ExpressionKind::Continue
            | ExpressionKind::None(_)
            | ExpressionKind::Literal(_)
            | ExpressionKind::Variable(_)
            | ExpressionKind::Undefined(_) => (),

            ExpressionKind::If(if_) => self.collect_if(if_),
            ExpressionKind::New(value)
            | ExpressionKind::Pass(value)
            | ExpressionKind::Copy(value)
            | ExpressionKind::Sizeof(value) => self.collect_expression(*value),
            ExpressionKind::Return(value) => {
                if let Some(value) = value {
                    self.collect_expression(*value)
                }
            }

            ExpressionKind::Ref(ref_) => self.collect_expression(ref_.value),
            ExpressionKind::For(for_) => self.collect_for(for_),
            ExpressionKind::Unary(unary) => self.collect_expression(unary.value),
            ExpressionKind::Deref(deref) => self.collect_expression(deref.value),
            ExpressionKind::Index(index) => {
                self.collect_expression(index.index);
                self.collect_expression(index.collection);
            }

            ExpressionKind::Tuple(values) => {
                for value in values {
                    self.collect_expression(*value);
                }
            }
            ExpressionKind::NamedTuple(values) => {
                for (_, value) in values {
                    self.collect_expression(*value);
                }
            }

            ExpressionKind::Match(match_) => self.collect_match(match_),
            ExpressionKind::Binary(binary) => {
                self.collect_expression(binary.left);
                self.collect_expression(binary.right);
            }
            ExpressionKind::Block(block_id) => self.collect_block(*block_id),
            ExpressionKind::TypeOf(type_of) => self.collect_typeof(type_of),
            ExpressionKind::Array(any_array) => self.collect_any_array(any_array),
            ExpressionKind::NewArray(any_array) => self.collect_any_array(any_array),
            ExpressionKind::Constructor(constructor) => self.collect_constructor(constructor),
            ExpressionKind::MatchMethod(match_method) => self.collect_match_methode(match_method),
            ExpressionKind::FieldAccess(field_access) => {
                if let Some(expr) = self.store.expressions.get(field_access.object) {
                    if let ExpressionKind::Variable(VariableExpression { name, .. }) = &expr.node {
                        if self
                            .scope_info
                            .scopes
                            .flat_lookup_type(name.as_str(), self.current.module)
                            .is_some()
                        {
                            return;
                        }
                    }
                }
                self.collect_expression(field_access.object)
            }
            ExpressionKind::StringFormat(string_format) => {
                self.collect_string_format(string_format)
            }
            ExpressionKind::FunctionCall(function_call) => {
                self.collect_function_call(function_call)
            }
            ExpressionKind::StructConstructor(struct_constructor) => {
                self.collect_struct_constructor(struct_constructor)
            }
            ExpressionKind::Lambda(lambda) => self.collect_lambda(lambda),
        }
    }

    fn collect_lambda(&mut self, lambda: &Lambda) {
        self.push_scope(lambda.body);
        for param in &lambda.parameters {
            self.collect_var_pattern(param);
        }
        self.collect_scopeless_block(lambda.body);
        self.pop_scope();
    }

    fn collect_struct_constructor(&mut self, ctor: &StructConstructor) {
        self.collect_type(&ctor.struct_type);
        for (_, value) in &ctor.values {
            self.collect_expression(*value);
        }
    }

    fn collect_function_call(&mut self, call: &FunctionCall) {
        for ty in &call.generics {
            self.collect_type(ty);
        }

        if let Some(callee) = &call.callee {
            match &callee.kind {
                FunctionCalleeKind::Type(soul_type) => self.collect_type(soul_type),
                FunctionCalleeKind::Expression(expression_id) => {
                    self.collect_expression(*expression_id)
                }
            }
        }

        for arg in &call.arguments {
            self.collect_expression(arg.value);
        }
    }

    fn collect_string_format(&mut self, fmt: &StringFormat) {
        for (_, id) in &fmt.parts {
            self.collect_expression(*id);
        }
    }

    fn collect_match_methode(&mut self, match_methode: &MatchMethod) {
        fn block_span(store: &AstStore, block_id: BlockId) -> Span {
            store
                .blocks
                .get(block_id)
                .map(|block| block.span)
                .unwrap_or(Span::error())
        }

        self.collect_expression(match_methode.scrutinee);
        for arm in &match_methode.arms {
            self.push_scope(arm.body);
            self.collect_scopeless_block(arm.body);
            if let Some(binding) = &arm.binding {
                self.insert_binding(binding);
            } else {
                let id = self.alloc_node();
                let span = block_span(self.store, arm.body);
                self.insert_binding(&Binding::from_text(id, "it", span));
            }
            self.pop_scope();
        }
    }

    fn collect_constructor(&mut self, constructor: &Constructor) {
        self.collect_type(&constructor.ty);
        for argument in &constructor.arguments {
            self.collect_expression(argument.value);
        }
    }

    fn collect_any_array(&mut self, any_array: &AnyArray) {
        match any_array {
            AnyArray::Array(array) => {
                if let Some(ty) = &array.collection_type {
                    self.collect_type(ty);
                }

                if let Some(ty) = &array.element_type {
                    self.collect_type(ty);
                }

                for value in &array.values {
                    self.collect_expression(*value);
                }
            }
            AnyArray::ArrayFiller(array_filler) => {
                if let Some(ty) = &array_filler.collection_type {
                    self.collect_type(ty);
                }

                if let Some(ty) = &array_filler.element_type {
                    self.collect_type(ty);
                }

                if let Some(index) = &array_filler.for_index {
                    self.insert_binding(index);
                }

                self.collect_expression(array_filler.amount);
                self.collect_expression(array_filler.element);
            }
        }
    }

    fn collect_typeof(&mut self, type_of: &TypeOf) {
        self.collect_expression(type_of.value);
    }

    fn collect_match(&mut self, match_: &Match) {
        self.collect_expression(match_.scrutinee);
        for arm in &match_.arms {
            self.push_scope(arm.body);
            self.collect_match_arm_pattern(&arm.pattern);
            self.collect_scopeless_block(arm.body);
            self.pop_scope();
        }
    }

    fn collect_match_arm_pattern(&mut self, arm: &MatchPattern) {
        match arm {
            MatchPattern::Null => (),
            MatchPattern::Wildcard => (),
            MatchPattern::Literal(_) => (),
            MatchPattern::NotNull(binding) | MatchPattern::Binding(binding) => {
                self.insert_binding(binding)
            }
            MatchPattern::Array(match_patterns) => {
                for arm in match_patterns {
                    self.collect_match_arm_pattern(arm);
                }
            }
            MatchPattern::Fallthrough(_) => {
                todo!()
            }
            MatchPattern::Constructor(match_contructor) => {
                if let Some(binding) = &match_contructor.binding {
                    self.insert_binding(binding);
                }
            }
            MatchPattern::If {
                pattern,
                if_condition,
            } => {
                self.collect_expression(*if_condition);
                self.collect_match_arm_pattern(&pattern);
            }
            MatchPattern::Tuple(tuple) => {
                for element in &tuple.elements {
                    self.collect_match_arm_pattern(element);
                }
            }
            MatchPattern::NamedTuple(named) => {
                for field in &named.fields {
                    if let Some(binding) = &field.binding {
                        self.insert_binding(binding);
                    }
                }
            }
            MatchPattern::ConstructorStruct(struct_pat) => {
                for field in &struct_pat.fields {
                    if let Some(binding) = &field.binding {
                        self.insert_binding(binding);
                    }
                }
            }
            MatchPattern::Rest => (),
        }
    }

    fn collect_for(&mut self, for_: &For) {
        self.push_scope(for_.block);
        match &for_.condition {
            ForCondition::Loop => (),
            ForCondition::While(id) => self.collect_expression(*id),
            ForCondition::Foreach {
                element_kind,
                index,
                collection,
            } => {
                self.collect_var_pattern(element_kind);
                self.collect_expression(*collection);
                if let Some(binding) = index {
                    self.insert_binding(binding);
                }
            }
        }
        self.collect_scopeless_block(for_.block);
        self.pop_scope();
    }

    fn collect_if(&mut self, if_: &If) {

        self.push_scope(if_.block);
        self.inner_collect_expression(if_.condition);
        self.collect_scopeless_block(if_.block);
        self.pop_scope();

        let mut current = if_.branch.as_ref();
        while let Some(branch) = current {
            match branch.as_ref() {
                IfBranch::If(elif) => {
                    self.push_scope(elif.block);
                    self.inner_collect_expression(elif.condition);
                    self.collect_scopeless_block(elif.block);
                    self.pop_scope();
                    current = elif.branch.as_ref();
                }
                IfBranch::Else(block_id) => {
                    self.collect_block(*block_id);
                    current = None;
                }
            }
        }
    }
}
