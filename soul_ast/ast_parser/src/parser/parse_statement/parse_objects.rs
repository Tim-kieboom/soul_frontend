use ast::{Enum, Field, Statement, Struct};
use soul_utils::{
    error::{SoulError, SoulResult},
    soul_names::{KeyWord, TypeModifier},
    try_result::{ResultTryErr, ResultTryNotValue, ToResult, TryErr, TryOk, TryResult},
};

use crate::parser::{
    Parser,
    parse_utils::{COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, STAMENT_END_TOKENS},
};

impl<'f, 'a> Parser<'f, 'a> {
    pub(crate) fn parse_enum(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::Enum.as_str())?;
        let name = self.try_bump_consume_ident()?;
        
        let mut variant = vec![];
        self.expect(&CURLY_OPEN)?;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }
            
            variant.push(
                self.try_bump_consume_ident()?
            );

            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }
            self.bump();
        }
        self.skip_end_lines();
        if !self.current_is(&CURLY_CLOSE) {
            
            return Err(SoulError::new(
                format!("expected: '{}' or '{}' but found: '{}'", CURLY_CLOSE.display(), COMMA.display(), self.token().kind.display()), 
                soul_utils::error::SoulErrorKind::InvalidTokenKind, 
                Some(self.token().span),
            ))
        }

        Ok(Statement::new(
            ast::StatementKind::Enum(Enum{ id: None, name, variants: variant }), 
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn parse_struct(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::Struct.as_str())?;

        let name = self.try_bump_consume_ident()?;
        let generics = self.parse_generic_declare()?.unwrap_or(vec![]);
        self.skip_end_lines();

        self.expect(&CURLY_OPEN)?;
        let mut fields = vec![];
        loop {
            self.skip_end_lines();

            if self.current_is(&CURLY_CLOSE) {
                break;
            }
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
            defined_in: None,
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
