use ast_model::{
    FunctionKind,
    expression::{Expression, ExpressionKind},
    soul_type::{SoulType, Stub},
    statements::{Statement, StatementKind, Trait, TypeDef},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    TypeModifier,
    collections::try_result::{
        ResultMapNotValue, ResultTryErr, ToResult, TryErr, TryOk, TryResult,
    },
    error::SoulResult,
    fault::Fault,
    soul_error_internal,
    span::{Span, Spanned},
};

use crate::{
    parser::Parser,
    utils::{ASSIGN, COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, SEMI_COLON, STAMENT_END_TOKENS},
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
                Statement::from_expression(&self.forest.store, value, self.current_is(&SEMI_COLON))
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

                let span = self.span_combine(start_span);
                let expression =
                    Expression::new_id(&mut self.forest.store, kind, span);

                Statement::from_expression(&self.forest.store, expression, self.current_is(&SEMI_COLON))
            }

            KeyWord::Import => return self.parse_import().try_err(),
            KeyWord::Extern => return self.parse_extern_function().try_err(),
            KeyWord::Pub => {
                let pub_span = self.token().span;

                self.bump();
                let mut statement = self.parse_statement().try_err()?;
                let is_public = true;
                let span = pub_span.combine(start_span);
                statement
                    .try_set_is_public(&mut self.forest.store, is_public, span)
                    .try_err()?;

                statement
            }

            KeyWord::As
            | KeyWord::Impl
            | KeyWord::Copy
            | KeyWord::Pass
            | KeyWord::Crate
            | KeyWord::Typeof
            | KeyWord::Distinct
            | KeyWord::InForLoop
            | KeyWord::Undefined
            | KeyWord::GenericWhere
            | KeyWord::Union
            | KeyWord::Async
            | KeyWord::Task
            | KeyWord::Spawn
            | KeyWord::Limit
            | KeyWord::Intrinsic => {
                return TryErr(soul_error_internal!(
                    format!(
                        "keyword '{}' should be parsed in expression not statement",
                        keyword.as_str()
                    ),
                    Some(self.token().span)
                ));
            }

            KeyWord::Use => return self.parse_use_block().try_err(),
            KeyWord::Enum => return self.parse_enum().try_err(),
            KeyWord::Trait => return self.parse_trait().try_err(),
            KeyWord::Struct => return self.parse_struct().try_err(),
            KeyWord::Type => return self.parse_typedef().map(Statement::from_typedef).try_err(),
        })
    }

    fn parse_trait(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;
        self.expect(&TokenKind::Keyword(KeyWord::Trait))?;
        let name = self.try_bump_consume_ident()?;
        let generics = self.parse_generic_declare()?.unwrap_or(vec![]);

        let mut trait_impls = vec![];
        if self.current_is(&COLON) {
            self.bump();
            loop {
                trait_impls.push(self.try_bump_consume_ident()?);
                if !self.current_is(&COMMA) {
                    break;
                }
            }
        }

        self.expect(&CURLY_OPEN)?;

        let mut methods = vec![];
        let mut typedefs = vec![];
        let this_type = SoulType::Stub(Stub::new(name.as_str()));
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            if self.current_is(&TokenKind::Keyword(KeyWord::Type)) {
                self.bump();
                let ty = self.try_parse_type().merge_to_result()?;
                typedefs.push(ty);
                continue;
            }

            let start_span = self.token().span;
            let modifier = self.try_bump_type_modiffier().unwrap_or(TypeModifier::Mut);
            let name = self.try_bump_consume_ident()?;
            let signature = self
                .try_parse_function_signature(start_span, modifier, &this_type, name, None)
                .map_try_not_value(|(_, err)| err)
                .merge_to_result()?
                .value;

            let spanned =
                FunctionKind::Signature(Spanned::new(signature, self.span_combine(start_span)));
            let id = self.forest.store.insert_function(spanned);
            methods.push(id);
        }
        self.expect(&CURLY_CLOSE)?;
        Ok(Statement::new(
            StatementKind::Trait(Trait {
                id: self.alloc_node(),
                name,
                generics,
                methods,
                typedefs,
                trait_impls,
            }),
            self.span_combine(start_span),
        ))
    }

    fn parse_typedef(&mut self) -> SoulResult<Spanned<TypeDef>> {
        let start_span = self.token().span;
        self.expect(&TokenKind::Keyword(KeyWord::Type))?;

        let new_type = self.try_parse_type().merge_to_result()?;
        self.expect(&ASSIGN)?;
        let is_distinct = self.current_is(&TokenKind::Keyword(KeyWord::Distinct));
        if is_distinct {
            self.bump();
        }

        let old_type = self.try_parse_type().merge_to_result()?;
        let typedef = TypeDef {
            new_type,
            old_type,
            is_distinct,
        };

        Ok(Spanned::new(typedef, self.span_combine(start_span)))
    }
}
