use parser_models::ast::{Array, SoulType};
use soul_utils::{error::SoulResult, span::Spanned, try_result::TryError};

use crate::parser::{
    Parser,
    parse_utils::{COLON, COMMA, SQUARE_CLOSE, SQUARE_OPEN},
};

impl<'a> Parser<'a> {
    pub fn parse_array(&mut self, ty: Option<SoulType>) -> SoulResult<Spanned<Array>> {
        let start_span = self.token().span;
        self.expect(&SQUARE_OPEN)?;

        let element_type = match self.try_parse_type() {
            Ok(ty) => {
                self.expect(&COLON)?;
                self.skip_end_lines();
                Some(ty)
            }
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(_)) => None,
        };

        let mut values = vec![];
        loop {
            self.skip_end_lines();
            if self.current_is(&SQUARE_CLOSE) {
                break;
            }

            let element = self.parse_expression(&[SQUARE_CLOSE, COMMA])?;
            values.push(element);

            self.skip_end_lines();
            if self.current_is(&SQUARE_CLOSE) {
                break;
            }

            self.expect(&COMMA)?;
        }

        self.skip_end_lines();
        self.expect(&SQUARE_CLOSE)?;
        Ok(Spanned::new(
            Array {
                collection_type: ty,
                element_type,
                values,
                id: None,
            },
            self.span_combine(start_span),
        ))
    }
}
