use ast::FunctionKind;
use hir::{
    Assign, Block, BlockId, ExpressionId, Global, GlobalKind, LazyTypeId, Statement, StatementKind,
    Terminator, TypeId, Variable,
};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    ids::{FunctionId, IdAlloc},
    soul_error_internal,
    soul_names::TypeModifier,
    span::Span,
};

use crate::TypedHirContext;

impl<'a> TypedHirContext<'a> {
    pub(crate) fn infer_global(&mut self, global: &Global) {
        let id = global.id;
        let ty = match &global.kind {
            GlobalKind::InternalAssign(assign) => self.infer_assign(assign).to_lazy(),
            GlobalKind::Variable(variable) | GlobalKind::InternalVariable(variable) => {
                self.infer_variable(variable, self.statement_span(id))
            }
            GlobalKind::Function(function) => {
                self.infer_function(*function);
                return;
            }
        };

        if global.should_be_inmutable() && is_mutable(self.lazy_id_get_modifier(ty)) {
            let span = global.get_span(&self.hir.info.spans);
            self.log_error(SoulError::new(
                "can not have 'mut' in global scope",
                SoulErrorKind::InvalidContext,
                Some(span),
            ));
        } else {
            self.type_statement(global.id, ty);
        }
    }

    pub(crate) fn infer_statement(&mut self, statement: &Statement) {
        let ty = match &statement.kind {
            StatementKind::Assign(assign) => self.infer_assign(assign).to_lazy(),
            StatementKind::Break | StatementKind::Continue => self.common_types.none_type.to_lazy(),
            StatementKind::Variable(variable) => {
                self.infer_variable(variable, self.statement_span(statement.id))
            }
            StatementKind::Expression { value, .. } => self.infer_expression(*value),

            StatementKind::Fall(value) => {
                if let Some(value) = value {
                    let span = self.hir.info.spans.expressions[*value];
                    self.log_error(soul_error_internal!(
                        "fall with value is not yet impl",
                        Some(span)
                    ));
                }
                return;
            }

            StatementKind::Return(value) => match *value {
                Some(val) => self.infer_expression(val),
                None => self.common_types.none_type.to_lazy(),
            },
        };

        self.type_statement(statement.id, ty);
    }

    fn infer_assign(&mut self, assign: &Assign) -> TypeId {
        let place_span = self.place_span(assign.place);
        let expected = self.infer_place(assign.place);
        if !self.is_mutable_or_modifier_none(expected) {
            let modifier_str = self
                .get_type_modifier(expected)
                .map(|m| m.as_str())
                .unwrap_or("<empty>");

            self.log_error(SoulError::new(
                format!("trying to reassign but typeModifier is '{modifier_str}' (make it 'mut' instead)"),
                SoulErrorKind::InvalidMutability,
                Some(place_span),
            ));
            return TypeId::error();
        }

        let value_span = self.expression_span(assign.value);
        let value_type = self.infer_expression(assign.value);
        self.unify(assign.value, expected, value_type, value_span);

        self.resolve_type_strict(value_type, value_span.combine(place_span))
            .unwrap_or(TypeId::error())
    }

    fn infer_function(&mut self, function_id: FunctionId) -> TypeId {
        self.current_function = Some(function_id);
        let function = &self.hir.nodes.functions[function_id];

        for (i, parameter) in function.parameters.iter().enumerate() {
            let modifier = self.lazy_id_get_modifier(parameter.ty).unwrap_or_else(|| {
                if i == 0 && matches!(function.kind, FunctionKind::MutRef) {
                    TypeModifier::Mut
                } else {
                    TypeModifier::Const
                }
            });

            let span = self.hir.info.spans.locals[parameter.local];
            self.type_local(parameter.local, parameter.ty, modifier, span);
        }

        let body = match function.body {
            hir::FunctionBody::Internal(block_id) => block_id,
            hir::FunctionBody::External(_) => return function.return_type,
        };

        let span = self.hir.info.spans.blocks[body];
        let block_type = self.infer_block_returnable(body);
        self.unify(
            ExpressionId::error(),
            function.return_type.to_lazy(),
            block_type,
            span,
        );

        self.current_function = None;
        function.return_type
    }

