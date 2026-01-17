use std::{iter, sync::LazyLock};

use parser_models::ast::{
    Assignment, Binary, BinaryOperator, BinaryOperatorKind, Expression, Statement,
    StatementHelpers, StatementKind,
};
use soul_tokenizer::TokenKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::AssignType,
    span::Span,
};

use crate::parser::{
    Parser,
    parse_utils::{SEMI_COLON, STAMENT_END_TOKENS},
};

static ASSIGNMENT_TOKENS: LazyLock<Vec<TokenKind>> = LazyLock::new(|| {
    AssignType::SYMBOLS
        .iter()
        .copied()
        .map(TokenKind::Symbol)
        .chain(iter::once(TokenKind::EndLine))
        .chain(iter::once(SEMI_COLON))
        .collect()
});

impl<'a> Parser<'a> {
    pub(crate) fn parse_assign(&mut self, start_span: Span) -> SoulResult<Statement> {
        let lvalue = self.parse_expression(&ASSIGNMENT_TOKENS)?;
        if self.current_is_any(STAMENT_END_TOKENS) {
            return Ok(Statement::from_expression(lvalue));
        }

        let assign_token = self.bump_consume();
        let assign = match &assign_token.kind {
            TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                AssignType::from_symbool(*val).unwrap()
            }
            _ => {
                return Err(SoulError::new(
                    format!(
                        "'{}' should be a assign symbool",
                        assign_token.kind.display(),
                    ),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.span_combine(start_span)),
                ));
            }
        };

        let rvalue = self.parse_expression(STAMENT_END_TOKENS)?;
        let resolved_rvalue = resolve_assign_type(&lvalue, assign, assign_token.span, rvalue);

        self.bump();

        let assignment = Assignment {
            left: lvalue,
            node_id: None,
            right: resolved_rvalue,
        };

        Ok(Statement::new(
            StatementKind::Assignment(assignment),
            self.span_combine(start_span),
        ))
    }
}

fn resolve_assign_type(
    lvalue: &Expression,
    assign: AssignType,
    span: Span,
    rvalue: Expression,
) -> Expression {
    let full_span = span.combine(rvalue.get_span());

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

    Expression::new(
        parser_models::ast::ExpressionKind::Binary(Binary::new(lvalue.clone(), operator, rvalue)),
        full_span,
    )
}
