use std::mem::discriminant;

use ast_model::{
    soul_type::{SoulType, Stub},
    statements::{Enum, EnumVariant, Field, Parameter, Statement, StatementKind, Struct},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{Ident, TypeModifier, collections::try_result::ToResult, error::SoulResult};

use crate::{
    parser::Parser,
    utils::{ASSIGN, COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN, STRUCT},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub fn parse_struct(&mut self) -> SoulResult<Statement> {
        let struct_span = self.current().span;
        self.expect(&STRUCT)?;
        let struct_name = self.try_bump_consume_ident()?;
        let generics = self.parse_generic_declare()?.unwrap_or(vec![]);

        self.expect(&CURLY_OPEN)?;
        self.skip_end_lines();

        let mut fields = vec![];
        let mut statements = vec![];
        let this_type = SoulType::Stub(Stub::new(struct_name.to_string()));

        let prev_type = self.current.this_type.take();
        self.current.this_type = Some(this_type);
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            let start_span = self.current().span;
            let statement = self.parse_statement()?;
            let is_public = statement.is_public();
            match statement.node {
                StatementKind::Variable(variable) => {
                    fields.push(Field::new(variable, is_public));
                }

                StatementKind::Enum(_)
                | StatementKind::Trait(_)
                | StatementKind::Import(_)
                | StatementKind::Struct(_)
                | StatementKind::TypeDef(_)
                | StatementKind::Function(_)
                | StatementKind::ExternalFunction(_) => {
                    let id = self.store.insert_statement(statement);
                    statements.push(id)
                }

                StatementKind::UseBlock(_)
                | StatementKind::Assignment(_)
                | StatementKind::Expression { .. } => {
                    self.log_error(
                        format!(
                            "{} can not be used in struct body",
                            statement.node.variant_name()
                        ),
                        Some(self.span_combine(start_span)),
                    );
                    continue;
                }
            }
        }
        self.current.this_type = prev_type;

        self.expect(&CURLY_CLOSE)?;

        let struct_ = Struct {
            id: None,
            fields,
            generics,
            statements,
            name: struct_name,
        };

        Ok(Statement::new(
            StatementKind::Struct(struct_),
            self.span_combine(struct_span),
        ))
    }

    pub(crate) fn parse_enum(&mut self) -> SoulResult<Statement> {
        let start_span = self.current().span;
        self.expect(&TokenKind::Keyword(KeyWord::Enum))?;
        let name = self.try_bump_consume_ident()?;

        let impl_type = if self.current_is(&COLON) {
            self.bump();
            Some(self.try_parse_type().merge_to_result()?)
        } else {
            None
        };

        let mut variants = vec![];
        self.expect(&CURLY_OPEN)?;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            let variant_span = self.current().span;
            let ident = self.try_bump_consume_ident()?;
            let variant = match &self.current().kind {
                &ROUND_OPEN => self.parse_enum_union(ident)?,
                &ASSIGN => self.parse_enum_assign(ident)?,
                _ => EnumVariant::Normal(ident),
            };

            if let Some(last) = variants.last() {
                if discriminant(last) != discriminant(&variant) {
                    self.log_error(
                        format!(
                            "enum type {} and {} are not compatible",
                            last.get_variant_name(),
                            variant.get_variant_name()
                        ),
                        Some(self.span_combine(variant_span)),
                    );

                    self.skip_till(&[CURLY_CLOSE]);
                    break;
                }
            }

            variants.push(variant);
            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }

            self.bump();
        }
        self.expect(&CURLY_CLOSE)?;

        let enum_ = Enum {
            id: None,
            name,
            variants,
            impl_type,
        };
        Ok(Statement::new(
            StatementKind::Enum(enum_),
            self.span_combine(start_span),
        ))
    }

    fn parse_enum_assign(&mut self, ident: Ident) -> SoulResult<EnumVariant> {
        self.expect(&ASSIGN)?;

        let value = self.parse_expression_id(&[COMMA, CURLY_CLOSE])?;
        Ok(EnumVariant::Assigned { name: ident, value })
    }

    fn parse_enum_union(&mut self, ident: Ident) -> SoulResult<EnumVariant> {
        let mut parameters = vec![];
        self.expect(&ROUND_OPEN)?;
        loop {
            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }

            let modifier = self
                .try_bump_type_modiffier()
                .unwrap_or(TypeModifier::Const);
            let name = self.try_bump_consume_ident()?;

            self.expect(&COLON)?;
            let ty = self.try_parse_type().merge_to_result()?;
            parameters.push(Parameter {
                ty,
                name,
                modifier,
                node_id: None,
                default: None,
            });

            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }

            self.bump();
        }
        self.expect(&ROUND_CLOSE)?;
        Ok(EnumVariant::Union {
            name: ident,
            parameters,
        })
    }
}
