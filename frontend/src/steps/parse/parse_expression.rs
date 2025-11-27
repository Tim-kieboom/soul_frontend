use std::sync::LazyLock;

use models::{abstract_syntax_tree::{conditionals::{ElseKind, For, ForPattern, If}, expression::{Expression, ExpressionKind, Index, ReturnKind, ReturnLike}, expression_groups::ExpressionGroup, literal::Literal, operator::{Binary, BinaryOperator, Unary, UnaryOperator, UnaryOperatorKind}, spanned::Spanned}, error::{SoulError, SoulErrorKind, SoulResult, Span}, soul_names::{self, AccessType, KeyWord, Operator, TypeModifier}, symbool_kind::SymboolKind};

use crate::steps::{parse::{parse_statement::{CURLY_OPEN, SQUARE_CLOSE, STAMENT_END_TOKENS}, parser::{Parser, TryError}}, tokenize::token_stream::{Number, TokenKind}};

const INCREMENT: TokenKind = TokenKind::Symbool(SymboolKind::DoublePlus);
const DECREMENT: TokenKind = TokenKind::Symbool(SymboolKind::DoubleMinus);

impl<'a> Parser<'a> {

    pub(crate) fn parse_expression(&mut self, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        if self.token().span.start_line == 19 {
            println!("breakpoint")
        }
        
        let expression = self.pratt_parse_precedence(0, end_tokens)?;
        Ok(
            expression
        )
    }

    pub(crate) fn parse_if(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::If.as_str())?;

        let if_condition = self.parse_expression(&[CURLY_OPEN])?;
        let if_block = self.parse_block(TypeModifier::Mut)?;

        let mut elses = vec![];

        loop {

            while !self.current_is_any(STAMENT_END_TOKENS) && !self.current_is(&TokenKind::EndFile) {
                self.bump();
            }

            if !self.current_is_ident(KeyWord::Else.as_str()) {
                break    
            }
    
            let else_span = self.token().span;

            self.bump();
            let else_kind = if self.current_is_ident(KeyWord::If.as_str()) {
                let start_span = self.token().span;
                
                self.bump();
                let condition = self.parse_expression(&[CURLY_OPEN])?;
                let block = self.parse_block(TypeModifier::Mut)?;
                ElseKind::ElseIf(Box::new(Spanned::new(
                    If{condition: Box::new(condition), block, else_branchs: vec![]}, 
                    self.new_span(start_span),
                )))
            }
            else {
                let start_span = self.token().span;
                let block = self.parse_block(TypeModifier::Mut)?;
                ElseKind::Else(Spanned::new(block, self.new_span(start_span)))
            };

            elses.push(Spanned::new(else_kind, self.new_span(else_span)));
        }

