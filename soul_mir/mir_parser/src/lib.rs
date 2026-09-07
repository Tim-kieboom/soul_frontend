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

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use ast_model::{
    AstStore, FunctionKind, NodeId,
    declare_store::DeclareStore,
    expression::ExpressionId,
    expression::ExpressionKind,
    operators::BinaryOperatorKind,
    soul_type::SoulType,
    statements::{StatementKind, VarPattern},
};
use mir_model::{BasicBlock, LocalId, MirFunction, MirType, Operand, Place, Rvalue, Terminator};
use soul_utils::{
    FunctionId, TypeModifier, collections::vec_map::VecMap, error::SoulResult, fault::Fault,
    ids::IdGenerator, span::Span,
};

pub fn lower_function(
    store: &AstStore,
    declares: &DeclareStore,
    function_id: FunctionId,
) -> SoulResult<MirFunction> {
    let function = match &store.functions[function_id] {
        FunctionKind::Normal(function) => function,
        FunctionKind::Signature(signature) => {
            return Err(Fault::error(
                "extern/signature-only declarations have no body to lower to MIR",
                Some(signature.span),
            ));
        }
    };
    let signature = &function.signature.value;
    let fn_span = function.signature.span;

    let mut lowerer = Lowerer {
        store,
        declares,
        locals: VecMap::new(),
        local_gen: IdGenerator::new(),
        node_to_local: HashMap::new(),
    };

    for param in &signature.parameters {
        require_primitive(&param.ty, param.name.span())?;
        let modifier = if param.is_mut {
            TypeModifier::Mut
        } else {
            TypeModifier::Const
        };
        let local = lowerer.alloc_local(param.ty.clone(), modifier, param.name.span());
        lowerer.node_to_local.insert(param.id, local);
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
    for &stmt_id in &block.statements {
        let stmt = &store.statements[stmt_id];
        match &stmt.node {
            StatementKind::Variable(var) => {
                let VarPattern::Simple { binding, modifier } = &var.pattern else {
                    return Err(Fault::error(
                        "only simple (non-destructuring) variable bindings are supported in this lowering slice",
                        Some(stmt.span),
                    ));
                };
                let Some(init) = var.initialize_value else {
                    return Err(Fault::error(
                        "a variable declaration with no initializer isn't supported in this lowering slice",
                        Some(stmt.span),
                    ));
                };
                let ty = declares
                    .get_variable_type(binding.id)
                    .and_then(|(_, ty, _)| ty.clone())
                    .ok_or_else(|| {
                        Fault::error("variable has no resolved type", Some(binding.ident.span()))
                    })?;

                require_primitive(&ty, binding.ident.span())?;

                let rvalue = lowerer.lower_rvalue(init, &mut statements)?;
                let local = lowerer.alloc_local(ty, *modifier, binding.ident.span());
                lowerer.node_to_local.insert(binding.id, local);
                statements.push(mir_model::Statement::Assign(Place::local(local), rvalue));
            }
            StatementKind::Expression { expression, .. } => {
                let expr = &store.expressions[*expression];
                let ExpressionKind::Return(Some(value_id)) = &expr.node else {
                    return Err(Fault::error(
                        "only a `return <expr>` statement is supported as a function's terminal statement in this lowering slice",
                        Some(stmt.span),
                    ));
                };
                let rvalue = lowerer.lower_rvalue(*value_id, &mut statements)?;
                statements.push(mir_model::Statement::Assign(
                    Place::local(return_local),
                    rvalue,
                ));
                terminator = Some(Terminator::Return);
                break;
            }
            _ => {
                return Err(Fault::error(
                    "this statement kind isn't supported in this lowering slice",
                    Some(stmt.span),
                ));
            }
        }
    }

    let terminator = terminator.ok_or_else(|| {
        Fault::error(
            "function has no `return <expr>` as its final reachable statement",
            Some(fn_span),
        )
    })?;

    // A single basic block is all this slice ever produces; a real block
    // generator is introduced once control flow (the next slice) needs more than
    // one, so a fresh one-shot allocator is enough here.
    let mut blocks = VecMap::new();
    let bb0 = IdGenerator::<mir_model::BlockId>::new().alloc();
    blocks.insert(
        bb0,
        BasicBlock {
            statements,
            terminator,
        },
    );

    Ok(MirFunction {
        name: function_id,
        locals: lowerer.locals,
        blocks,
        arg_count,
        return_local,
    })
}

/// Every primitive scalar is `AutoCopy` per `soul-lang.md` §11, which is what lets
/// this slice always lower a variable read as `Operand::Copy` without checking a
/// real `AutoCopy` bound (that check doesn't exist yet in the resolver) — so
/// non-primitive types are out of scope until it does.
fn require_primitive(ty: &SoulType, span: Span) -> SoulResult<()> {
    if matches!(ty, SoulType::Primitive(_)) {
        Ok(())
    } else {
        Err(Fault::error(
            format!(
                "type `{ty:?}` isn't a primitive scalar, which is all this lowering slice supports"
            ),
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
    store: &'a AstStore,
    declares: &'a DeclareStore,
    locals: VecMap<LocalId, mir_model::LocalDecl>,
    local_gen: IdGenerator<LocalId>,
    node_to_local: HashMap<NodeId, LocalId>,
}

impl<'a> Lowerer<'a> {
    fn alloc_local(&mut self, ty: MirType, mutability: TypeModifier, span: Span) -> LocalId {
        let id = self.local_gen.alloc();
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

    /// An `Rvalue`: either a flat binary op between two operands, or a bare
    /// operand. Each operand may itself be an arbitrarily nested expression —
    /// see `lower_operand` for how that's flattened into three-address form.
    fn lower_rvalue(
        &mut self,
        expr_id: ExpressionId,
        statements: &mut Vec<mir_model::Statement>,
    ) -> SoulResult<Rvalue> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ExpressionKind::Binary(binary) => {
                if !is_supported_arithmetic_op(binary.operator.value) {
                    return Err(Fault::error(
                        "only arithmetic binary operators (+ - * / %) are supported in this lowering slice",
                        Some(expr.span),
                    ));
                }
                let left = self.lower_operand(binary.left, statements)?;
                let right = self.lower_operand(binary.right, statements)?;
                Ok(Rvalue::BinaryOp(binary.operator.value, left, right))
            }
            _ => Ok(Rvalue::Use(self.lower_operand(expr_id, statements)?)),
        }
    }

    /// An operand every `Rvalue` consumes: a literal constant, a read of an
    /// already-lowered local, or — for a nested compound expression like
    /// `b * c` inside `a + b * c` — a fresh temporary. The temporary is lowered
    /// three-address-code style: recursively lower the sub-expression to an
    /// `Rvalue`, assign it to a new local sized by the sub-expression's
    /// resolver-computed type, and read that local back as the operand. All
    /// primitive scalars are `AutoCopy` (soul-lang.md §11), so every variable
    /// (and every temporary this creates) reads back as `Operand::Copy` in this
    /// slice — never `Move`.
    fn lower_operand(
        &mut self,
        expr_id: ExpressionId,
        statements: &mut Vec<mir_model::Statement>,
    ) -> SoulResult<Operand> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ExpressionKind::Literal((_, literal)) => Ok(Operand::Constant(literal.clone())),
            ExpressionKind::Variable(var) => {
                let resolved = self.declares.get_variable_resolve(var.id).ok_or_else(|| {
                    Fault::error("variable has no resolved binding", Some(expr.span))
                })?;
                let local = *self.node_to_local.get(&resolved).ok_or_else(|| {
                    Fault::error(
                        "variable isn't bound to a local in this function's lowered scope",
                        Some(expr.span),
                    )
                })?;
                Ok(Operand::Copy(Place::local(local)))
            }
            ExpressionKind::Binary(_) => {
                let span = expr.span;
                let ty = self
                    .declares
                    .get_expression_type(expr_id)
                    .cloned()
                    .ok_or_else(|| {
                        Fault::error("nested expression has no resolved type", Some(span))
                    })?;
                require_primitive(&ty, span)?;

                let rvalue = self.lower_rvalue(expr_id, statements)?;
                let temp = self.alloc_local(ty, TypeModifier::Const, span);
                statements.push(mir_model::Statement::Assign(Place::local(temp), rvalue));
                Ok(Operand::Copy(Place::local(temp)))
            }
            _ => Err(Fault::error(
                "only literals, variables, and arithmetic binary expressions are supported as operands in this lowering slice",
                Some(expr.span),
            )),
        }
    }
}
