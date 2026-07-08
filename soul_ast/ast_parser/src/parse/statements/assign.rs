use std::{iter, sync::LazyLock};

use crate::{
    parse::statements::variable::AssignType,
    parser::Parser,
    utils::{CURLY_CLOSE, SEMI_COLON, STAMENT_END_TOKENS},
};
use ast_model::{
    AstStore,
    expression::{Expression, ExpressionId},
    operators::{BinaryOperator, BinaryOperatorKind},
    statements::{Assignment, Statement, StatementKind},
};
use soul_tokenizer::model::TokenKind;
use soul_utils::{error::SoulResult, fault::Fault, span::Span};

static ASSIGNMENT_TOKENS: LazyLock<Vec<TokenKind>> = LazyLock::new(|| {
    AssignType::SYMBOL_VALUES
        .iter()
        .copied()
        .map(TokenKind::Symbol)
        .chain(iter::once(TokenKind::EndLine))
        .chain(iter::once(SEMI_COLON))
        .chain(iter::once(CURLY_CLOSE))
        .chain(iter::once(TokenKind::EndFile))
        .collect()
});

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_assign_or_expression(&mut self, start_span: Span) -> SoulResult<Statement> {
        let lvalue = self.parse_expression_id(&ASSIGNMENT_TOKENS)?;
        if self.current_is_any(STAMENT_END_TOKENS) {
            return Ok(Statement::from_expression(
                &self.forest.store,
                lvalue,
                self.current_is(&SEMI_COLON),
            ));
        }

        let assign_token = self.bump_consume();
        let assign = match &assign_token.kind {
            TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                AssignType::from_symbool(*val).unwrap()
            }
            _ => {
                return Err(Fault::error(
                    format!(
                        "'{}' should be a assign symbool",
                        assign_token.kind.display(),
                    ),
                    Some(self.span_combine(start_span)),
                ));
            }
        };

        let rvalue = self.parse_expression_id(STAMENT_END_TOKENS)?;
        let resolved_rvalue =
            resolve_assign_type(&mut self.forest.store, lvalue, assign, rvalue, assign_token.span);

        self.bump();

        let assignment = Assignment {
            left: lvalue,
            right: resolved_rvalue,
        };

        Ok(Statement::new(
            StatementKind::Assignment(assignment),
            self.span_combine(start_span),
        ))
    }
}

fn resolve_assign_type(
    store: &mut AstStore,
    lvalue: ExpressionId,
    assign: AssignType,
    rvalue: ExpressionId,
    span: Span,
) -> ExpressionId {
    let rspan = store.expressions[rvalue].span;
    let full_span = span.combine(rspan);

    let operator = match assign {
        AssignType::AddAssign => BinaryOperator::new(BinaryOperatorKind::Add, span),
        AssignType::SubAssign => BinaryOperator::new(BinaryOperatorKind::Sub, span),
        AssignType::MulAssign => BinaryOperator::new(BinaryOperatorKind::Mul, span),
        AssignType::DivAssign => BinaryOperator::new(BinaryOperatorKind::Div, span),
        AssignType::ModAssign => BinaryOperator::new(BinaryOperatorKind::Mod, span),
        AssignType::BitOrAssign => BinaryOperator::new(BinaryOperatorKind::BitOr, span),
        AssignType::BitXorAssign => BinaryOperator::new(BinaryOperatorKind::BitXor, span),
        AssignType::BitAndAssign => BinaryOperator::new(BinaryOperatorKind::BitAnd, span),
        AssignType::Assign | AssignType::Declaration => return rvalue,
    };

    let id = store.alloc_node();
    store.insert_expression(Expression::new_binary(
        id,
        lvalue.clone(),
        operator,
        rvalue,
        full_span,
    ))
}
