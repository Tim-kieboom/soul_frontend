use ast_model::{NodeId, expression::{ExpressionId, For, ForCondition, ForElementKind}};
use soul_tokenizer::model::TokenKind;
use soul_utils::{
    Ident, TypeModifier, error::SoulResult, fault::Fault, soul_names::Symbol, span::Spanned,
};

use crate::{parser::Parser, utils::{CURLY_OPEN, FOR, IN}};

impl<'a, 'f> Parser<'a, 'f> {
    pub fn parse_for_loop(&mut self) -> SoulResult<Spanned<For>> {
        let start_span = self.token().span;
        self.expect(&FOR)?;

        let condition = match &self.token().kind {
            &CURLY_OPEN => ForCondition::Loop,
            _ => {
                let saved = self.tokens.current_position();

                match self.try_parse_foreach_elements() {
                    Ok(Some((element, collection))) => ForCondition::Foreach {
                        element,
                        index: None,
                        collection,
                    },
                    Ok(None) => {
                        self.goto(saved);
                        let value = self.parse_expression_id(&[CURLY_OPEN])?;
                        ForCondition::While(value)
                    }
                    Err(err) => return Err(err),
                }
            }
        };

        let block = self.parse_block(TypeModifier::Mut)?;
        Ok(Spanned::new(
            For {
                id: None,
                block,
                condition,
            },
            self.span_combine(start_span),
        ))
    }

    fn try_parse_foreach_elements(&mut self) -> SoulResult<Option<(ForElementKind, ExpressionId)>> {
        let mut idents: Vec<(Option<NodeId>, Ident)> = Vec::new();

        match &self.token().kind {
            TokenKind::Ident(name) => {
                let span = self.token().span;
                let name = name.clone();
                self.bump();
                idents.push((None, Ident::new(name, span)));
            }
            _ => return Ok(None),
        }

        loop {
            self.skip_end_lines();
            match &self.token().kind {
                &IN => {
                    self.bump();
                    let collection = self.parse_expression_id(&[CURLY_OPEN])?;
                    let element = if idents.len() == 1 {
                        ForElementKind::Single(idents.remove(0))
                    } else {
                        ForElementKind::Tuple(idents)
                    };
                    return Ok(Some((element, collection)));
                }
                TokenKind::Symbol(Symbol::Comma) => {
                    self.bump();
                    self.skip_end_lines();
                    match &self.token().kind {
                        TokenKind::Ident(name) => {
                            let span = self.token().span;
                            let name = name.clone();
                            self.bump();
                            idents.push((None, Ident::new(name, span)));
                        }
                        _ => {
                            return Err(Fault::error(
                                "expected identifier after ',' in foreach element",
                                Some(self.token().span),
                            ));
                        }
                    }
                }
                _ => return Ok(None),
            }
        }
    }
}