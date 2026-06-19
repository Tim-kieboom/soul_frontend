use ast_model::{
    expression::{Expression, ExpressionKind},
    statements::{Statement, StatementKind, TypeDef},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    TypeModifier,
    collections::try_result::{ResultTryErr, ToResult, TryErr, TryOk, TryResult},
    error::SoulResult,
    fault::Fault,
    soul_error_internal,
    span::Span,
};

use crate::{
    parser::Parser,
    utils::{ASSIGN, SEMI_COLON, STAMENT_END_TOKENS},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn try_parse_from_keyword(
        &mut self,
        start_span: Span,
        keyword: KeyWord,
    ) -> TryResult<Statement, Fault> {
        TryOk(match keyword {
            KeyWord::Mut | KeyWord::Const | KeyWord::Literal => {
                let modifier = match keyword {
                    KeyWord::Mut => TypeModifier::Mut,
                    KeyWord::Const => TypeModifier::Const,
                    KeyWord::Literal => TypeModifier::Literal,
                    _ => unreachable!(),
                };

                return self.try_parse_from_modifier(start_span, modifier);
            }

            KeyWord::If
            | KeyWord::New
            | KeyWord::For
            | KeyWord::True
            | KeyWord::Null
            | KeyWord::Else
            | KeyWord::False
            | KeyWord::Sizeof
            | KeyWord::Match => {
                let value = self.parse_expression_id(STAMENT_END_TOKENS).try_err()?;
                Statement::from_expression(self.store, value, self.current_is(&SEMI_COLON))
            }

            KeyWord::Break | KeyWord::Return | KeyWord::Continue => {
                self.bump();
                let kind = match keyword {
                    KeyWord::Break => ExpressionKind::Break,
                    KeyWord::Continue => ExpressionKind::Continue,
                    KeyWord::Return if self.current_is_any(STAMENT_END_TOKENS) => {
                        ExpressionKind::Return(None)
                    }
                    KeyWord::Return => ExpressionKind::Return(Some(
                        self.parse_expression_id(STAMENT_END_TOKENS).try_err()?,
                    )),
                    _ => unreachable!(),
                };

                let expression =
                    Expression::new_id(self.store, kind, self.span_combine(start_span));

                Statement::from_expression(self.store, expression, self.current_is(&SEMI_COLON))
            }

            KeyWord::Import => return self.parse_import().try_err(),
            KeyWord::Extern => return self.parse_extern_function().try_err(),
            KeyWord::Pub => {
                let pub_span = self.current().span;

                self.bump();
                let mut statement = self.parse_statement().try_err()?;
                statement
                    .try_set_is_public(true, pub_span.combine(start_span))
                    .try_err()?;

                statement
            }

            KeyWord::As
            | KeyWord::Impl
            | KeyWord::Copy
            | KeyWord::Crate
            | KeyWord::Typeof
            | KeyWord::Distinct
            | KeyWord::InForLoop
            | KeyWord::Undefined
            | KeyWord::GenericWhere => {
                return TryErr(soul_error_internal!(
                    format!(
                        "keyword '{}' should be parsed in expression not statement",
                        keyword.as_str()
                    ),
                    Some(self.current().span)
                ));
            }

            KeyWord::Use => return self.parse_use_block().try_err(),
            KeyWord::Enum => return self.parse_enum().try_err(),
            KeyWord::Trait => todo!(),
            KeyWord::Struct => return self.parse_struct().try_err(),
            KeyWord::Type => return self.parse_type_statement().try_err(),
        })
    }

    fn parse_type_statement(&mut self) -> SoulResult<Statement> {
        let start_span = self.current().span;
        self.expect(&TokenKind::Keyword(KeyWord::Type))?;

        let new_type = self.try_parse_type().merge_to_result()?;
        self.expect(&ASSIGN)?;
        let is_distinct = self.current_is(&TokenKind::Keyword(KeyWord::Distinct));
        if is_distinct {
            self.bump();
        }

        let old_type = self.try_parse_type().merge_to_result()?;
        let typedef = TypeDef {
            id: None,
            new_type,
            old_type,
            is_distinct,
        };

        Ok(Statement::new(
            StatementKind::TypeDef(typedef),
            self.span_combine(start_span),
        ))
    }
}