        Ok(Expression::new(
            ExpressionKind::If(If{
                condition: Box::new(if_condition),
                block: if_block,
                else_branchs: elses,
            }), 
            self.new_span(start_span),
        ))
    }
    
    pub(crate) fn parse_for(&mut self) -> SoulResult<Expression> {
        static END_TOKENS: LazyLock<[TokenKind; 2]> = LazyLock::new(|| [
            TokenKind::Ident(KeyWord::InForLoop.as_str().to_string()),
            CURLY_OPEN,
        ]);
        
        let start_span = self.token().span;
        self.expect_ident(KeyWord::For.as_str())?;
        

        let expression = self.parse_expression(END_TOKENS.as_ref())?;
        let (element, collection) = if self.current_is_ident(KeyWord::InForLoop.as_str()) {
            self.bump();
            let collection = self.parse_expression(&[CURLY_OPEN])?;
            (Some(ForPattern::from_expression(expression)?), Box::new(collection))
        }
        else {
            (None, Box::new(expression))
        };

        let block = self.parse_block(TypeModifier::Mut)?;

        Ok(Expression::new(
            ExpressionKind::For(For{
                block,
                element,
                collection,
            }),
            self.new_span(start_span),
        ))
    }
    
    pub(crate) fn parse_while(&mut self) -> SoulResult<Expression> {
        self.expect_ident(KeyWord::While.as_str())?;
        todo!()
    }
    
    pub(crate) fn parse_return_like(&mut self, kind: ReturnKind) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(kind.as_keyword().as_str())?;
        
        let value = if self.current_is_any(STAMENT_END_TOKENS) {
            None
        }
        else {
            Some(Box::new(self.parse_expression(STAMENT_END_TOKENS)?))
        };

        self.expect_any(STAMENT_END_TOKENS)?;
        Ok(Expression::new(
            ExpressionKind::ReturnLike(ReturnLike{value, kind}), 
            self.new_span(start_span),
        ))
    }
    
    pub(crate) fn parse_match(&self) -> SoulResult<Expression> {
        todo!()
    }

    fn pratt_parse_precedence(&mut self, min_precedence: usize, end_tokens: &[TokenKind]) -> SoulResult<Expression> {
        let start_span = self.token().span;
        let mut left = self.parse_primary()?;

        loop {

            if self.current_is_any(end_tokens) {
                break
            }
            
            self.skip_end_lines();
            if self.current_is_any(end_tokens) {
                break
            }

            if self.current_is(&TokenKind::EndFile) {
                
                return Err(
                    SoulError::new(
                        format!("unexpected end of file while parsing expression"), 
                        SoulErrorKind::UnexpecedFileEnd, 
                        Some(self.new_span(start_span)),
                    )
                )
            }

            let precedence = self.current_precedence();

            // If precedence is lower than the minimum required, stop parsing more operators here
            if precedence < min_precedence {
                break
            }

            let operator = match self.consume_expression_operator(start_span)? {
                ExpressionOperator::Binary(val) => val,
                ExpressionOperator::Access(AccessType::AccessIndex) => {
                    left = self.parse_index(start_span, left)?;
                    continue
                },
                ExpressionOperator::Access(AccessType::AccessThis) => {
                    left = self.parse_access(start_span, left)?;
                    continue
                },
            };
            

            let next_min_precedence = precedence + 1;
            let right = self.pratt_parse_precedence(next_min_precedence, end_tokens)?;

            left = self.new_binary(start_span, left, operator, right);
        }

        Ok(left)
    }
    
    fn parse_primary(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;

        let expression = match &self.token().kind {
            &CURLY_OPEN => {
                
                match self.try_parse_named_tuple() {
                    Ok(named_tuple) => {
                        Expression::new(
                            ExpressionKind::ExpressionGroup(ExpressionGroup::NamedTuple(named_tuple)), 
                            self.new_span(start_span),
                        )
                    },
                    Err(TryError::IsErr(err)) => return Err(err),
                    Err(TryError::IsNotValue(_)) => {
                        let block = self.parse_block(TypeModifier::Mut)?;
                        Expression::new(ExpressionKind::Block(block), self.new_span(start_span))
                    }
                }
            },
            TokenKind::Symbool(symbool) => {

                let unary = self.expect_unary(start_span, *symbool)?;
                self.bump();

                let right = self.parse_primary()?;
                self.new_unary(start_span, unary, right)
            },
            TokenKind::Ident(ident) => {
                
                let expression = match KeyWord::from_str(&ident) {
                    Some(KeyWord::If) => self.parse_if()?,
                    Some(KeyWord::For) => self.parse_for()?,
                    Some(KeyWord::While) => self.parse_while()?,
                    Some(KeyWord::Match) => self.parse_match()?,
                    Some(KeyWord::Break) => self.parse_return_like(ReturnKind::Break)?,
                    Some(KeyWord::Return) => self.parse_return_like(ReturnKind::Return)?,
                    Some(KeyWord::Continue) => self.parse_return_like(ReturnKind::Continue)?,
                    _ => {
                        Expression::new(
                            ExpressionKind::Variable(ident.clone()),
                            self.new_span(start_span),
                        )
                    },
                };

                self.bump();
                expression
            },
            TokenKind::CharLiteral(char) => {
                let char = *char;
                self.bump();
                Expression::new_literal(Literal::Char(char), self.new_span(start_span))
            },
            TokenKind::StringLiteral(_) => {
                let token = self.bump_consume();
                let ident = match token.kind {
                    TokenKind::StringLiteral(val) => val,
                    _ => unreachable!(),
                };
                Expression::new_literal(Literal::Str(ident), self.new_span(start_span))
            },
            TokenKind::Number(Number::Int(num)) => {
                let number = *num;
                self.bump();
                Expression::new_literal(Literal::Int(number), self.new_span(start_span))
            },
            TokenKind::Number(Number::Uint(num)) => {
                let number = *num;
                self.bump();
                Expression::new_literal(Literal::Uint(number), self.new_span(start_span))
            },
            TokenKind::Number(Number::Float(num)) => {
                let number = *num;
                self.bump();
                Expression::new_literal(Literal::Float(number), self.new_span(start_span))
            },
            other => return Err(
                SoulError::new(
                    format!("'{}' is invalid as start of expression", other.display()),
                    SoulErrorKind::InvalidAssignType,
                    Some(self.new_span(start_span)),
                )
            )
        };

        if self.current_is_any(&[INCREMENT, DECREMENT]) {

            let operator = if self.current_is(&INCREMENT) {
                UnaryOperator::new(
                    UnaryOperatorKind::Increment{before_var: false}, 
                    self.new_span(start_span),
                )
            }
            else {
                UnaryOperator::new(
                    UnaryOperatorKind::Decrement{before_var: false}, 
                    self.new_span(start_span),
                )
            };
            
            self.bump();
            return Ok(self.new_unary(start_span, operator, expression));
        }

        Ok(expression)
    }

    fn parse_index(&mut self, start_span: Span, collection: Expression) -> SoulResult<Expression> {
        let index = self.parse_expression(&[SQUARE_CLOSE])?;
        self.expect(&SQUARE_CLOSE)?;

        Ok(Expression::new(
            ExpressionKind::Index(Index{collection: Box::new(collection), index: Box::new(index)}),
            self.new_span(start_span),
        ))
    }

    fn parse_access(&mut self, start_span: Span, expression: Expression) -> SoulResult<Expression> {
        todo!()
    }

    fn expect_unary(&self, start_span: Span, symbool: SymboolKind) -> SoulResult<UnaryOperator> {
        
        match Operator::from_symbool(symbool) {
            Some(op) => {
                if let Some(unary) = op.to_unary() {
                    Ok(UnaryOperator::new(unary, self.new_span(start_span)))
                }
                else {
                    Err(
                        SoulError::new(
                            format!("'{}' is not a valid unary operator", op.as_str()),
                            SoulErrorKind::InvalidOperator,
                            Some(self.new_span(start_span)),
                        )
                    )
                }
            },
            None => Err(
                SoulError::new(
                    format!("'{}' is not a valid operator", symbool.as_str()),
                    SoulErrorKind::InvalidOperator,
                    Some(self.new_span(start_span)),
                )
            ),
        }
    }

    fn new_binary(&self, start_span: Span, left: Expression, operator: BinaryOperator, right: Expression) -> Expression {
        Expression::new(
            ExpressionKind::Binary(Binary { left: Box::new(left), operator, right: Box::new(right) }),
            self.new_span(start_span)
        )
    }

    fn new_unary(&self, start_span: Span, operator: UnaryOperator, expression: Expression) -> Expression {
        Expression::new(
            ExpressionKind::Unary(Unary{operator, expression: Box::new(expression)}),
            self.new_span(start_span),
        )
    }

    fn consume_expression_operator(&mut self, start_span: Span) -> SoulResult<ExpressionOperator> {
        let get_invalid_error = || Err(SoulError::new(
            format!("invalid expression operator {}", self.token().kind.display()),
            SoulErrorKind::InvalidOperator,
            Some(start_span),
        ));
        
        match &self.token().kind {

            TokenKind::Symbool(sym) => {

                if let Some(access) = AccessType::from_symbool(*sym) {
                    self.bump();
                    return Ok(ExpressionOperator::Access(access));
                }
                else if let Some(Some(binary)) = Operator::from_symbool(*sym).map(|el| el.to_binary()) {
                    self.bump();
                    return Ok(ExpressionOperator::Binary(BinaryOperator::new(binary, self.new_span(start_span))));
                }

                return get_invalid_error()
            }

            _ => return get_invalid_error()
        }
    }

    fn current_precedence(&self) -> usize {
        
        match &self.token().kind {

            TokenKind::Ident(ident) => {

                if let Some(keyword) = soul_names::KeyWord::from_str(&ident) {
                    keyword.precedence()
                }
                else {
                    0
                }
            },
            TokenKind::Symbool(symbool_kind) => {

                if let Some(access) = soul_names::AccessType::from_symbool(*symbool_kind) {
                    access.precedence()
                }
                else if let Some(op) = soul_names::Operator::from_symbool(*symbool_kind) {
                    op.precedence()
                }
                else {
                    0
                }
            },
            _ => 0,
        }
    } 
}

enum ExpressionOperator {
    Binary(BinaryOperator),
    Access(AccessType),
}