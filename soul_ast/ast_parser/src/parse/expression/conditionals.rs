use crate::{
    parser::Parser,
    utils::{
        COMMA, CURLY_CLOSE, CURLY_OPEN, DOT, ELSE, IF, LAMBDA_ARROW, MATCH, NOT, NULL, ROUND_CLOSE, ROUND_OPEN, SQUARE_CLOSE, SQUARE_OPEN, STAMENT_END_TOKENS
    },
};
use ast_model::{
    block::{Block, BlockId},
    expression::{
        Binding, Expression, ExpressionKind, If, IfBranch, Match, MatchArm, MatchContructor,
        MatchPattern,
    },
    statements::Statement,
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident, TypeModifier, error::SoulResult, fault::Fault, soul_names::Symbol, span::Span,
};

const IF_STR: &str = KeyWord::If.as_str();
const ELSE_STR: &str = KeyWord::Else.as_str();

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_if(&mut self) -> SoulResult<Expression> {
        let start_span = self.current().span;
        self.expect(&IF)?;

        let condition = self.parse_expression_id(&[CURLY_OPEN])?;
        let if_block = self.parse_block(TypeModifier::Mut)?;

        let mut r#if = If {
            condition,
            id: None,
            block: if_block,
            branch: None,
        };

        self.parse_if_arms(&mut r#if.branch)?;
        Ok(Expression::new(
            ExpressionKind::If(r#if),
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn parse_match(&mut self) -> SoulResult<Expression> {
        let start_span = self.current().span;
        self.expect(&MATCH)?;

        let scrutinee = self.parse_expression_id(&[CURLY_OPEN])?;

        let arms = self.parse_match_arms()?;

        Ok(Expression::new(
            ExpressionKind::Match(Match {
                arms,
                id: None,
                scrutinee,
            }),
            start_span,
        ))
    }

    pub(crate) fn parse_match_method_arm(
        &mut self,
        start_span: Span,
    ) -> SoulResult<(Option<Binding>, BlockId)> {
        let save_pos = self.tokens.current_position();

        self.expect(&CURLY_OPEN)?;
        self.skip_end_lines();

        let Ok(ident) = self.try_bump_consume_ident() else {
            self.goto(save_pos);
            let body = self.parse_block(TypeModifier::Mut)?;
            return Ok((None, body));
        };

        self.skip_end_lines();
        if self.current_is(&LAMBDA_ARROW) {
            self.bump();
            let expression =
                self.parse_expression_id(&[CURLY_CLOSE, TokenKind::EndLine, TokenKind::EndFile])?;
            self.expect(&CURLY_CLOSE)?;
            let statement = self
                .store
                .insert_statement(Statement::from_expression(self.store, expression, false));
            let block = self.store.insert_block(Block {
                modifier: TypeModifier::Mut,
                statements: vec![statement],
                scope_id: None,
                node_id: None,
                span: self.span_combine(start_span),
            });
            return Ok((Some(Binding::new(ident)), block));
        }

        self.goto(save_pos);
        let body = self.parse_block(TypeModifier::Mut)?;
        Ok((None, body))
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

            let pattern = match self.parse_match_pattern() {
                Ok(val) => val,
                Err(err) => {
                    self.log_fault(err);
                    self.skip_match_pattern();
                    continue
                }
            };
            self.skip_end_lines();

            if !self.current_is(&TokenKind::Symbol(Symbol::LambdaArrow)) {
                return Err(Fault::error(
                    "expected '=>' in match arm",
                    Some(self.current().span),
                ));
            }
            self.bump();

            let body = if self.current_is(&CURLY_OPEN) {
                self.parse_block(TypeModifier::Mut)?
            } else {
                let span = self.current().span;
                let statement = self.parse_statement_id()?;
                self.store.insert_block(Block {
                    span,
                    node_id: None,
                    scope_id: None,
                    modifier: TypeModifier::Mut,
                    statements: vec![statement],
                })
            };
            arms.push(MatchArm { pattern, body });
        }

        Ok(arms)
    }

    fn parse_if_arms(&mut self, head: &mut Option<Box<IfBranch>>) -> SoulResult<()> {
        let mut tail = head;
        let mut has_else = false;

        loop {
            let position = self.tokens.current_position();
            self.skip_while_any(STAMENT_END_TOKENS);

            let start_span = self.current().span;
            if !self.current_is(&ELSE) {
                self.goto(position);
                break Ok(());
            }

            if has_else {
                return Err(Fault::error(
                    format!(
                        "can not have '{ELSE_STR}' or '{ELSE_STR} {IF_STR}' after '{ELSE_STR}'"
                    ),
                    Some(start_span),
                ));
            }

            self.bump();
            let else_kind = if self.current_is(&IF) {
                self.bump();
                let condition = self.parse_expression_id(&[CURLY_OPEN])?;
                let block = self.parse_block(TypeModifier::Mut)?;
                IfBranch::If(If {
                    condition,
                    block,
                    id: None,
                    branch: None,
                })
            } else {
                has_else = true;
                let block = self.parse_block(TypeModifier::Mut)?;
                IfBranch::Else(block)
            };

            *tail = Some(Box::new(else_kind));
            tail = match try_next_mut(tail.as_mut().expect("just made Some(_)")) {
                Some(val) => val,
                None => return Ok(()),
            };
        }
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

        if self.current_is(&NOT) && self.peek_is(&NULL) {
            let not_span = self.current().span;
            self.bump();
            let null_span = self.current().span;
            self.bump();
            
            let type_name = Ident::new(KeyWord::Null.as_str(), null_span);
            let variant_name = Ident::new(Symbol::Not.as_str(), not_span);
            self.expect(&ROUND_OPEN)?;
            let binding = Some(self.try_bump_consume_ident()?);
            self.expect(&ROUND_CLOSE)?;
            return Ok(MatchPattern::Constructor(MatchContructor {
                binding,
                type_name,
                variant_name,
                binding_id: None,
            }))
        } 
        
        if self.current_is(&NULL) {
            let span = self.current().span;
            self.bump();
            let binding = Binding::from_text(KeyWord::Null.as_str(), span);
            return Ok(MatchPattern::Binding(binding))
        }

        let ident_name = match &self.current().kind {
            TokenKind::Ident(name) => name.clone(),
            _ => {
                let end_tokens = [TokenKind::Symbol(Symbol::LambdaArrow), COMMA, SQUARE_CLOSE];
                let expr = self.parse_expression(&end_tokens)?;
                return match expr.node {
                    ExpressionKind::Literal((_, lit)) => Ok(MatchPattern::Literal(lit)),
                    _ => Err(Fault::error(
                        "expected a literal or '_' for match pattern",
                        Some(expr.span),
                    )),
                };
            }
        };

        if KeyWord::from_str(&ident_name).is_some() {
            let end_tokens = [TokenKind::Symbol(Symbol::LambdaArrow), COMMA, SQUARE_CLOSE];
            let expr = self.parse_expression(&end_tokens)?;
            return match expr.node {
                ExpressionKind::Literal((_, lit)) => Ok(MatchPattern::Literal(lit)),
                _ => Err(Fault::error(
                    "expected a literal or '_' for match pattern",
                    Some(expr.span),
                )),
            };
        }

        let saved = self.tokens.current_position();
        let type_name_span = self.current().span;
        self.bump();
        if self.current_is(&DOT) {
            self.bump();
            let variant_name = match &self.current().kind {
                TokenKind::Ident(name) => name.clone(),
                _ => {
                    return Err(Fault::error(
                        "expected variant name after '.' in constructor pattern",
                        Some(self.current().span),
                    ));
                }
            };
            let variant_span = self.current().span;
            self.bump();
            let binding = if self.current_is(&ROUND_OPEN) {
                self.bump();
                let inner = if self.current_is_ident("_") {
                    self.bump();
                    None
                } else if let TokenKind::Ident(bind_name) = &self.current().kind {
                    let bind_span = self.current().span;
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
            return Ok(MatchPattern::Constructor(MatchContructor {
                type_name: Ident::new(ident_name, type_name_span),
                variant_name: Ident::new(variant_name, variant_span),
                binding,
                binding_id: None,
            }));
        }

        self.goto(saved);
        let bind_span = self.current().span;
        self.bump();
        Ok(MatchPattern::Binding(Binding {
            id: None,
            ident: Ident::new(ident_name, bind_span),
        }))
    }
    
    fn skip_match_pattern(&mut self) {
        self.skip_till(&[CURLY_OPEN, TokenKind::EndLine]);
        if !self.current_is(&CURLY_OPEN) {
            return
        }

        let mut bracket_stack = 1;
        loop {
            self.bump();
            if self.current_is(&CURLY_OPEN) {
                bracket_stack += 1;
            }

            if self.current_is(&CURLY_CLOSE) {
                bracket_stack -= 1;
            }

            if bracket_stack == 0 {
                self.bump();
                break
            }
        }
    }
}

fn try_next_mut(arm: &mut IfBranch) -> Option<&mut Option<Box<IfBranch>>> {
    match arm {
        IfBranch::If(arm) => Some(&mut arm.branch),
        IfBranch::Else(_) => None,
    }
}
