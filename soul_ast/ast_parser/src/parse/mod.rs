use ast_model::soul_type::SoulType;
use soul_utils::{
    collections::try_result::{ResultTryErr, TryNotValue, TryOk, TryResult},
    fault::Fault,
};

use crate::{
    parser::Parser,
    utils::{ARROW_LEFT, ARROW_RIGHT, COMMA},
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
}
