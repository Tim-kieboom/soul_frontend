use ast_model::{expression::{Expression, ExpressionId}, soul_type::SoulType};
use soul_tokenizer::model::TokenKind;
use soul_utils::{
    collections::try_result::{ResultTryErr, TryNotValue, TryOk, TryResult}, error::SoulResult, fault::Fault, soul_error_internal,
};

use crate::{
    parser::Parser,
    utils::{ARROW_LEFT, ARROW_RIGHT, ASSIGN, COMMA},
};

mod expression;
mod function;
mod parse_module;
mod soul_type;
mod statements;

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_generic_define(&mut self) -> TryResult<Vec<SoulType>, Fault> {
        let start_position = self.tokens.current_position();

        self.expect(&ARROW_LEFT).try_err()?;
        let mut types = vec![];
        loop {
            if let TokenKind::Ident(_) = self.token().kind
                && self.peek_is(&ASSIGN)
            {
                self.bump();
                self.bump();
                let value = self.try_parse_type()?;
                types.push(value);
                if self.current_is(&ARROW_RIGHT) {
                    self.bump();
                    break;
                }
                if !self.current_is(&COMMA) {
                    self.goto(start_position);
                    return TryNotValue(self.get_expect_error(&COMMA));
                }
                self.bump();
                continue;
            }

            let ty = self.try_parse_type()?;
            types.push(ty);

            if self.current_is(&ARROW_RIGHT) {
                self.bump();
                break;
            }

            if !self.current_is(&COMMA) {
                self.goto(start_position);
                return TryNotValue(self.get_expect_error(&COMMA));
            }
            self.bump();
        }
        TryOk(types)
    }

    pub(crate) fn get_forest_expression(&self, id: ExpressionId) -> SoulResult<&Expression> {
        self.forest.store.expressions.get(id).ok_or(soul_error_internal!(format!("{id:?} not found"), None))
    }
}
