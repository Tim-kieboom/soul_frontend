use ast_model::{
    expression::{Expression, ExpressionId},
    soul_type::SoulType,
};
use soul_tokenizer::model::TokenKind;
use soul_utils::{
    collections::try_result::{ResultTryErr, TryError, TryErr, TryNotValue, TryOk},
    soul_error_internal,
};

use crate::{
    fault::AstTryResult, parser::Parser, utils::{ARROW_LEFT, ARROW_RIGHT, ASSIGN, COMMA},
};

mod expression;
mod function;
mod parse_module;
mod soul_type;
mod statements;

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_generic_define(
        &mut self,
    ) -> AstTryResult<Vec<SoulType>, crate::fault::AstFault> {
        let start_position = self.tokens.current_position();

        self.expect(&ARROW_LEFT).try_err()?;
        let mut types = vec![];
        loop {
            if let TokenKind::Ident(_) = self.token().kind
                && self.peek_is(&ASSIGN)
            {
                self.bump();
                self.bump();
                let value = match self.try_parse_type() {
                    Ok(val) => val,
                    Err(TryError::IsErr(err)) => return TryErr(err),
                    Err(TryError::IsNotValue(err)) => {
                        return TryNotValue(err);
                    }
                };
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

            let ty = match self.try_parse_type() {
                Ok(val) => val,
                Err(TryError::IsErr(err)) => return TryErr(err),
                Err(TryError::IsNotValue(err)) => {
                    return TryNotValue(err);
                }
            };
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

    pub(crate) fn get_forest_expression(
        &self,
        id: ExpressionId,
    ) -> Result<&Expression, crate::fault::AstFault> {
        self.forest
            .store
            .expressions
            .get(id)
            .ok_or_else(|| soul_error_internal!(format!("{id:?} not found"), None).into_kind())
    }
}
