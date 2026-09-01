use std::str::FromStr;

use crate::{
    parser::Parser,
    utils::{
        COLON, COLON_ASSIGN, COMMA, CURLY_CLOSE, CURLY_OPEN, DOT, DOUBLE_DOT, ELSE, IF,
        LAMBDA_ARROW, MATCH, NOT, NULL, OR, ROUND_CLOSE, ROUND_OPEN, SQUARE_CLOSE, SQUARE_OPEN,
        STAMENT_END_TOKENS,
    },
};
use ast_model::{
    block::{Block, BlockId},
    expression::{
        Binding, ConstructorStructPattern, Expression, ExpressionId, ExpressionKind, If, IfBranch,
        IfCondition, Match, MatchArm, MatchContructor, MatchPattern, NamedMatchPattern,
        NamedTupleMatchPattern, TupleMatchPattern,
    },
    statements::Statement,
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident, TypeModifier, collections::try_result::ToResult, error::SoulResult, fault::Fault,
    ids::IdAlloc, span::Span,
};

const IF_STR: &str = KeyWord::If.as_str();
const ELSE_STR: &str = KeyWord::Else.as_str();

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_if(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect(&IF)?;

        let condition = if self.current_is_keyword(KeyWord::Type) {
            self.bump();
            let condition = self.parse_type_assert()?;
            let block = self.parse_block(TypeModifier::Mut)?;
            let mut r#if = If {
                condition,
                block,
                branch: None,
            };
            self.parse_if_arms(&mut r#if.branch)?;
            return Ok(Expression::new(
                ExpressionKind::If(r#if),
                self.span_combine(start_span),
            ));
        } else {
            match self.parse_expression_id(&[CURLY_OPEN]) {
                Ok(val) => val,
                Err(err) => {
                    self.log_fault(err);
                    self.skip_till(&[CURLY_OPEN]);
                    ExpressionId::error()
                }
            }
        };
        let if_block = self.parse_block(TypeModifier::Mut)?;

        let mut r#if = If {
            condition: IfCondition::Expression(condition),
            block: if_block,
            branch: None,
        };

        self.parse_if_arms(&mut r#if.branch)?;
        Ok(Expression::new(
            ExpressionKind::If(r#if),
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn parse_type_assert(&mut self) -> SoulResult<IfCondition> {
        let pattern = self.inner_parse_match_pattern()?;
        self.skip_end_lines();
        if self.current_is(&COLON) {
            self.bump();
            let ty = self.try_parse_type().merge_to_result()?;
            let binding = match pattern {
                MatchPattern::Binding(binding) => binding,
                MatchPattern::NotNull(binding) => binding,
                _ => return Err(Fault::error("expected ident", Some(self.token().span))),
            };
            self.skip_end_lines();
            self.expect(&COLON_ASSIGN)?;
            let scrutinee = self.parse_expression_id(&[CURLY_OPEN])?;
            return Ok(IfCondition::CastType {
                binding,
                ty,
                scrutinee,
            });
        }

        self.skip_end_lines();
        self.expect(&COLON_ASSIGN)?;
        let scrutinee = self.parse_expression_id(&[CURLY_OPEN])?;
        Ok(IfCondition::MatchType { pattern, scrutinee })
    }

    pub(crate) fn parse_match(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect(&MATCH)?;

        let scrutinee = self.parse_expression_id(&[CURLY_OPEN])?;

        let arms = self.parse_match_arms()?;

        Ok(Expression::new(
            ExpressionKind::Match(Match { arms, scrutinee }),
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
                .forest
                .store
                .insert_statement(Statement::from_expression(
                    &self.forest.store,
                    expression,
                    false,
                ));
            let block = self.forest.store.insert_block(Block {
                is_const: false,
                statements: vec![statement],
                span: self.span_combine(start_span),
            });
            return Ok((Some(Binding::new(self.alloc_node(), ident)), block));
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
                    continue;
                }
            };

            self.skip_end_lines();

            if !self.current_is(&LAMBDA_ARROW) {
                return Err(Fault::error(
                    "expected '=>' in match arm",
                    Some(self.token().span),
                ));
            }
            self.bump();

            let body = if self.current_is(&CURLY_OPEN) {
                self.parse_block(TypeModifier::Mut)?
            } else {
                let span = self.token().span;
                let expr = self.parse_expression_id(&[
                    COMMA,
                    CURLY_CLOSE,
                    TokenKind::EndFile,
                    TokenKind::EndLine,
                ])?;
                let statement = self
                    .forest
                    .store
                    .insert_statement(Statement::from_expression(&self.forest.store, expr, false));
                self.forest.store.insert_block(Block {
                    span,
                    is_const: false,
                    statements: vec![statement],
                })
            };
            arms.push(MatchArm { pattern, body });

            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                continue;
            }
            self.bump();
        }

        Ok(arms)
    }

    fn parse_if_arms(&mut self, head: &mut Option<Box<IfBranch>>) -> SoulResult<()> {
        let mut tail = head;
        let mut has_else = false;

        loop {
            let position = self.tokens.current_position();
            self.skip_while_any(STAMENT_END_TOKENS);

            let start_span = self.token().span;
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
                if self.current_is_keyword(KeyWord::Type) {
                    self.bump();
                    let condition = self.parse_type_assert()?;
                    let block = self.parse_block(TypeModifier::Mut)?;
                    IfBranch::If(If {
                        condition,
                        block,
                        branch: None,
                    })
                } else {
                    let condition = self.parse_expression_id(&[CURLY_OPEN])?;
                    let block = self.parse_block(TypeModifier::Mut)?;
                    IfBranch::If(If {
                        condition: IfCondition::Expression(condition),
                        block,
                        branch: None,
                    })
                }
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
        let pattern = self.inner_parse_match_pattern()?;
        self.skip_end_lines();
        Ok(match self.token().kind {
            OR => {
                self.bump();

                let mut chain = vec![pattern];
                loop {
                    chain.push(self.inner_parse_match_pattern()?);
                    self.skip_end_lines();
                    if !self.current_is(&OR) {
                        break;
                    }

                    self.bump();
                }
                MatchPattern::Fallthrough(chain)
            }
            IF => {
                self.bump();
                let if_condition = self.parse_expression_id(&[LAMBDA_ARROW])?;
                MatchPattern::If {
                    pattern: Box::new(pattern),
                    if_condition,
                }
            }
            _ => pattern,
        })
    }

    fn inner_parse_match_pattern(&mut self) -> SoulResult<MatchPattern> {
        if self.current_is(&SQUARE_OPEN) {
            self.bump();
            let mut elements = Vec::new();
            let mut first = true;
            loop {
                self.skip_end_lines();
                if self.current_is(&SQUARE_CLOSE) {
                    self.bump();
                    break;
                }
                if !first {
                    self.expect(&COMMA)?;
                    self.skip_end_lines();
                    if self.current_is(&SQUARE_CLOSE) {
                        self.bump();
                        break;
                    }
                }
                first = false;

                if self.current_is(&DOUBLE_DOT) {
                    self.bump();
                    elements.push(MatchPattern::Rest);
                    break;
                }

                elements.push(self.parse_match_pattern()?);
            }
            return Ok(MatchPattern::Array(elements));
        }

        if self.current_is(&ROUND_OPEN) {
            return self.parse_match_tuple_pattern();
        }

        if self.current_is(&CURLY_OPEN) {
            return self.parse_match_named_tuple_pattern();
        }

        if self.current_is_ident("_") {
            self.bump();
            return Ok(MatchPattern::Wildcard);
        }

        if self.current_is(&NOT) && self.peek_is(&NULL) {
            self.bump();
            self.bump();

            self.expect(&ROUND_OPEN)?;
            let binding = self.try_bump_consume_ident()?;
            self.expect(&ROUND_CLOSE)?;
            return Ok(MatchPattern::NotNull(Binding::new(
                self.alloc_node(),
                binding,
            )));
        }

        if self.current_is(&NULL) {
            self.bump();
            return Ok(MatchPattern::Null);
        }

        let ident_name = match &self.token().kind {
            TokenKind::Ident(name) => name.clone(),
            _ => {
                let end_tokens = [
                    LAMBDA_ARROW,
                    IF,
                    COMMA,
                    SQUARE_CLOSE,
                    CURLY_CLOSE,
                    ROUND_CLOSE,
                ];
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

        if KeyWord::from_str(&ident_name).is_ok() {
            let end_tokens = [
                LAMBDA_ARROW,
                IF,
                COMMA,
                SQUARE_CLOSE,
                CURLY_CLOSE,
                ROUND_CLOSE,
            ];
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
        let type_name_span = self.token().span;
        self.bump();

        // Try constructor struct pattern: TypeName{field, ...}
        if self.current_is(&CURLY_OPEN) {
            return self
                .parse_match_constructor_struct_pattern(Ident::new(ident_name, type_name_span));
        }

        if self.current_is(&ROUND_OPEN) {
            self.bump();
            let binding = if self.current_is_ident("_") {
                self.bump();
                None
            } else if let TokenKind::Ident(bind_name) = &self.token().kind {
                let bind_span = self.token().span;
                let bind_name = bind_name.clone();
                self.bump();
                let ident = Ident::new(bind_name, bind_span);
                Some(Binding::new(self.forest.store.alloc_node(), ident))
            } else {
                None
            };
            self.expect(&ROUND_CLOSE)?;
            return Ok(MatchPattern::Constructor(MatchContructor {
                type_name: Ident::new(ident_name.clone(), type_name_span),
                variant_name: Ident::new(ident_name, type_name_span),
                binding,
            }));
        }

        if self.current_is(&DOT) {
            self.bump();
            let variant_name = match &self.token().kind {
                TokenKind::Ident(name) => name.clone(),
                _ => {
                    return Err(Fault::error(
                        "expected variant name after '.' in constructor pattern",
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
                    let ident = Ident::new(bind_name, bind_span);
                    Some(Binding::new(self.forest.store.alloc_node(), ident))
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
            }));
        }

        self.goto(saved);
        let bind_span = self.token().span;
        self.bump();
        Ok(MatchPattern::Binding(Binding {
            id: self.forest.store.alloc_node(),
            ident: Ident::new(ident_name, bind_span),
        }))
    }

    fn parse_match_tuple_pattern(&mut self) -> SoulResult<MatchPattern> {
        self.expect(&ROUND_OPEN)?;
        let mut elements = Vec::new();
        let mut rest = false;

        let mut first = true;
        loop {
            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }

            if !first {
                self.expect(&COMMA)?;
                self.skip_end_lines();
                if self.current_is(&ROUND_CLOSE) {
                    break;
                }
            }
            first = false;

            if self.current_is(&DOUBLE_DOT) {
                rest = true;
                self.bump();
                break;
            }

            elements.push(self.parse_match_pattern()?);
        }

        self.expect(&ROUND_CLOSE)?;
        Ok(MatchPattern::Tuple(TupleMatchPattern { elements, rest }))
    }

    fn parse_match_named_tuple_pattern(&mut self) -> SoulResult<MatchPattern> {
        self.expect(&CURLY_OPEN)?;
        let mut fields = Vec::new();
        let mut rest = false;

        let mut first = true;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            if !first {
                self.expect(&COMMA)?;
                self.skip_end_lines();
                if self.current_is(&CURLY_CLOSE) {
                    break;
                }
            }
            first = false;

            if self.current_is(&DOUBLE_DOT) {
                rest = true;
                self.bump();
                break;
            }

            let field = self.try_bump_consume_ident()?;

            let binding = if self.current_is(&COLON) {
                self.bump();
                if self.current_is_ident("_") {
                    self.bump();
                    None
                } else {
                    let alias = self.try_bump_consume_ident()?;
                    Some(Binding::new(self.alloc_node(), alias))
                }
            } else {
                Some(Binding::new(self.alloc_node(), field.clone()))
            };

            fields.push(NamedMatchPattern { field, binding });
        }

        self.expect(&CURLY_CLOSE)?;
        Ok(MatchPattern::NamedTuple(NamedTupleMatchPattern {
            fields,
            rest,
        }))
    }

    fn parse_match_constructor_struct_pattern(
        &mut self,
        type_name: Ident,
    ) -> SoulResult<MatchPattern> {
        self.expect(&CURLY_OPEN)?;
        let mut fields = Vec::new();
        let mut rest = false;

        let mut first = true;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            if !first {
                self.expect(&COMMA)?;
                self.skip_end_lines();
                if self.current_is(&CURLY_CLOSE) {
                    break;
                }
            }
            first = false;

            if self.current_is(&DOUBLE_DOT) {
                rest = true;
                self.bump();
                break;
            }

            let field = self.try_bump_consume_ident()?;

            let binding = if self.current_is(&COLON) {
                self.bump();
                if self.current_is_ident("_") {
                    self.bump();
                    None
                } else {
                    let alias = self.try_bump_consume_ident()?;
                    Some(Binding::new(self.alloc_node(), alias))
                }
            } else {
                Some(Binding::new(self.alloc_node(), field.clone()))
            };

            fields.push(NamedMatchPattern { field, binding });
        }

        self.expect(&CURLY_CLOSE)?;
        Ok(MatchPattern::ConstructorStruct(ConstructorStructPattern {
            type_name,
            fields,
            rest,
        }))
    }

    fn skip_match_pattern(&mut self) {
        self.skip_till(&[CURLY_OPEN, TokenKind::EndLine]);
        if !self.current_is(&CURLY_OPEN) {
            return;
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
                break;
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
