//! AST-to-MIR lowering. First (smallest-testable) slice only: a single non-generic
//! function whose parameters/return/locals are all primitive scalars, whose body is
//! a flat sequence of `name := <expr>` variable declarations built from arithmetic
//! and (arbitrarily nested) sub-expressions, followed by exactly one `return <expr>`.
//! No control flow, no calls, no aggregates yet — each is a separate follow-on slice
//! (see `docs/mir-design.md` at the repo root).
//!
//! Anything outside that subset is rejected with a `Fault` rather than panicking or
//! silently mis-lowering — the same fault-reporting mechanism (`soul_utils::fault`)
//! every other pipeline stage uses, so a caller collects/prints lowering failures the
//! same way it collects parse or resolve faults. This pass runs on already-name-
//! resolved, well-typed input, so every fault here means "not supported by this
//! slice yet," not "the input program is invalid."

pub mod fault;
#[cfg(test)]
mod tests;

use ast_model::{
    self as ast, SoulType, declare_store::DeclareStore, operators::BinaryOperatorKind,
};

use fault::{MirErrorKind, MirResult};
use mir_model as mir;
use soul_utils::{
    FunctionId, TypeModifier, collections::vec_map::VecMap, fault::Fault, ids::IdGenerator,
    span::Span,
};

pub fn lower_function(
    store: &ast::AstStore,
    declares: &DeclareStore,
    function_id: FunctionId,
) -> MirResult<mir::Function> {
    let function = match &store.functions[function_id] {
        ast::FunctionKind::Normal(function) => function,
        ast::FunctionKind::Signature(signature) => {
            return Err(Fault::error_with_kind(
                MirErrorKind::SignatureOnlyFunctionHasNoBody,
                Some(signature.span),
            ));
        }
    };

    let signature = &function.signature.value;
    let fn_span = function.signature.span;
    let mut lowerer = Lowerer::new(store, declares);

    for parameter in &signature.parameters {
        let span = parameter.name.span();
        require_primitive(&parameter.ty, span)?;
        let modifier = parameter.mutable.to_type_modifier();
        let local = lowerer.alloc_local(parameter.ty.clone(), modifier, span);
        lowerer.node_to_local.insert(parameter.id, local);
    }

    let arg_count = signature.parameters.len();
    require_primitive(&signature.return_type, signature.name.span())?;
    let return_local = lowerer.alloc_local(
        signature.return_type.clone(),
        TypeModifier::Mut,
        signature.name.span(),
    );

    let mut statements = Vec::new();
    let mut terminator = None;

    let block = &store.blocks[function.block];
    for &id in &block.statements {
        let stmt = &store.statements[id];
        match &stmt.node {
            ast::StatementKind::Variable(var) => {
                lower_variable(declares, &mut lowerer, &mut statements, stmt, var)?;
            }
            ast::StatementKind::Expression { expression, .. } => {
                let expr = &store.expressions[*expression];
                let ast::ExpressionKind::Return(Some(value_id)) = &expr.node else {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::NonReturnTerminalStatementUnsupported,
                        Some(stmt.span),
                    ));
                };
                let rvalue = lowerer.lower_rvalue(*value_id, &mut statements)?;
                statements.push(mir_model::Statement::Assign(
                    mir::Place::local(return_local),
                    rvalue,
                ));
                terminator = Some(mir::Terminator::Return);
                break;
            }
            _ => {
                return Err(Fault::error_with_kind(
                    MirErrorKind::UnsupportedStatementKind,
                    Some(stmt.span),
                ));
            }
        }
    }

    let terminator = terminator.ok_or_else(|| {
        Fault::error_with_kind(MirErrorKind::MissingReturnStatement, Some(fn_span))
    })?;

    let mut blocks = VecMap::new();
    let bb0 = IdGenerator::<mir_model::BlockId>::new().alloc();
    blocks.insert(
        bb0,
        mir::BasicBlock {
            statements,
            terminator,
        },
    );

    Ok(mir::Function {
        name: function_id,
        locals: lowerer.locals,
        blocks,
        arg_count,
        return_local,
    })
}

fn lower_variable(
    declares: &DeclareStore,
    lowerer: &mut Lowerer<'_>,
    statements: &mut Vec<mir_model::Statement>,
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

    let ty = declares
        .get_variable_type(binding.id)
        .and_then(|(_, ty, _)| ty.clone())
        .ok_or_else(|| {
            Fault::error_with_kind(
                MirErrorKind::VariableHasNoResolvedType,
                Some(binding.ident.span()),
            )
        })?;

    require_primitive(&ty, binding.ident.span())?;
    let rvalue = lowerer.lower_rvalue(init, statements)?;
    let local = lowerer.alloc_local(ty, *modifier, binding.ident.span());
    lowerer.node_to_local.insert(binding.id, local);
    statements.push(mir::Statement::Assign(mir::Place::local(local), rvalue));
    Ok(())
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

struct Lowerer<'a> {
    store: &'a ast::AstStore,
    declares: &'a DeclareStore,
    locals: VecMap<mir::LocalId, mir_model::LocalDecl>,
    local_alloc: IdGenerator<mir::LocalId>,
    node_to_local: VecMap<ast::NodeId, mir::LocalId>,
}
impl<'a> Lowerer<'a> {
    fn new(store: &'a ast::AstStore, declares: &'a DeclareStore) -> Self {
        Self {
            store,
            declares,
            locals: VecMap::new(),
            local_alloc: IdGenerator::new(),
            node_to_local: VecMap::new(),
        }
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

    fn lower_rvalue(
        &mut self,
        expr_id: ast::ExpressionId,
        statements: &mut Vec<mir_model::Statement>,
    ) -> MirResult<mir::Rvalue> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ast::ExpressionKind::Binary(binary) => {
                if !is_supported_arithmetic_op(binary.operator.value) {
                    return Err(Fault::error_with_kind(
                        MirErrorKind::UnsupportedBinaryOperator,
                        Some(expr.span),
                    ));
                }
                let left = self.lower_operand(binary.left, statements)?;
                let right = self.lower_operand(binary.right, statements)?;
                Ok(mir::Rvalue::BinaryOp(binary.operator.value, left, right))
            }
            _ => Ok(mir::Rvalue::Use(self.lower_operand(expr_id, statements)?)),
        }
    }

    fn lower_operand(
        &mut self,
        expr_id: ast::ExpressionId,
        statements: &mut Vec<mir_model::Statement>,
    ) -> MirResult<mir::Operand> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ast::ExpressionKind::Literal((_, literal)) => {
                Ok(mir::Operand::Constant(literal.clone()))
            }
            ast::ExpressionKind::Variable(var) => {
                let resolved = self.declares.get_variable_resolve(var.id).ok_or_else(|| {
                    Fault::error_with_kind(
                        MirErrorKind::VariableHasNoResolvedBinding,
                        Some(expr.span),
                    )
                })?;
                let local = *self.node_to_local.get(resolved).ok_or_else(|| {
                    Fault::error_with_kind(MirErrorKind::VariableNotBoundToLocal, Some(expr.span))
                })?;
                Ok(mir::Operand::Copy(mir::Place::local(local)))
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

                let rvalue = self.lower_rvalue(expr_id, statements)?;
                let temp = self.alloc_local(ty, TypeModifier::Comptime, span);
                statements.push(mir::Statement::Assign(mir::Place::local(temp), rvalue));
                Ok(mir::Operand::Copy(mir::Place::local(temp)))
            }
            _ => Err(Fault::error_with_kind(
                MirErrorKind::UnsupportedOperandExpression,
                Some(expr.span),
            )),
        }
    }
}
