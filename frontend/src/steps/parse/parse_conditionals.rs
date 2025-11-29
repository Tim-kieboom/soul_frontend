use std::sync::LazyLock;

use crate::steps::{parse::{parse_statement::{CURLY_OPEN, STAMENT_END_TOKENS}, parser::Parser}, tokenize::token_stream::TokenKind};
use models::{abstract_syntax_tree::{conditionals::{ElseKind, For, ForPattern, If}, expression::{Expression, ExpressionKind}, spanned::Spanned}, error::{SoulError, SoulErrorKind, SoulResult}, soul_names::{KeyWord, TypeModifier}};

impl<'a> Parser<'a> {
    pub(crate) fn parse_if(&mut self) -> SoulResult<Expression> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::If.as_str())?;

        let if_condition = self.parse_expression(&[CURLY_OPEN])?;
        let if_block = self.parse_block(TypeModifier::Mut)?;

        let mut elses = vec![];
        let mut has_else = false;
        loop {

            while self.current_is_any(STAMENT_END_TOKENS) && !self.current_is(&TokenKind::EndFile) {
                self.bump();
            }

            let start_span = self.token().span;
            if !self.current_is_ident(KeyWord::Else.as_str()) {
                break    
            }

            if has_else {
                const ELSE: &str = KeyWord::Else.as_str();
                const IF: &str = KeyWord::If.as_str();

                return Err(
                    SoulError::new(
                        format!("can not have '{ELSE}' or '{ELSE} {IF}' after '{ELSE}'"),
                        SoulErrorKind::InvalidContext,
                        Some(start_span),
                    )
                )
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
                has_else = true;
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
}