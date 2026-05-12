use ast::{Block, ElseKind, Expression, ExpressionKind, If, IfArm, IfArmHelper, Literal, Match, MatchArm, MatchPattern, While};
use soul_tokenizer::{Number, TokenKind};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{KeyWord, TypeModifier},
    span::{Spanned},
    symbool_kind::SymbolKind,
};

use crate::parser::{
    Parser,
    parse_utils::{CURLY_CLOSE, CURLY_OPEN, STAMENT_END_TOKENS},
};

const IF_STR: &str = KeyWord::If.as_str();
const ELSE_STR: &str = KeyWord::Else.as_str();
const WHILE_STR: &str = KeyWord::While.as_str();
const MATCH_STR: &str = KeyWord::Match.as_str();

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_if(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(IF_STR)?;

        let if_condition = self.parse_expression(&[CURLY_OPEN])?;
        let if_block = self.parse_block(TypeModifier::Mut)?;

        let mut r#if = If {
            condition: Box::new(if_condition),
            block: if_block,
            else_branchs: None,
            id: None,
        };

        self.parse_if_arms(&mut r#if.else_branchs)?;
        Ok(Expression::new(
            ExpressionKind::If(r#if),
            self.span_combine(start_span),
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
            ExpressionKind::While(While {
                condition,
                block,
                id: None,
            }),
            self.span_combine(start_span),
        ))
    }

    fn parse_if_arms(&mut self, head: &mut Option<IfArm>) -> SoulResult<()> {
        let mut tail: &mut Option<IfArm> = head;
        let mut has_else = false;

        loop {
            let position = self.current_position();
            self.skip_till(STAMENT_END_TOKENS);

            let start_span = self.token().span;
            if !self.current_is_ident(ELSE_STR) {
                self.go_to(position);
                break Ok(());
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
                        else_branchs: None,
                        id: None,
                    },
                    self.span_combine(start_span),
                )))
            } else {
                has_else = true;
                let start_span = self.token().span;
                let block = self.parse_block(TypeModifier::Mut)?;
                ElseKind::Else(Spanned::new(block, self.span_combine(start_span)))
            };

            *tail = Some(IfArm::new_arm(else_kind, self.span_combine(else_span)));
            tail = match tail.as_mut().expect("just made Some(_)").try_next_mut() {
                Some(val) => val,
                None => return Ok(()),
            };
        }
    }

    pub(crate) fn parse_match(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(MATCH_STR)?;

        let scrutinee = self.parse_expression(&[CURLY_OPEN])?;

        let arms = self.parse_match_arms()?;

        Ok(Expression::new(
            ExpressionKind::Match(Match {
                id: None,
                scrutinee: Box::new(scrutinee),
                arms,
            }),
            start_span,
        ))
    }

    fn parse_match_arms(&mut self) -> SoulResult<Vec<MatchArm>> {
        self.expect(&CURLY_OPEN)?;
        let mut arms = Vec::new();

        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                self.bump();
                break;
            }

            let pattern = self.parse_match_pattern()?;
            self.skip_end_lines();

            if !self.current_is(&TokenKind::Symbol(SymbolKind::LambdaArray)) {
                return Err(SoulError::new(
                    "expected '=>' in match arm",
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.token().span),
                ));
            }
            self.bump();

            let body = if self.current_is(&CURLY_OPEN) {
                self.parse_block(TypeModifier::Mut)?
            } else {
                let span = self.token().span;
                let statement = self.parse_statement()?;
                Block {
                    span,
                    node_id: None,
                    scope_id: None,
                    modifier: TypeModifier::Mut,
                    statements: vec![statement],
                }
            };
            arms.push(MatchArm { pattern, body });
        }

        Ok(arms)
    }

    fn parse_match_pattern(&mut self) -> SoulResult<MatchPattern> {
        if self.current_is_ident("_") {
            self.bump();
            return Ok(MatchPattern::Wildcard);
        }

        let token_kind = self.token().kind.clone();
        match &token_kind {
            TokenKind::Number(num) => {
                let value = match num {
                    Number::Uint(u) => *u as i128,
                    Number::Int(i) => *i as i128,
                    Number::Float(_) => {
                        return Err(SoulError::new(
                            "expected integer literal for match pattern",
                            SoulErrorKind::InvalidIdent,
                            Some(self.token().span),
                        ));
                    }
                };
                self.bump();
                Ok(MatchPattern::Literal(Literal::Int(value)))
            }
            TokenKind::Ident(ident) => {
                if let Ok(i) = ident.parse::<i128>() {
                    self.bump();
                    return Ok(MatchPattern::Literal(Literal::Int(i)));
                }
                Err(SoulError::new(
                    "expected integer literal or '_' for match pattern",
                    SoulErrorKind::InvalidIdent,
                    Some(self.token().span),
                ))
            }
            _ => Err(SoulError::new(
                "expected integer literal or '_' for match pattern",
                SoulErrorKind::InvalidIdent,
                Some(self.token().span),
            )),
        }
    }
}
