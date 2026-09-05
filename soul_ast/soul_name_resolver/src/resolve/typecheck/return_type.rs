use ast_model::{
    block::BlockId,
    expression::{ExpressionId, ExpressionKind, If, IfBranch},
    soul_type::{Generic, SoulType},
    statements::StatementKind,
};
use soul_utils::{fault::Fault, span::Span};

use super::expression::default_concrete_type;
use super::function_call::is_generic_parameter;
use crate::NameResolver;

impl<'a> NameResolver<'a> {
    pub(crate) fn check_tail_return_type(
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

    pub(crate) fn check_return_statement(&mut self, value: Option<ExpressionId>, span: Span) {
        if let Some(function_id) = self.current.function {
            let Some((signature, _)) = self.declares.get_function(function_id) else {
                return;
            };
            let return_type = signature.return_type.clone();
            let generics = signature.generics.clone();
            if is_generic_parameter(&return_type, &generics) {
                return;
            }
            self.check_return_value(value, &return_type, &generics, span);
            return;
        }

        if let Some(return_type) = self.current.lambda_return_type.clone() {
            self.check_return_value(value, &return_type, &[], span);
        }
    }

    fn check_return_value(
        &mut self,
        value: Option<ExpressionId>,
        return_type: &SoulType,
        generics: &[Generic],
        span: Span,
    ) {
        match value {
            Some(expression_id) => self.check_tail_expression(expression_id, return_type, generics),
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
        let block = self.get_block(block_id)?;
        let statement = self.get_statement(*block.statements.last()?)?;
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
            ExpressionKind::Block(block_id) => {
                self.check_tail_return_type(*block_id, return_type, generics)
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

    pub(crate) fn first_lambda_return_type(&self, body: BlockId) -> Option<SoulType> {
        let value = self.first_return_value(body)?;
        self.expression_type(value).map(default_concrete_type)
    }

    fn first_return_value(&self, block_id: BlockId) -> Option<ExpressionId> {
        let block = self.get_block(block_id)?;
        let last_index = block.statements.len().checked_sub(1);
        for (index, statement_id) in block.statements.iter().enumerate() {
            let statement = self.get_statement(*statement_id)?;
            let StatementKind::Expression {
                expression,
                ends_semicolon,
            } = &statement.node
            else {
                continue;
            };

            if let Some(found) = self.first_return_in_expression(*expression) {
                return Some(found);
            }

            if Some(index) == last_index && !ends_semicolon {
                return Some(*expression);
            }
        }
        None
    }

    fn first_return_in_expression(&self, expression_id: ExpressionId) -> Option<ExpressionId> {
        let expression = self.get_expression(expression_id)?;
        match &expression.node {
            ExpressionKind::Return(value) => *value,
            ExpressionKind::Block(block_id) => self.first_return_value(*block_id),
            ExpressionKind::If(if_) => self.first_return_in_if(if_),
            ExpressionKind::Match(match_) => match_
                .arms
                .iter()
                .find_map(|arm| self.first_return_value(arm.body)),
            _ => None,
        }
    }

    fn first_return_in_if(&self, if_: &If) -> Option<ExpressionId> {
        if let Some(found) = self.first_return_value(if_.block) {
            return Some(found);
        }

        let mut current = if_.branch.as_ref();
        loop {
            match current {
                Some(IfBranch::Else(block_id)) => return self.first_return_value(*block_id),
                Some(IfBranch::If(elif)) => {
                    if let Some(found) = self.first_return_value(elif.block) {
                        return Some(found);
                    }
                    current = elif.branch.as_ref();
                }
                None => return None,
            }
        }
    }
}
