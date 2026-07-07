use std::sync::LazyLock;

use ast_model::{
    expression::Binding,
    soul_type::SoulType,
    statements::{Statement, VarPattern, Variable},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord, types::Types};
use soul_utils::{
    Ident, TypeModifier,
    collections::try_result::{
        ResultMapNotValue, ResultTryErr, TryErr, TryError, TryNotValue, TryOk, TryResult,
    },
    error::SoulResult,
    fault::Fault,
    span::Span,
};

use crate::{
    parse::statements::{try_assign_type, variable::AssignType},
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

        if self.current_is(&ROUND_OPEN) {
            if modifier != TypeModifier::Const {
                return TryErr(Fault::error(
                    "'mut' cannot be applied to tuple patterns; use per-element 'mut' instead (e.g., (mut a, b))".to_string(),
                    Some(start_span),
                ));
            }
            let pattern = self.parse_tuple_pattern().try_err()?;
            return self
                .parse_pattern_declaration(pattern, modifier, start_span)
                .try_err();
        }

        if self.current_is(&CURLY_OPEN) {
            return self.try_parse_named_tuple_or_block(modifier, start_span);
        }

        let name = match self.try_consume_name().try_err()? {
            Some(val) => val,
            None => return TryErr(self.invalid_after_modifier()),
        };

        if self.current_is(&CURLY_OPEN) {
            if modifier != TypeModifier::Const {
                return TryErr(Fault::error(
                    "'mut' cannot be applied to constructor patterns; use per-field 'mut' instead"
                        .to_string(),
                    Some(start_span),
                ));
            }
            return self
                .try_parse_constructor_declaration(name, modifier, start_span)
                .try_err();
        }

        if self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return self
                .try_parse_function_declaration_id(start_span, modifier, &SoulType::None, name)
                .map(Statement::from_function)
                .map_try_not_value(|(_, err)| err);
        }

        let pattern = if name.as_str() == "_" {
            VarPattern::Discard
        } else {
            VarPattern::Simple {
                binding: Binding::new(self.alloc_node(), name),
                modifier,
            }
        };

        let mut ty = None;
        if self.current_is(&COLON) {
            self.bump();
            ty = Some(self.try_parse_type()?);
        }

        if self.current_is_any(STAMENT_END_TOKENS) {
            let span = self.token().span;
            let variable = Variable {
                id: self.alloc_node(),
                is_public: false,
                pattern,
                ty,
                modifier,
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
            id: self.alloc_node(),
            is_public: false,
            pattern,
            ty,
            modifier,
            initialize_value: Some(value),
        };

        TryOk(Statement::new_variable(
            variable,
            self.span_combine(start_span),
        ))
    }

    /// Try named-tuple destructuring `{field1, field2} = expr` or fall back to block.
    fn try_parse_named_tuple_or_block(
        &mut self,
        modifier: TypeModifier,
        start_span: Span,
    ) -> TryResult<Statement, Fault> {
        let saved = self.tokens.current_position();

        match self.try_parse_named_tuple(start_span, modifier) {
            Ok(val) => return TryOk(val),
            Err(TryError::IsErr(err)) => return TryErr(err),
            Err(TryError::IsNotValue(())) => (),
        }

        self.goto(saved);
        let block = self.parse_block(modifier).try_err()?;
        TryOk(Statement::new_block(
            self.store,
            block,
            self.span_combine(start_span),
            self.current_is(&SEMI_COLON),
        ))
    }

    fn try_parse_named_tuple(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
    ) -> TryResult<Statement, ()> {
        if modifier != TypeModifier::Const {
            return TryNotValue(());
        }

        let Ok(pattern) = self.parse_named_tuple_pattern() else {
            return TryNotValue(());
        };

        let Some(assign) = try_assign_type(&self.token()) else {
            return TryNotValue(());
        };

        if assign == AssignType::Assign || assign == AssignType::Declaration {
            self.bump();
            return self
                .parse_expression_id(STAMENT_END_TOKENS)
                .map(|value| {
                    Statement::new_variable(
                        Variable {
                            id: self.alloc_node(),
                            is_public: false,
                            pattern,
                            ty: None,
                            modifier,
                            initialize_value: Some(value),
                        },
                        self.span_combine(start_span),
                    )
                })
                .try_err();
        }

        return TryNotValue(());
    }

    /// Try constructor destructuring: `TypeName{field1, field2} = expr`.
    fn try_parse_constructor_declaration(
        &mut self,
        type_name: Ident,
        modifier: TypeModifier,
        start_span: Span,
    ) -> SoulResult<Statement> {
        let pattern = self.parse_constructor_pattern(type_name)?;
        let assign = match &self.token().kind {
            TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                AssignType::from_symbool(*val).unwrap()
            }
            _ => {
                return Err(Fault::error(
                    "expected '=' or ':=' after constructor pattern",
                    Some(self.token().span),
                ));
            }
        };

        if assign != AssignType::Assign && assign != AssignType::Declaration {
            return Err(Fault::error(
                format!(
                    "'{}' is not valid for variable declaration (can use ['=', ':='])",
                    assign.as_str()
                ),
                Some(self.token().span),
            ));
        }

        self.bump();
        let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
        Ok(Statement::new_variable(
            Variable {
                id: self.alloc_node(),
                is_public: false,
                pattern,
                ty: None,
                modifier,
                initialize_value: Some(value),
            },
            self.span_combine(start_span),
        ))
    }

    /// Parse a declaration with the given pattern-fn, checking for = or :=.
    fn parse_pattern_declaration(
        &mut self,
        pattern: VarPattern,
        modifier: TypeModifier,
        start_span: Span,
    ) -> SoulResult<Statement> {
        let assign = match &self.token().kind {
            TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                AssignType::from_symbool(*val).unwrap()
            }
            _ => {
                return Err(Fault::error(
                    "expected '=' or ':=' after destructuring pattern",
                    Some(self.token().span),
                ));
            }
        };

        if assign != AssignType::Assign && assign != AssignType::Declaration {
            return Err(Fault::error(
                format!(
                    "'{}' is not valid for variable declaration (can use ['=', ':='])",
                    assign.as_str()
                ),
                Some(self.token().span),
            ));
        }

        self.bump();
        let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
        Ok(Statement::new_variable(
            Variable {
                id: self.alloc_node(),
                is_public: false,
                pattern,
                ty: None,
                modifier,
                initialize_value: Some(value),
            },
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
                "'{}' invalid after modifier (could be ['{{', '(', or <name>])",
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