    pub(crate) fn infer_block_expression(&mut self, body: BlockId) -> LazyTypeId {
        self.inner_infer_block(body, false)
    }

    pub(crate) fn infer_block_returnable(&mut self, body: BlockId) -> LazyTypeId {
        self.inner_infer_block(body, true)
    }

    fn inner_infer_block(&mut self, body: BlockId, returnable: bool) -> LazyTypeId {
        let mut return_type = None;

        let block = &self.hir.nodes.blocks[body];
        for statement in &block.statements {
            self.infer_statement(statement);
            if !returnable {
                continue;
            }

            if let hir::StatementKind::Return(value) = statement.kind {
                let span = self.statement_span(statement.id);
                let got = match value {
                    Some(val) => self.infer_expression(val),
                    None => self.common_types.none_type.to_lazy(),
                };

                let value = value.unwrap_or(ExpressionId::error());
                match return_type {
                    Some(ty) => _ = self.unify(value, ty, got, span),
                    None => return_type = Some(got),
                }
            }
        }

        self.handle_block_terminator(block, &mut return_type, returnable);

        let ty = match return_type {
            Some(ty) => ty,
            None => {
                if !returnable && self.block_has_return(block) {
                    self.common_types.never_type.to_lazy()
                } else {
                    self.common_types.none_type.to_lazy()
                }
            }
        };
        self.type_block(body, ty);
        ty
    }

    fn block_has_return(&self, block: &Block) -> bool {
        if matches!(block.terminator, Some(Terminator::Return(_))) {
            return true;
        }
        block
            .statements
            .iter()
            .any(|s| matches!(s.kind, hir::StatementKind::Return(_)))
    }

    fn handle_block_terminator(
        &mut self,
        block: &Block,
        return_type: &mut Option<LazyTypeId>,
        returnable: bool,
    ) {
        if let Some(terminator) = block.terminator {
            if matches!(terminator, Terminator::Return(_)) && !returnable {
                *return_type = Some(self.common_types.never_type.to_lazy());
                return;
            }

            let value = terminator.get_expression_id();
            let span = self.expression_span(value);
            let got = self.infer_expression(value);
            match return_type {
                Some(ty) => _ = self.unify(value, *ty, got, span),
                None => *return_type = Some(got),
            }
        }
    }

    fn infer_variable(&mut self, variable: &Variable, span: Span) -> LazyTypeId {
        let declared_type_id = self.get_variable_type(variable);
        if declared_type_id == LazyTypeId::error() {
            let modifier = self
                .lazy_id_get_modifier(declared_type_id)
                .unwrap_or(TypeModifier::Const);
            self.type_local(variable.local, declared_type_id, modifier, span);
            return declared_type_id;
        }

        let (value, span) = match self.get_variable_value(variable) {
            Some(val) => (val, self.expression_span(val)),
            None => {
                let modifier = self
                    .lazy_id_get_modifier(declared_type_id)
                    .unwrap_or(TypeModifier::Const);
                self.type_local(variable.local, declared_type_id, modifier, span);
                return declared_type_id;
            }
        };

        let value_type_id = self.infer_expression(value);
        self.unify(value, declared_type_id, value_type_id, span);

        let mut variable_type_id = match declared_type_id {
            LazyTypeId::Known(_) => declared_type_id,
            LazyTypeId::Infer(_) => value_type_id,
        };

        let modifier = self
            .lazy_id_get_modifier(declared_type_id)
            .unwrap_or(TypeModifier::Const);
        variable_type_id = self.resolve_type_lazy(variable_type_id, span);

        self.type_local(variable.local, variable_type_id, modifier, span)
    }
}

fn is_mutable(mo: Option<TypeModifier>) -> bool {
    mo == Some(TypeModifier::Mut)
}
