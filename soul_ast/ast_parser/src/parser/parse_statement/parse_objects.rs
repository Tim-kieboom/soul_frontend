use ast::{
    Block, Enum, Field, Function, Statement, Struct, Trait, Union, UnionVariant,
};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{KeyWord, TypeModifier},
    try_result::{ResultMapNotValue, ResultTryErr, ResultTryNotValue, ToResult, TryErr, TryOk, TryResult},
};

use crate::parser::{
    Parser,
    parse_utils::{
        COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN, STAMENT_END_TOKENS,
    },
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

            variant.push(self.try_bump_consume_ident()?);

            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }
            self.bump();
        }
        self.skip_end_lines();
        if !self.current_is(&CURLY_CLOSE) {
            return Err(SoulError::new(
                format!(
                    "expected: '{}' or '{}' but found: '{}'",
                    CURLY_CLOSE.display(),
                    COMMA.display(),
                    self.token().kind.display()
                ),
                soul_utils::error::SoulErrorKind::InvalidTokenKind,
                Some(self.token().span),
            ));
        }

        self.bump();
        Ok(Statement::new(
            ast::StatementKind::Enum(Enum {
                id: None,
                name,
                variants: variant,
            }),
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

    pub(crate) fn parse_union(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::Union.as_str())?;
        let name = self.try_bump_consume_ident()?;
        let generics = self.parse_generic_declare()?.unwrap_or(vec![]);
        self.skip_end_lines();

        self.expect(&ROUND_OPEN)?;
        let mut variants = vec![];
        loop {
            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }
            let variant_name = self.try_bump_consume_ident()?;
            self.expect(&ROUND_OPEN)?;
            let mut ty = self.try_parse_type().merge_to_result()?;
            ty.modifier = Some(TypeModifier::Const);
            self.expect(&ROUND_CLOSE)?;
            variants.push(UnionVariant {
                id: None,
                name: variant_name,
                ty,
            });

            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }
            self.bump();
        }
        self.skip_end_lines();
        self.expect(&ROUND_CLOSE)?;

        let obj = Union {
            id: None,
            name,
            generics,
            variants,
            defined_in: None,
        };

        Ok(Statement::new(
            ast::StatementKind::Union(obj),
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn parse_trait(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::Trait.as_str())?;
        let name = self.try_bump_consume_ident()?;
        let generics = self.parse_generic_declare()?.unwrap_or(vec![]);
        self.skip_end_lines();

        let mut methods = vec![];
        self.expect(&CURLY_OPEN)?;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            let mut method_name = self.try_bump_consume_ident()?;
            let modifier = match TypeModifier::from_str(method_name.as_str()) {
                Some(modifier) => {
                    method_name = self.try_bump_consume_ident()?;
                    modifier
                }
                None => TypeModifier::Mut,
            };

            let signature = self
                .try_parse_function_signature(
                    method_name.span,
                    self.default_methode_type(modifier, method_name.span),
                    method_name,
                    None,
                )
                .map_try_not_value(|(_, err)| *err)
                .merge_to_result()?;

            methods.push(Function {
                signature,
                block: Block {
                    modifier: TypeModifier::Mut,
                    statements: vec![],
                    scope_id: None,
                    node_id: None,
                    span: start_span,
                },
            });
        }
        self.expect(&CURLY_CLOSE)?;

        Ok(Statement::new(
            ast::StatementKind::Trait(Trait {
                id: None,
                name,
                generics,
                methods,
                defined_in: None,
            }),
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn parse_standalone_impl(&mut self) -> SoulResult<Statement> {
        Err(SoulError::new(
            "'impl' must be inside a 'use' block. Use 'use Type impl Trait { ... }' instead.".to_string(),
            SoulErrorKind::InvalidContext,
            Some(self.token().span),
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
