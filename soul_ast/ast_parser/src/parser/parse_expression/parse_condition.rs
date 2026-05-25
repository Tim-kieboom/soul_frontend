use ast::{
    Block, ElseKind, Expression, ExpressionKind, If, IfArm, IfArmHelper, Match, MatchArm,
    MatchPattern, While,
};
use soul_tokenizer::TokenKind;
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{KeyWord, TypeModifier},
    span::Spanned,
    symbool_kind::SymbolKind,
};

use crate::parser::{
    Parser,
    parse_utils::{
        COMMA, CURLY_CLOSE, CURLY_OPEN, DOT, ROUND_CLOSE, ROUND_OPEN, SQUARE_CLOSE, SQUARE_OPEN,
        STAMENT_END_TOKENS,
    },
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
        if self.current_is(&SQUARE_OPEN) {
            self.bump();
            let mut elements = Vec::new();
            loop {
                self.skip_end_lines();
                if self.current_is(&SQUARE_CLOSE) {
                    self.bump();
                    break;
                }
                if !elements.is_empty() {
                    self.expect(&COMMA)?;
                    self.skip_end_lines();
                    if self.current_is(&SQUARE_CLOSE) {
                        self.bump();
                        break;
                    }
                }
                elements.push(self.parse_match_pattern()?);
            }
            return Ok(MatchPattern::Array(elements));
        }

        if self.current_is_ident("_") {
            self.bump();
            return Ok(MatchPattern::Wildcard);
        }


        let ident_name = match &self.token().kind {
            TokenKind::Ident(name) => name.clone(),
            _ => {

                let end_tokens = [
                    TokenKind::Symbol(SymbolKind::LambdaArray),
                    COMMA,
                    SQUARE_CLOSE,
                ];
                let expr = self.parse_expression(&end_tokens)?;
                return match expr.node {
                    ExpressionKind::Literal((_, lit)) => Ok(MatchPattern::Literal(lit)),
                    _ => Err(SoulError::new(
                        "expected a literal or '_' for match pattern",
                        SoulErrorKind::InvalidIdent,
                        Some(expr.span),
                    )),
                };
            }
        };


        if soul_utils::soul_names::KeyWord::from_str(&ident_name).is_some() {
            let end_tokens = [
                TokenKind::Symbol(SymbolKind::LambdaArray),
                COMMA,
                SQUARE_CLOSE,
            ];
            let expr = self.parse_expression(&end_tokens)?;
            return match expr.node {
                ExpressionKind::Literal((_, lit)) => Ok(MatchPattern::Literal(lit)),
                _ => Err(SoulError::new(
                    "expected a literal or '_' for match pattern",
                    SoulErrorKind::InvalidIdent,
                    Some(expr.span),
                )),
            };
        }


        let saved = self.current_position();
        let type_name_span = self.token().span;
        self.bump();
        if self.current_is(&DOT) {
            self.bump();
            let variant_name = match &self.token().kind {
                TokenKind::Ident(name) => name.clone(),
                _ => {
                    return Err(SoulError::new(
                        "expected variant name after '.' in constructor pattern",
                        SoulErrorKind::InvalidIdent,
                        Some(self.token().span),
                    ));
                }
            };
            let variant_span = self.token().span;
            self.bump();
            let binding = if self.current_is(&ROUND_OPEN) {
                self.bump();
                let inner = if self.current_is_ident("_") {
                    self.bump();
                    None
                } else if let TokenKind::Ident(bind_name) = &self.token().kind {
                    let bind_span = self.token().span;
                    let bind_name = bind_name.clone();
                    self.bump();
                    Some(Ident::new(bind_name, bind_span))
                } else {
                    None
                };
                self.expect(&ROUND_CLOSE)?;
                inner
            } else {
                None
            };
            return Ok(MatchPattern::Constructor {
                type_name: Ident::new(ident_name, type_name_span),
                variant_name: Ident::new(variant_name, variant_span),
                binding,
                binding_id: None,
            });
        }


        self.go_to(saved);
        let bind_span = self.token().span;
        self.bump();
        Ok(MatchPattern::Binding {
            ident: Ident::new(ident_name, bind_span),
            id: None,
        })
    }
}
