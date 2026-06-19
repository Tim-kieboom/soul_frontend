use ast_model::{
    soul_type::SoulType,
    statements::{ImplBlock, Methode, Statement, StatementKind, UseBlock},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    TypeModifier,
    collections::try_result::{ResultMapNotValue, ToResult},
    error::SoulResult,
    span::Span,
};

use crate::{
    parser::Parser,
    utils::{CONST, CURLY_CLOSE, CURLY_OPEN, LITERAL, MUT, PUB},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_use_block(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;
        self.expect(&TokenKind::Keyword(KeyWord::Use))?;
        let use_generics = self.parse_generic_declare()?.unwrap_or(vec![]);

        let method_type = self.try_parse_type().merge_to_result()?;
        let prev = self.current.this_type.take();

        let mut impls = vec![];
        let mut methodes = vec![];
        let mut statements = vec![];
        self.current.this_type = Some(method_type.clone());
        match &self.token().kind {
            TokenKind::Keyword(KeyWord::Impl) => {
                let impl_block = self.parse_impl_block(&method_type, self.token().span)?;
                impls.push(impl_block);
            }

            &PUB | &MUT | &CONST | &LITERAL | TokenKind::Ident(_) => {
                let methode = self.parse_use_method(&method_type, self.token().span)?;
                methodes.push(methode);
            }
            _ => (),
        }

        if !impls.is_empty() || !methodes.is_empty() {
            self.current.this_type = prev;
            let use_block = UseBlock {
                ty: method_type,
                impls,
                methodes,
                statements,
                use_generics,
            };

            return Ok(Statement::new(
                StatementKind::UseBlock(use_block),
                self.span_combine(start_span),
            ));
        }

        self.expect(&CURLY_OPEN)?;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }
            let start_span = self.token().span;

            if self.current_is(&TokenKind::Keyword(KeyWord::Impl)) {
                let impl_block = self.parse_impl_block(&method_type, start_span)?;
                impls.push(impl_block);
                continue;
            }

            let statement = self.parse_statement()?;
            let is_public = statement.is_public();
            match statement.node {
                StatementKind::Variable(_) => {
                    self.log_error(
                        "Variable is not allowed in use block",
                        Some(self.span_combine(start_span)),
                    );
                    continue;
                }

                StatementKind::Enum(_)
                | StatementKind::Trait(_)
                | StatementKind::Struct(_)
                | StatementKind::Import(_)
                | StatementKind::TypeDef(_)
                | StatementKind::ExternalFunction(_) => {
                    let id = self.store.insert_statement(statement);
                    statements.push(id)
                }

                StatementKind::Function(function) => {
                    methodes.push(Methode::new(function, is_public));
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

        self.expect(&CURLY_CLOSE)?;
        self.current.this_type = prev;
        let use_block = UseBlock {
            ty: method_type,
            impls,
            methodes,
            statements,
            use_generics,
        };

        Ok(Statement::new(
            StatementKind::UseBlock(use_block),
            self.span_combine(start_span),
        ))
    }

    fn parse_impl_block(
        &mut self,
        method_type: &SoulType,
        start_span: Span,
    ) -> SoulResult<ImplBlock> {
        self.expect(&TokenKind::Keyword(KeyWord::Impl))?;
        let impl_trait = self.try_parse_type().merge_to_result()?;

        self.expect(&CURLY_OPEN)?;
        let mut methods = vec![];
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            let modifier = self.try_bump_type_modiffier().unwrap_or(TypeModifier::Mut);
            let name = self.try_bump_consume_ident()?;
            methods.push(
                self.try_parse_function_declaration_id(start_span, modifier, method_type, name)
                    .map_try_not_value(|(_, err)| err)
                    .merge_to_result()?
                    .value,
            );
        }
        self.expect(&CURLY_CLOSE)?;

        Ok(ImplBlock {
            impl_trait,
            methods,
        })
    }

    fn parse_use_method(&mut self, ty: &SoulType, start_span: Span) -> SoulResult<Methode> {
        let is_public = self.current_is(&PUB);
        if is_public {
            self.bump();
        }

        let modifier = self.try_bump_type_modiffier().unwrap_or(TypeModifier::Mut);
        let name = self.try_bump_consume_ident()?;
        self.try_parse_function_declaration_id(start_span, modifier, ty, name)
            .map(|spanned| Methode::new(spanned.value, is_public))
            .map_try_not_value(|(_, err)| err)
            .merge_to_result()
    }
}
