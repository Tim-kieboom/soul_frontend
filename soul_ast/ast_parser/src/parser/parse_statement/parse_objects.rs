use ast::{Field, Statement, Struct};
use soul_utils::{
    error::{SoulError, SoulResult},
    soul_names::{KeyWord, TypeModifier},
    try_result::{ResultTryErr, ResultTryNotValue, ToResult, TryErr, TryOk, TryResult},
};

use crate::parser::{
    Parser,
    parse_utils::{COLON, CURLY_CLOSE, CURLY_OPEN, STAMENT_END_TOKENS},
};

impl<'f, 'a> Parser<'f, 'a> {
    pub fn parse_struct(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::Struct.as_str())?;

        let name = self.try_bump_consume_ident()?;
        let generics = self.parse_generic_declare()?.unwrap_or(vec![]);
        self.skip_end_lines();

        self.expect(&CURLY_OPEN)?;
        let mut fields = vec![];
        loop {
            self.skip_end_lines();
            match self.parse_field().merge_to_result() {
                Ok(field) => fields.push(field),
                Err(err) => {
                    self.log_error(err);
                    break;
                }
            }

            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }
        }
        self.expect(&CURLY_CLOSE)?;

        let obj = Struct {
            id: None,
            name,
            fields,
            generics,
        };

        Ok(Statement::new(
            ast::StatementKind::Struct(obj),
            self.span_combine(start_span),
        ))
    }

    fn parse_field(&mut self) -> TryResult<Field, SoulError> {
        let mut name = self.try_bump_consume_ident().try_err()?;
        let modifier = TypeModifier::from_str(name.as_str());
        if modifier.is_some() {
            name = self.try_bump_consume_ident().try_err()?;
        }

        self.expect(&COLON).try_not_value()?;
        let mut ty = self.try_parse_type()?;
        ty.modifier = Some(modifier.unwrap_or(TypeModifier::Const));

        if !self.current_is_any(STAMENT_END_TOKENS) {
            return TryErr(self.get_expect_any_error(STAMENT_END_TOKENS));
        }

        TryOk(Field { id: None, name, ty })
    }
}
