use ast::{
    Expression, ExpressionKind, ReturnKind, ReturnLike, Statement,
};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_error_internal,
    soul_names::KeyWord,
    span::Span,
    try_result::{ResultTryErr, TryErr, TryOk, TryResult},
};

use crate::parser::{Parser, parse_utils::{SEMI_COLON, STAMENT_END_TOKENS}};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn try_parse_from_keyword(
        &mut self,
        start_span: Span,
        keyword: KeyWord,
    ) -> TryResult<Statement, SoulError> {
        let kind = match keyword {
            KeyWord::If 
            | KeyWord::True
            | KeyWord::Null
            | KeyWord::Else 
            | KeyWord::False
            | KeyWord::While => {
                let value = self.parse_expression(STAMENT_END_TOKENS).try_err()?;
                Statement::from_expression(value, self.current_is(&SEMI_COLON))
            }

            KeyWord::Break 
            | KeyWord::Return 
            | KeyWord::Continue => {
                let kind = ReturnKind::from_keyword(keyword).expect("should be return keyword");

                self.bump();
                let value = if self.current_is_any(STAMENT_END_TOKENS) {
                    None
                } else {
                    Some(Box::new(
                        self.parse_expression(STAMENT_END_TOKENS).try_err()?,
                    ))
                };

                let return_like = ReturnLike {
                    value,
                    kind,
                    id: None,
                };
                Statement::from_expression(
                    Expression::new(
                        ExpressionKind::ReturnLike(return_like),
                        self.span_combine(start_span),
                    ), 
                    self.current_is(&SEMI_COLON),
                )
            }
            KeyWord::Import => self.parse_import().try_err()?,

            KeyWord::New
            | KeyWord::For
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

            KeyWord::As => {
                return TryErr(soul_error_internal!(
                    format!(
                        "keyword '{}' should be parsed in expression not statement",
                        keyword.as_str()
                    ),
                    Some(self.token().span)
                ));
            }
        };

        TryOk(kind)
    }
}
