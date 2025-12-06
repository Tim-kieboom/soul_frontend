use crate::steps::{
    parse::{
        parse_statement::{CURLY_OPEN, STAMENT_END_TOKENS},
        parser::Parser,
    },
    tokenize::token_stream::TokenKind,
};
use models::{
    abstract_syntax_tree::{
        conditionals::{ElseKind, For, ForPattern, If, While},
        expression::{Expression, ExpressionKind},
        spanned::Spanned,
    },
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{KeyWord, TypeModifier},
};
use std::sync::LazyLock;

const IF_STR: &str = KeyWord::If.as_str();
const FOR_STR: &str = KeyWord::For.as_str();
const ELSE_STR: &str = KeyWord::Else.as_str();
const WHILE_STR: &str = KeyWord::While.as_str();
const MATCH_STR: &str = KeyWord::Match.as_str();

impl<'a> Parser<'a> {
    pub(crate) fn parse_if(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(IF_STR)?;

        let if_condition = self.parse_expression(&[CURLY_OPEN])?;
        let if_block = self.parse_block(TypeModifier::Mut)?;

        Ok(Expression::new(
            ExpressionKind::If(If {
                condition: Box::new(if_condition),
                block: if_block,
                else_branchs: self.parse_elses()?,
            }),
            self.new_span(start_span),
        ))
    }

    pub(crate) fn parse_for(&mut self) -> SoulResult<Expression> {
        static END_TOKENS: LazyLock<[TokenKind; 2]> = LazyLock::new(|| {
            [
                TokenKind::Ident(KeyWord::InForLoop.as_str().to_string()),
                CURLY_OPEN,
            ]
        });

        let start_span = self.token().span;
        self.expect_ident(FOR_STR)?;

        let expression = self.parse_expression(END_TOKENS.as_ref())?;
        let (element, collection) = if self.current_is_ident(KeyWord::InForLoop.as_str()) {
            self.bump();
            let collection = self.parse_expression(&[CURLY_OPEN])?;
            (
                Some(ForPattern::from_expression(expression)?),
                Box::new(collection),
            )
        } else {
            (None, Box::new(expression))
        };

        let block = self.parse_block(TypeModifier::Mut)?;

        Ok(Expression::new(
            ExpressionKind::For(For {
                block,
                element,
                collection,
            }),
            self.new_span(start_span),
        ))
    }

    pub(crate) fn parse_while(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(WHILE_STR)?;

        self.skip_end_lines();
        let condition = if self.current_is(&CURLY_OPEN) {
            None
        } else {
            Some(Box::new(self.parse_expression(&[CURLY_OPEN])?))
        };

        let block = self.parse_block(TypeModifier::Mut)?;
        Ok(Expression::new(
            ExpressionKind::While(While { condition, block }),
            self.new_span(start_span),
        ))
    }

    pub(crate) fn parse_match(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(MATCH_STR)?;
        todo!()
    }

    fn parse_elses(&mut self) -> SoulResult<Vec<Spanned<ElseKind>>> {
        let mut elses = vec![];
        let mut has_else = false;

        loop {
            let position = self.current_position();
            self.skip(STAMENT_END_TOKENS);

            let start_span = self.token().span;
            if !self.current_is_ident(ELSE_STR) {
                self.go_to(position);
                break Ok(elses);
            }

            if has_else {
                return Err(SoulError::new(
                    format!(
                        "can not have '{ELSE_STR}' or '{ELSE_STR} {IF_STR}' after '{ELSE_STR}'"
                    ),
                    SoulErrorKind::InvalidContext,
                    Some(start_span),
                ));
            }

            let else_span = self.token().span;

            self.bump();
            let else_kind = if self.current_is_ident(IF_STR) {
                let start_span = self.token().span;

                self.bump();
                let condition = self.parse_expression(&[CURLY_OPEN])?;
                let block = self.parse_block(TypeModifier::Mut)?;
                ElseKind::ElseIf(Box::new(Spanned::new(
                    If {
                        condition: Box::new(condition),
                        block,
                        else_branchs: vec![],
                    },
                    self.new_span(start_span),
                )))
            } else {
                has_else = true;
                let start_span = self.token().span;
                let block = self.parse_block(TypeModifier::Mut)?;
                ElseKind::Else(Spanned::new(block, self.new_span(start_span)))
            };

            elses.push(Spanned::new(else_kind, self.new_span(else_span)));
        }
    }
}
