use parser_models::ast::{ElseKind, Expression, ExpressionKind, If, IfArm, IfArmHelper, While};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{KeyWord, TypeModifier},
    span::Spanned,
};

use crate::parser::{
    Parser,
    parse_utils::{CURLY_OPEN, STAMENT_END_TOKENS},
};

const IF_STR: &str = KeyWord::If.as_str();
const ELSE_STR: &str = KeyWord::Else.as_str();
const WHILE_STR: &str = KeyWord::While.as_str();

impl<'a> Parser<'a> {
    pub(crate) fn parse_if(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(IF_STR)?;

        let if_condition = self.parse_expression(&[CURLY_OPEN])?;
        let if_block = self.parse_block(TypeModifier::Mut)?;

        let mut r#if = If {
            condition: Box::new(if_condition),
            block: if_block,
            else_branchs: None,
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
            ExpressionKind::While(While { condition, block }),
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
}
