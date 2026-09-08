use ast_model::{
    self as ast, SoulType, declare_store::DeclareStore, operators::BinaryOperatorKind,
};
use mir_model as mir;
use soul_utils::{
    TypeModifier, collections::vec_map::VecMap, fault::Fault, ids::IdGenerator, span::Span,
};

use crate::fault::{MirErrorKind, MirResult};

/// The `(header, exit)` block pair of an enclosing loop, so a nested `break`/
/// `continue` can find its target regardless of how deep inside `if`s it is.
/// Soul has no labeled break/continue, so this is a plain stack: `break`/
/// `continue` always target the innermost entry.
struct LoopTargets {
    header: mir::BlockId,
    exit: mir::BlockId,
}

pub struct FunctionLowerer<'a> {
    store: &'a ast::AstStore,
    declares: &'a DeclareStore,
    local_alloc: IdGenerator<mir::LocalId>,
    node_to_local: VecMap<ast::NodeId, mir::LocalId>,
    locals: VecMap<mir::LocalId, mir::LocalDecl>,

    block_alloc: IdGenerator<mir::BlockId>,
    blocks: VecMap<mir::BlockId, mir::BasicBlock>,

    current: Option<mir::BlockId>,
    statements: Vec<mir::Statement>,
    loops: Vec<LoopTargets>,
}
impl<'a> FunctionLowerer<'a> {
    pub(crate) fn new(store: &'a ast::AstStore, declares: &'a DeclareStore) -> Self {
        Self {
            store,
            declares,
            current: None,
            loops: vec![],
            statements: vec![],
            locals: VecMap::new(),
            blocks: VecMap::new(),
            node_to_local: VecMap::new(),
            local_alloc: IdGenerator::new(),
            block_alloc: IdGenerator::new(),
        }
    }

    pub(crate) fn lower(&mut self, function: &ast::Function) -> MirResult<mir::Function> {
        self.reset();

        let signature = &function.signature.value;
        let fn_span = function.signature.span;

        for parameter in &signature.parameters {
            let span = parameter.name.span();
            require_primitive(&parameter.ty, span)?;
            let modifier = parameter.mutable.to_type_modifier();
            let local = self.alloc_local(parameter.ty.clone(), modifier, span);
            self.node_to_local.insert(parameter.id, local);
        }

        let arg_count = signature.parameters.len();
        require_primitive(&signature.return_type, signature.name.span())?;
        let return_local = self.alloc_local(
            signature.return_type.clone(),
            TypeModifier::Mut,
            signature.name.span(),
        );

        let entry = self.new_block();
        self.current = Some(entry);

        let statement_ids = self.store.blocks[function.block].statements.clone();
        self.lower_body(return_local, &statement_ids)?;

        if self.current.is_some() {
            return Err(Fault::error_with_kind(
                MirErrorKind::MissingReturnStatement,
                Some(fn_span),
            ));
        }

        Ok(mir::Function {
            id: signature.id,
            locals: std::mem::take(&mut self.locals),
            blocks: std::mem::take(&mut self.blocks),
            arg_count,
            return_local,
        })
    }

    fn reset(&mut self) {
        self.loops.clear();
        self.current = None;
        self.blocks.clear();
        self.locals.clear();
        self.statements.clear();
        self.node_to_local.clear();
        self.local_alloc = IdGenerator::new();
        self.block_alloc = IdGenerator::new();
    }

    fn new_block(&mut self) -> mir::BlockId {
        self.block_alloc.alloc()
    }

    fn is_terminated(&self) -> bool {
        self.current.is_none()
    }

    fn seal(&mut self, terminator: mir::Terminator, next: Option<mir::BlockId>) {
        if let Some(current) = self.current.take() {
            let statements = std::mem::take(&mut self.statements);
            self.blocks.insert(
                current,
                mir::BasicBlock {
                    terminator,
                    statements,
                },
            );
        }
        self.current = next;
    }

    fn lower_body(
        &mut self,
        return_local: mir::LocalId,
        statements: &[ast::StatementId],
    ) -> MirResult<()> {
        for &id in statements {
            let statement = &self.store.statements[id];
            if self.is_terminated() {
                return Err(Fault::error_with_kind(
                    MirErrorKind::UnreachableStatement,
                    Some(statement.span),
                ));
            }
            self.lower_statement(return_local, statement)?;
        }
        Ok(())
    }

