use ast_model::{
    block::BlockId,
    expression::{ExpressionId, ExpressionKind, If, IfBranch},
    soul_type::{Generic, SoulType},
    statements::StatementKind,
};
use soul_utils::{fault::Fault, span::Span};

use super::function_call::is_generic_parameter;
use crate::NameResolver;

impl<'a> NameResolver<'a> {
    /// Checks a function body's implicit tail-expression return against its
    /// declared return type. Explicit `return` statements are not checked
    /// here — this only follows tail position, recursing through `if`/`match`
    /// branches. Anything it can't fully determine (a non-exhaustive `if`, no
    /// tail expression at all, a return type this checker doesn't model like
    /// `impl Trait` or a generic parameter, an expression kind with no known
    /// type) is silently skipped rather than faulted, to avoid false
    /// positives.
    pub(super) fn check_tail_return_type(
        &mut self,
        block_id: BlockId,
        return_type: &SoulType,
        generics: &[Generic],
    ) {
        if is_generic_parameter(return_type, generics) {
            return;
        }
        let Some(tail) = self.tail_position(block_id) else {
            return;
        };
        self.check_tail_expression(tail, return_type, generics);
    }

    /// Checks an explicit `return` statement's value against the current
    /// function's declared return type. A bare `return` (no value) is only
    /// valid when that return type is void. Skipped entirely when there's no
    /// current function context (e.g. inside a lambda, which has no declared
    /// return type of its own) or the return type isn't one this checker
    /// models (generic/`impl Trait`).
    pub(super) fn check_return_statement(&mut self, value: Option<ExpressionId>, span: Span) {
        let Some(function_id) = self.current.function else {
            return;
        };
        let Some((signature, _)) = self.declares.get_function(function_id) else {
            return;
        };
        let return_type = signature.return_type.clone();
        let generics = signature.generics.clone();

        if is_generic_parameter(&return_type, &generics) {
            return;
        }

        match value {
            Some(expression_id) => {
                self.check_tail_expression(expression_id, &return_type, &generics)
            }
            None => {
                if !matches!(return_type, SoulType::None) {
                    self.log_fault(Fault::error(
                        format!("return type mismatch: expected `{return_type:?}`, got nothing"),
                        Some(span),
                    ));
                }
            }
        }
    }

    fn tail_position(&self, block_id: BlockId) -> Option<ExpressionId> {
        let block = self.store.blocks.get(block_id)?;
        let statement = self.store.statements.get(*block.statements.last()?)?;
        let StatementKind::Expression {
            expression,
            ends_semicolon,
        } = &statement.node
        else {
            return None;
        };
        if *ends_semicolon {
            None
        } else {
            Some(*expression)
        }
    }

    fn check_tail_expression(
        &mut self,
        expression_id: ExpressionId,
        return_type: &SoulType,
        generics: &[Generic],
    ) {
        let Some(expression) = self.store.expressions.get(expression_id) else {
            return;
        };

        match &expression.node {
            ExpressionKind::If(if_) => self.check_tail_if(if_, return_type, generics),
            ExpressionKind::Match(match_) => {
                for arm in &match_.arms {
                    self.check_tail_return_type(arm.body, return_type, generics);
                }
            }
            _ => {
                let Some(tail_ty) = self.expression_type(expression_id) else {
                    return;
                };

                if self.combine_operand_types(&tail_ty, return_type).is_some() {
                    return;
                }

                self.log_fault(Fault::error(
                    format!("return type mismatch: expected `{return_type:?}`, got `{tail_ty:?}`"),
                    Some(expression.span),
                ));
            }
        }
    }

    fn check_tail_if(&mut self, if_: &If, return_type: &SoulType, generics: &[Generic]) {
        let mut blocks = vec![if_.block];
        let mut current = if_.branch.as_ref();
        loop {
            match current {
                Some(IfBranch::Else(block_id)) => {
                    blocks.push(*block_id);
                    break;
                }
                Some(IfBranch::If(elif)) => {
                    blocks.push(elif.block);
                    current = elif.branch.as_ref();
                }
                None => return,
            }
        }

        for block_id in blocks {
            self.check_tail_return_type(block_id, return_type, generics);
        }
    }
}
