use crate::parser::{
    Parser,
    parse_utils::{ARROW_LEFT, COLON, CURLY_OPEN, ROUND_OPEN, STAMENT_END_TOKENS},
};
use parser_models::ast::{SoulType, Statement, StatementHelpers, Variable};
use soul_tokenizer::TokenKind;
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{AssignType, KeyWord, TypeModifier},
    span::Span,
    try_result::{ResultMapNotValue, ResultTryErr, ToResult, TryErr, TryError, TryOk, TryResult},
};

const ILIGAL_NAMES: &[&[&str]] = &[KeyWord::VALUES, TypeModifier::VALUES];

impl<'a> Parser<'a> {
    pub(super) fn try_parse_from_modifier(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
    ) -> TryResult<Statement, SoulError> {
        self.bump();

        if self.current_is(&CURLY_OPEN) {
            let block = self.parse_block(modifier).try_err()?;
            return TryOk(Statement::new_block(block, self.span_combine(start_span)));
        }

        let name = match self.try_consume_name().try_err()? {
            Some(val) => val,
            None => return TryErr(self.invalid_after_modifier()),
        };

        if self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            let mut methode_type = match self.try_parse_type(modifier) {
                Ok(val) => val,
                Err(TryError::IsNotValue(_)) => SoulType::none(self.token().span),
                Err(TryError::IsErr(err)) => return TryErr(err),
            };

            methode_type.modifier = modifier;
            return self
                .try_parse_function_declaration(start_span, methode_type, name)
                .map(Statement::from_function)
                .map_try_not_value(|(_, err)| *err)
                .merge_to_result()
                .try_err();
        }

        const DEFAULT_VARIABLE_MODIFIER: TypeModifier = TypeModifier::Const;
        let mut ty = None;
        if self.current_is(&COLON) {
            self.bump();
            ty = Some(self.try_parse_type(DEFAULT_VARIABLE_MODIFIER)?);
        }

        if self.current_is_any(STAMENT_END_TOKENS) {
            let span = self.token().span;
            let variable = Variable {
                name,
                ty,
                node_id: None,
                initialize_value: None,
            };
            return TryOk(Statement::new_variable(variable, span));
        }

        let assign = match &self.token().kind {
            TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                AssignType::from_symbool(*val).unwrap()
            }
            _ => return TryErr(self.invalid_assign()),
        };
        
        if assign != AssignType::Assign && assign != AssignType::Declaration {
            return TryErr(self.invalid_assign());
        }
        
        self.bump();
        let value = self.parse_expression(STAMENT_END_TOKENS).try_err()?;
        let variable = Variable {
            name,
            ty,
            node_id: None,
            initialize_value: Some(value),
        };

        TryOk(Statement::new_variable(
            variable,
            self.span_combine(start_span),
        ))
    }

    fn try_consume_name(&mut self) -> SoulResult<Option<Ident>> {
        let ident = match self.try_bump_consume_ident() {
            Ok(val) => val,
            Err(_) => return Ok(None),
        };

        let mut iligal_names = ILIGAL_NAMES.iter().copied().flatten();

        if iligal_names.any(|name| *name == ident.as_str()) {
            return Err(SoulError::new(
                format!("ident '{}', is not allowed as name", ident.as_str()),
                SoulErrorKind::InvalidIdent,
                Some(ident.span),
            ));
        }

        Ok(Some(ident))
    }

    fn invalid_after_modifier(&self) -> SoulError {
        SoulError::new(
            format!(
                "'{}' invalid after modifier (could be ['{{' or <name>])",
                self.token().kind.display()
            ),
            SoulErrorKind::InvalidTokenKind,
            Some(self.token().span),
        )
    }

    fn invalid_assign(&self) -> SoulError {
        SoulError::new(
            format!("'{}' should be '=' or ':='", self.token().kind.display(),),
            SoulErrorKind::InvalidContext,
            Some(self.token().span),
        )
    }
}
