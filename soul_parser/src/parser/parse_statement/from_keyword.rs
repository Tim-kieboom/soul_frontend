use parser_models::ast::{
    Expression, ExpressionKind, ReturnKind, ReturnLike, Statement, StatementHelpers,
};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_names::KeyWord,
    span::Span,
    try_result::{ResultTryErr, TryErr, TryOk, TryResult},
};

use crate::parser::{Parser, parse_utils::STAMENT_END_TOKENS};

impl<'a> Parser<'a> {
    pub(super) fn try_parse_from_keyword(
        &mut self,
        start_span: Span,
        keyword: KeyWord,
    ) -> TryResult<Statement, SoulError> {
        let kind = match keyword {
            KeyWord::If | KeyWord::Else | KeyWord::While => {
                Statement::from_expression(self.parse_expression(STAMENT_END_TOKENS).try_err()?)
            }
            KeyWord::Break | KeyWord::Return | KeyWord::Continue => {
                let kind = ReturnKind::from_keyword(keyword).expect("should be return keyword");

                self.bump();
                let value = if self.current_is_any(STAMENT_END_TOKENS) {
                    None
                } else {
                    Some(Box::new(
                        self.parse_expression(STAMENT_END_TOKENS).try_err()?,
                    ))
                };

                let return_like = ReturnLike { value, kind, id: None };
                Statement::from_expression(Expression::new(
                    ExpressionKind::ReturnLike(return_like),
                    self.span_combine(start_span),
                ))
            }
            KeyWord::Import => self.parse_import().try_err()?,

            KeyWord::For
            | KeyWord::Use
            | KeyWord::Dyn
            | KeyWord::Fall
            | KeyWord::Enum
            | KeyWord::Copy
            | KeyWord::Impl
            | KeyWord::Trait
            | KeyWord::Class
            | KeyWord::Union
            | KeyWord::Match
            | KeyWord::Await
            | KeyWord::Struct
            | KeyWord::Typeof
            | KeyWord::InForLoop
            | KeyWord::GenericWhere => {
                return TryErr(SoulError::new(
                    format!("keyword '{}' is unstable", keyword.as_str()),
                    SoulErrorKind::InvalidContext,
                    Some(self.token().span),
                ));
            }
        };

        TryOk(kind)
    }
}