    fn lower_statement(
        &mut self,
        return_local: mir::LocalId,
        statement: &ast::Statement,
    ) -> MirResult<()> {
        match &statement.node {
            ast::StatementKind::Variable(variable) => self.lower_variable(statement, variable),
            ast::StatementKind::Expression { expression, .. } => {
                self.lower_expression_statement(return_local, statement, *expression)
            }
            _ => Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedStatementKind,
                Some(statement.span),
            )),
        }
    }

    fn lower_expression_statement(
        &mut self,
        return_local: mir::LocalId,
        stmt: &ast::Statement,
        expression: ast::ExpressionId,
    ) -> MirResult<()> {
        let expr = &self.store.expressions[expression];
        match &expr.node {
            ast::ExpressionKind::Return(Some(value_id)) => {
                let rvalue = self.lower_rvalue(*value_id)?;
                self.statements.push(mir::Statement::Assign(
                    mir::Place::local(return_local),
                    rvalue,
                ));
                self.seal(mir::Terminator::Return, None);
                Ok(())
            }
            ast::ExpressionKind::If(if_expr) => self.lower_if(return_local, if_expr, expr.span),
            ast::ExpressionKind::For(for_expr) => self.lower_for(return_local, for_expr, expr.span),
            ast::ExpressionKind::Break => self.lower_break(expr.span),
            ast::ExpressionKind::Continue => self.lower_continue(expr.span),
            _ => Err(Fault::error_with_kind(
                MirErrorKind::NonReturnTerminalStatementUnsupported,
                Some(stmt.span),
            )),
        }
    }

    fn lower_if(
        &mut self,
        return_local: mir::LocalId,
        if_expr: &ast::If,
        span: Span,
    ) -> MirResult<()> {
        let ast::IfCondition::Expression(condition_id) = &if_expr.condition else {
            return Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedConditionExpression,
                Some(span),
            ));
        };
        let discriminant = self.lower_bool_condition(*condition_id)?;

        let then_id = self.new_block();
        let join_id = self.new_block();
        let else_id = if_expr.branch.as_ref().map(|_| self.new_block());
        let false_target = else_id.unwrap_or(join_id);

        self.seal(
            mir::Terminator::SwitchInt {
                discriminant,
                targets: vec![(mir::ConstValue::Bool(true), then_id)],
                otherwise: false_target,
            },
            Some(then_id),
        );

        let then_statements = self.store.blocks[if_expr.block].statements.clone();
        self.lower_body(return_local, &then_statements)?;
        let then_reaches_join = !self.is_terminated();
        if then_reaches_join {
            self.seal(mir::Terminator::Goto(join_id), None);
        }

        let else_reaches_join = match &if_expr.branch {
            None => true,
            Some(branch) => {
                self.current = else_id;
                match branch {
                    ast::IfBranch::Else(block_id) => {
                        let else_statements = self.store.blocks[*block_id].statements.clone();
                        self.lower_body(return_local, &else_statements)?;
                    }
                    ast::IfBranch::If(nested_if) => {
                        self.lower_if(return_local, nested_if, span)?;
                    }
                }
                
                let reaches = !self.is_terminated();
                if reaches {
                    self.seal(mir::Terminator::Goto(join_id), None);
                }
                reaches
            }
        };

        self.current = (then_reaches_join || else_reaches_join).then_some(join_id);
        Ok(())
    }

    fn lower_for(
        &mut self,
        return_local: mir::LocalId,
        for_expr: &ast::For,
        span: Span,
    ) -> MirResult<()> {
        let ast::ForCondition::While(condition_id) = &for_expr.condition else {
            return Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedLoopCondition,
                Some(span),
            ));
        };

        let header_id = self.new_block();
        let body_id = self.new_block();
        let exit_id = self.new_block();

        self.seal(mir::Terminator::Goto(header_id), Some(header_id));

        let discriminant = self.lower_bool_condition(*condition_id)?;
        self.seal(
            mir::Terminator::SwitchInt {
                discriminant,
                targets: vec![(mir::ConstValue::Bool(true), body_id)],
                otherwise: exit_id,
            },
            Some(body_id),
        );

        self.loops.push(LoopTargets {
            header: header_id,
            exit: exit_id,
        });
        let body_statements = self.store.blocks[for_expr.block].statements.clone();
        let body_result = self.lower_body(return_local, &body_statements);
        self.loops.pop();
        body_result?;

        if !self.is_terminated() {
            self.seal(mir::Terminator::Goto(header_id), None);
        }

        // The exit block is always reachable via the header's false edge,
        // regardless of how the body ended.
        self.current = Some(exit_id);
        Ok(())
    }

    fn lower_break(&mut self, span: Span) -> MirResult<()> {
        let Some(target) = self.loops.last().map(|l| l.exit) else {
            return Err(Fault::error_with_kind(
                MirErrorKind::BreakOutsideLoop,
                Some(span),
            ));
        };
        self.seal(mir::Terminator::Goto(target), None);
        Ok(())
    }

    fn lower_continue(&mut self, span: Span) -> MirResult<()> {
        let Some(target) = self.loops.last().map(|l| l.header) else {
            return Err(Fault::error_with_kind(
                MirErrorKind::ContinueOutsideLoop,
                Some(span),
            ));
        };
        self.seal(mir::Terminator::Goto(target), None);
        Ok(())
    }

    fn lower_bool_condition(&mut self, expr_id: ast::ExpressionId) -> MirResult<mir::Operand> {
        let expr = &self.store.expressions[expr_id];
        if !matches!(
            &expr.node,
            ast::ExpressionKind::Literal((_, ast::Literal::Bool(_)))
        ) {
            return Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedConditionExpression,
                Some(expr.span),
            ));
        }
        self.lower_operand(expr_id)
    }

    fn lower_variable(
        &mut self,
        stmt: &ast::Statement,
        var: &ast::Variable,
    ) -> Result<(), Fault<MirErrorKind>> {
        let ast::VarPattern::Simple { binding, modifier } = &var.pattern else {
            return Err(Fault::error_with_kind(
                MirErrorKind::NonSimpleVariablePatternUnsupported,
                Some(stmt.span),
            ));
        };

        let Some(init) = var.initialize_value else {
            return Err(Fault::error_with_kind(
                MirErrorKind::UninitializedVariableUnsupported,
                Some(stmt.span),
            ));
        };

        let ty = self
            .declares
            .get_variable_type(binding.id)
            .and_then(|(_, ty, _)| ty.clone())
            .ok_or_else(|| {
                Fault::error_with_kind(
                    MirErrorKind::VariableHasNoResolvedType,
                    Some(binding.ident.span()),
                )
            })?;

        require_primitive(&ty, binding.ident.span())?;
        let rvalue = self.lower_rvalue(init)?;
        let local = self.alloc_local(ty, *modifier, binding.ident.span());
        self.node_to_local.insert(binding.id, local);
        self.statements
            .push(mir::Statement::Assign(mir::Place::local(local), rvalue));
        Ok(())
    }

    fn alloc_local(&mut self, ty: mir::Type, mutability: TypeModifier, span: Span) -> mir::LocalId {
        let id = self.local_alloc.alloc();
        self.locals.insert(
            id,
            mir::LocalDecl {
                ty,
                mutability,
                span,
            },
        );
        id
    }

    fn lower_rvalue(&mut self, expr_id: ast::ExpressionId) -> MirResult<mir::Rvalue> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ast::ExpressionKind::Binary(binary) => {
                if !is_supported_arithmetic_op(binary.operator.value) {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::UnsupportedBinaryOperator,
                        Some(expr.span),
                    ));
                }
                let left = self.lower_operand(binary.left)?;
                let right = self.lower_operand(binary.right)?;
                Ok(mir::Rvalue::BinaryOp(binary.operator.value, left, right))
            }
            _ => Ok(mir::Rvalue::Use(self.lower_operand(expr_id)?)),
        }
    }

    fn lower_operand(&mut self, expr_id: ast::ExpressionId) -> MirResult<mir::Operand> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ast::ExpressionKind::Literal((_, literal)) => {
                Ok(mir::Operand::Constant(literal.clone()))
            }
            ast::ExpressionKind::Variable(var) => {
                let Some(resolved) = self.declares.get_variable_resolve(var.id) else {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::VariableHasNoResolvedBinding,
                        Some(expr.span),
                    ));
                };

                let Some(local) = self.node_to_local.get(resolved) else {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::VariableNotBoundToLocal,
                        Some(expr.span),
                    ));
                };

                Ok(mir::Operand::Copy(mir::Place::local(*local)))
            }
            ast::ExpressionKind::Binary(_) => {
                let span = expr.span;
                let ty = self
                    .declares
                    .get_expression_type(expr_id)
                    .cloned()
                    .ok_or_else(|| {
                        Fault::error_with_kind(
                            MirErrorKind::NestedExpressionHasNoResolvedType,
                            Some(span),
                        )
                    })?;

                require_primitive(&ty, span)?;

                let rvalue = self.lower_rvalue(expr_id)?;
                let temp = self.alloc_local(ty, TypeModifier::Comptime, span);
                self.statements
                    .push(mir::Statement::Assign(mir::Place::local(temp), rvalue));
                Ok(mir::Operand::Copy(mir::Place::local(temp)))
            }
            _ => Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedOperandExpression,
                Some(expr.span),
            )),
        }
    }
}

fn require_primitive(ty: &SoulType, span: Span) -> MirResult<()> {
    if matches!(ty, SoulType::Primitive(_)) {
        Ok(())
    } else {
        Err(Fault::error_with_kind(
            MirErrorKind::NonPrimitiveType {
                ty: format!("{ty:?}").into(),
            },
            Some(span),
        ))
    }
}

fn is_supported_arithmetic_op(op: BinaryOperatorKind) -> bool {
    matches!(
        op,
        BinaryOperatorKind::Add
            | BinaryOperatorKind::Sub
            | BinaryOperatorKind::Mul
            | BinaryOperatorKind::Div
            | BinaryOperatorKind::Mod
    )
}
