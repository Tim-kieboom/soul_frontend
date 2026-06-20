use std::sync::LazyLock;

use ast_model::{
    soul_type::SoulType,
    statements::{Statement, Variable},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord, types::Types};
use soul_utils::{
    Ident, TypeModifier,
    collections::try_result::{ResultMapNotValue, ResultTryErr, TryErr, TryOk, TryResult},
    error::SoulResult,
    fault::Fault,
    span::Span,
};

use crate::{
    parse::statements::variable::AssignType,
    parser::Parser,
    utils::{ARROW_LEFT, COLON, CURLY_OPEN, ROUND_OPEN, SEMI_COLON, STAMENT_END_TOKENS},
};

const RAW_ILIGAL_NAMES: &[&[&str]] = &[
    Types::STRING_VALUES,
    KeyWord::STRING_VALUES,
    TypeModifier::STRING_VALUES,
];

const ILIGAL_NAMES: LazyLock<Vec<&str>> = LazyLock::new(|| {
    let len = RAW_ILIGAL_NAMES.iter().map(|slice| slice.len()).sum();
    let mut vec = Vec::with_capacity(len);
    for names in RAW_ILIGAL_NAMES {
        vec.extend(*names);
    }
    vec
});

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn try_parse_from_modifier(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
    ) -> TryResult<Statement, Fault> {
        self.bump();

        if self.current_is(&CURLY_OPEN) {
            let block = self.parse_block(modifier).try_err()?;
            return TryOk(Statement::new_block(
                self.store,
                block,
                self.span_combine(start_span),
                self.current_is(&SEMI_COLON),
            ));
        }

        let name = match self.try_consume_name().try_err()? {
            Some(val) => val,
            None => return TryErr(self.invalid_after_modifier()),
        };

        if self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return self
                .try_parse_function_declaration_id(start_span, modifier, &SoulType::None, name)
                .map(Statement::from_function)
                .map_try_not_value(|(_, err)| err);
        }

        let mut ty = None;
        if self.current_is(&COLON) {
            self.bump();
            ty = Some(self.try_parse_type()?);
        }

        if self.current_is_any(STAMENT_END_TOKENS) {
            let span = self.token().span;
            let variable = Variable {
                ty,
                name,
                modifier,
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
        let value = self.parse_expression_id(STAMENT_END_TOKENS).try_err()?;
        let variable = Variable {
            name,
            ty,
            modifier,
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

        if ILIGAL_NAMES.iter().any(|name| *name == ident.as_str()) {
            return Err(Fault::error(
                format!("ident '{}', is not allowed as name", ident.as_str()),
                Some(ident.span()),
            ));
        }

        Ok(Some(ident))
    }

    fn invalid_after_modifier(&self) -> Fault {
        Fault::error(
            format!(
                "'{}' invalid after modifier (could be ['{{' or <name>])",
                self.token().kind.display(),
            ),
            Some(self.token().span),
        )
    }

    fn invalid_assign(&self) -> Fault {
        Fault::error(
            format!("'{}' should be '=' or ':='", self.token().kind.display(),),
            Some(self.token().span),
        )
    }
}
