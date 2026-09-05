//! AST-to-MIR lowering. First (smallest-testable) slice only: a single non-generic
//! function whose parameters/return/locals are all primitive scalars, whose body is
//! a flat sequence of `name := <atom> [op <atom>]` variable declarations followed by
//! exactly one `return <expr>`. No control flow, no calls, no aggregates, no nested
//! compound expressions yet — each is a separate follow-on slice (see
//! `docs/mir-design.md` at the repo root).
//!
//! Anything outside that subset is rejected with a `LowerError` rather than panicking
//! or silently mis-lowering: this pass runs on already-name-resolved, well-typed
//! input, so every `LowerError` here means "not supported by this slice yet," not
//! "the input program is invalid."

use std::collections::HashMap;

#[cfg(test)]
mod tests;

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
    FunctionId, TypeModifier, collections::vec_map::VecMap, ids::IdGenerator, span::Span,
};

#[derive(Debug, PartialEq)]
pub enum LowerError {
    /// `function_id` names an extern/signature-only declaration, not a function
    /// with a body.
    NotANormalFunction,
    /// The body has no `return <expr>` as its final reachable statement.
    MissingReturn,
    /// A parameter/return/local type isn't a primitive scalar. Every primitive
    /// scalar is `AutoCopy` per `soul-lang.md` §11, which is what lets this slice
    /// always lower a variable read as `Operand::Copy` without checking a real
    /// `AutoCopy` bound (that check doesn't exist yet in the resolver) — so
    /// non-primitive types are out of scope until it does.
    UnsupportedType(SoulType),
    UnsupportedStatement(&'static str),
    UnsupportedExpression(&'static str),
}

pub fn lower_function(
    store: &AstStore,
    declares: &DeclareStore,
    function_id: FunctionId,
) -> Result<MirFunction, LowerError> {
    let FunctionKind::Normal(function) = &store.functions[function_id] else {
        return Err(LowerError::NotANormalFunction);
    };
    let signature = &function.signature.value;

    let mut lowerer = Lowerer {
        store,
        declares,
        locals: VecMap::new(),
        local_gen: IdGenerator::new(),
        node_to_local: HashMap::new(),
    };

    for param in &signature.parameters {
        require_primitive(&param.ty)?;
        let modifier = if param.is_mut {
            TypeModifier::Mut
        } else {
            TypeModifier::Const
        };
        let local = lowerer.alloc_local(param.ty.clone(), modifier, param.name.span());
        lowerer.node_to_local.insert(param.id, local);
    }
    let arg_count = signature.parameters.len();

    require_primitive(&signature.return_type)?;
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
                    return Err(LowerError::UnsupportedStatement(
                        "only simple (non-destructuring) variable bindings are supported in this lowering slice",
                    ));
                };
                let Some(init) = var.initialize_value else {
                    return Err(LowerError::UnsupportedStatement(
                        "a variable declaration with no initializer isn't supported in this lowering slice",
                    ));
                };
                let ty = declares
                    .get_variable_type(binding.id)
                    .and_then(|(_, ty, _)| ty.clone())
                    .ok_or(LowerError::UnsupportedStatement(
                        "variable has no resolved type",
                    ))?;

                require_primitive(&ty)?;

                let rvalue = lowerer.lower_rvalue(init)?;
                let local = lowerer.alloc_local(ty, *modifier, binding.ident.span());
                lowerer.node_to_local.insert(binding.id, local);
                statements.push(mir_model::Statement::Assign(Place::local(local), rvalue));
            }
            StatementKind::Expression { expression, .. } => {
                let expr = &store.expressions[*expression];
                let ExpressionKind::Return(Some(value_id)) = &expr.node else {
                    return Err(LowerError::UnsupportedStatement(
                        "only a `return <expr>` statement is supported as a function's terminal statement in this lowering slice",
                    ));
                };
                let rvalue = lowerer.lower_rvalue(*value_id)?;
                statements.push(mir_model::Statement::Assign(
                    Place::local(return_local),
                    rvalue,
                ));
                terminator = Some(Terminator::Return);
                break;
            }
            _ => {
                return Err(LowerError::UnsupportedStatement(
                    "this statement kind isn't supported in this lowering slice",
                ));
            }
        }
    }

    let terminator = terminator.ok_or(LowerError::MissingReturn)?;

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

fn require_primitive(ty: &SoulType) -> Result<(), LowerError> {
    if matches!(ty, SoulType::Primitive(_)) {
        Ok(())
    } else {
        Err(LowerError::UnsupportedType(ty.clone()))
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

    /// An `Rvalue`: either a flat binary op between two atoms, or a bare atom.
    /// Nested compound expressions (e.g. `a + b * c`) aren't supported yet — that
    /// needs three-address-code-style temporaries (a temp local per sub-expression)
    /// which is deferred to the next slice.
    fn lower_rvalue(&mut self, expr_id: ExpressionId) -> Result<Rvalue, LowerError> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ExpressionKind::Binary(binary) => {
                if !is_supported_arithmetic_op(binary.operator.value) {
                    return Err(LowerError::UnsupportedExpression(
                        "only arithmetic binary operators (+ - * / %) are supported in this lowering slice",
                    ));
                }
                let left = self.lower_operand(binary.left)?;
                let right = self.lower_operand(binary.right)?;
                Ok(Rvalue::BinaryOp(binary.operator.value, left, right))
            }
            _ => Ok(Rvalue::Use(self.lower_operand(expr_id)?)),
        }
    }

    /// An atom: a literal constant, or a read of an already-lowered local. All
    /// primitive scalars are `AutoCopy` (soul-lang.md §11), so a variable read is
    /// always `Operand::Copy` in this slice — never `Move`.
    fn lower_operand(&mut self, expr_id: ExpressionId) -> Result<Operand, LowerError> {
        let expr = &self.store.expressions[expr_id];
        match &expr.node {
            ExpressionKind::Literal((_, literal)) => Ok(Operand::Constant(literal.clone())),
            ExpressionKind::Variable(var) => {
                let resolved = self.declares.get_variable_resolve(var.id).ok_or(
                    LowerError::UnsupportedExpression("variable has no resolved binding"),
                )?;
                let local =
                    *self
                        .node_to_local
                        .get(&resolved)
                        .ok_or(LowerError::UnsupportedExpression(
                            "variable isn't bound to a local in this function's lowered scope",
                        ))?;
                Ok(Operand::Copy(Place::local(local)))
            }
            _ => Err(LowerError::UnsupportedExpression(
                "only literals and variables are supported as operands in this lowering slice",
            )),
        }
    }
}
