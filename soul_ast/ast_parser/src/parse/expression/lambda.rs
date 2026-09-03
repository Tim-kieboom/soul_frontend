use ast_model::{
    block::Block,
    expression::{Expression, ExpressionId, ExpressionKind, Lambda},
    statements::{Statement, VarPattern},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{TypeModifier, error::SoulResult, soul_names::Symbol, span::Span};

use crate::{parser::Parser, utils::LAMBDA_ARROW};

impl<'a, 'f> Parser<'a, 'f> {
    /// Try to parse a lambda expression: `params => body`.
    /// Returns `Ok(expr)` if successful, or `Err(())` on failure (caller should backtrack).
    pub(super) fn try_parse_lambda(&mut self, start_span: Span) -> Option<Expression> {
        let saved = self.tokens.current_position();

        let pattern = self.parse_var_pattern(TypeModifier::Const).ok()?;

        if self.current_is(&LAMBDA_ARROW) {
            self.bump();
            let body_expression = self.parse_lambda_body().ok()?;
            let params = match pattern {
                VarPattern::Tuple(tuple) => tuple.elements,
                other => vec![other],
            };

            let statement = self
                .forest
                .store
                .insert_statement(Statement::from_expression(
                    &self.forest.store,
                    body_expression,
                    false,
                ));

            let body = self.forest.store.insert_block(Block {
                is_const: false,
                statements: vec![statement],
                span: self.span_combine(start_span),
            });

            Some(Expression::from_lambda(
                Lambda {
                    id: self.alloc_node(),
                    parameters: params,
                    body,
                },
                self.span_combine(start_span),
            ))
        } else {
            self.goto(saved);
            None
        }
    }

    fn parse_lambda_body(&mut self) -> SoulResult<ExpressionId> {
        let start_span = self.token().span;

        let keyword = match &self.token().kind {
            TokenKind::Keyword(keyword @ (KeyWord::Break | KeyWord::Return | KeyWord::Continue)) => {
                Some(*keyword)
            }
            _ => None,
        };

        if let Some(keyword) = keyword {
            self.bump();
            let kind = match keyword {
                KeyWord::Break => ExpressionKind::Break,
                KeyWord::Continue => ExpressionKind::Continue,
                KeyWord::Return if self.current_is_any(LAMBDA_BODY_END) => {
                    ExpressionKind::Return(None)
                }
                KeyWord::Return => {
                    ExpressionKind::Return(Some(self.parse_expression_id(LAMBDA_BODY_END)?))
                }
                _ => unreachable!(),
            };

            let span = self.span_combine(start_span);
            return Ok(Expression::new_id(&mut self.forest.store, kind, span));
        }

        self.parse_expression_id(LAMBDA_BODY_END)
    }
}

const LAMBDA_BODY_END: &[TokenKind] = &[
    TokenKind::EndFile,
    TokenKind::EndLine,
    TokenKind::Symbol(Symbol::SemiColon),
    TokenKind::Symbol(Symbol::RoundClose),
    TokenKind::Symbol(Symbol::CurlyClose),
    TokenKind::Symbol(Symbol::SquareClose),
    TokenKind::Symbol(Symbol::Comma),
    TokenKind::Symbol(Symbol::Colon),
];
