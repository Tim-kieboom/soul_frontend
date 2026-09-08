use ast_model::{
    self as ast, SoulType, declare_store::DeclareStore, operators::BinaryOperatorKind,
};
use mir_model::{self as mir, LocalDecl, LocalId, Terminator};
use soul_utils::{
    TypeModifier, collections::vec_map::VecMap, fault::Fault, ids::IdGenerator, span::Span,
};

use crate::fault::{MirErrorKind, MirResult};

pub struct FunctionLowerer<'a> {
    store: &'a ast::AstStore,
    declares: &'a DeclareStore,
    statements: Vec<mir::Statement>,
    terminator: Option<mir::Terminator>,
    local_alloc: IdGenerator<mir::LocalId>,
    node_to_local: VecMap<ast::NodeId, mir::LocalId>,
    locals: VecMap<mir::LocalId, mir_model::LocalDecl>,
}
impl<'a> FunctionLowerer<'a> {
    pub(crate) fn new(store: &'a ast::AstStore, declares: &'a DeclareStore) -> Self {
        Self {
            store,
            declares,
            terminator: None,
            statements: vec![],
            locals: VecMap::new(),
            local_alloc: IdGenerator::new(),
            node_to_local: VecMap::new(),
        }
    }

    pub(crate) fn lower(&mut self, function: &ast::Function) -> MirResult<mir::Function> {
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

        let block = &self.store.blocks[function.block];
        for &id in &block.statements {
            let statement = &self.store.statements[id];
            match &statement.node {
                ast::StatementKind::Variable(variable) => {
                    self.lower_variable(statement, variable)?;
                }
                ast::StatementKind::Expression { expression, .. } => {
                    self.lower_expression(return_local, statement, expression)?;
                    break;
                }
                _ => {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::UnsupportedStatementKind,
                        Some(statement.span),
                    ));
                }
            }
        }

        let (terminator, statements, locals) = self.drain_reset();
        let Some(terminator) = terminator else {
            return Err(Fault::error_with_kind(
                MirErrorKind::MissingReturnStatement,
                Some(fn_span),
            ));
        };

        let mut blocks = VecMap::new();
        let bb0 = IdGenerator::<mir_model::BlockId>::new().alloc();
        blocks.insert(
            bb0,
            mir::BasicBlock {
                terminator,
                statements,
            },
        );

        Ok(mir::Function {
            id: signature.id,
            locals,
            blocks,
            arg_count,
            return_local,
        })
    }

    fn drain_reset(
        &mut self,
    ) -> (
        Option<Terminator>,
        Vec<mir::Statement>,
        VecMap<LocalId, LocalDecl>,
    ) {
        use std::mem::take;
        self.node_to_local.clear();
        self.local_alloc = IdGenerator::new();

        (
            take(&mut self.terminator),
            take(&mut self.statements),
            take(&mut self.locals),
        )
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

    fn lower_expression(
        &mut self,
        return_local: mir::LocalId,
        stmt: &ast::Statement,
        expression: &ast::ExpressionId,
    ) -> MirResult<()> {
        let expr = &self.store.expressions[*expression];
        let ast::ExpressionKind::Return(Some(value_id)) = &expr.node else {
            return Err(Fault::error_with_kind(
                MirErrorKind::NonReturnTerminalStatementUnsupported,
                Some(stmt.span),
            ));
        };
        let rvalue = self.lower_rvalue(*value_id)?;
        self.statements.push(mir_model::Statement::Assign(
            mir::Place::local(return_local),
            rvalue,
        ));
        self.terminator = Some(mir::Terminator::Return);
        Ok(())
    }

    fn alloc_local(&mut self, ty: mir::Type, mutability: TypeModifier, span: Span) -> mir::LocalId {
        let id = self.local_alloc.alloc();
        self.locals.insert(
            id,
            mir_model::LocalDecl {
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
