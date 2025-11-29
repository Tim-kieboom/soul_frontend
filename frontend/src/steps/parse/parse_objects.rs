use itertools::Itertools;
use models::{abstract_syntax_tree::{objects::{Field, FieldAccess, Struct, Visibility}, spanned::Spanned}, error::{SoulError, SoulErrorKind, SoulResult}, soul_names::KeyWord};
use crate::{steps::{parse::{parse_statement::{ASSIGN, COLON, CURLY_CLOSE, CURLY_OPEN, SEMI_COLON, STAMENT_END_TOKENS}, parser::Parser}, tokenize::token_stream::TokenKind}, utils::try_result::{MapNotValue, ResultTryResult, TryError, TryNotValue, TryOk, TryResult}};

impl<'a> Parser<'a> {
    
    pub(crate) fn parse_struct(&mut self) -> SoulResult<Struct> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::Struct.as_str())?;

        let ident_token = self.bump_consume(); 
        let ident = match ident_token.kind {
            TokenKind::Ident(val) => val,
            other => return Err(
                SoulError::new(
                    format!("expected name got '{}'", other.display()),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.new_span(start_span))
                )
            ),
        };

        self.expect(&CURLY_OPEN)?;
        let scope_id = self.push_scope();

        let mut fields = vec![];
        loop {

            match self.parse_field() {
                Ok(val) => fields.push(val),
                Err(TryError::IsNotValue(())) => break,
                Err(TryError::IsErr(err)) => return Err(err),
            }
        }
        self.pop_scope();
        self.skip_end_lines();
        self.expect(&CURLY_CLOSE)?;

        Ok(Struct{
            fields, 
            scope_id, 
            name: ident, 
            generics: vec![], 
        })
    }

    fn parse_field(&mut self) -> TryResult<Spanned<Field>, ()> {
        let begin_position = self.current_position();
        let result = self.inner_parse_field();
        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    fn inner_parse_field(&mut self) -> TryResult<Spanned<Field>, ()> {
        let start_span = self.token().span;

        let ident_token = self.bump_consume(); 
        let name = match ident_token.kind {
            TokenKind::Ident(val) => val,
            _ => return TryNotValue(()),
        };

        if !self.current_is(&COLON) {
            return TryNotValue(())
        }

        self.bump();
        let ty = self.try_parse_type()
            .map_not_value(|_| ())?;

        if self.current_is_any(STAMENT_END_TOKENS) {

            return TryOk(Spanned::new(
                Field{name, ty, default_value: None, vis: FieldAccess::default()},
                self.new_span(start_span)
            ))
        }

        self.expect(&ASSIGN)
            .try_err()?;

        let default_value = Some(
            self.parse_expression(&[CURLY_OPEN, SEMI_COLON, TokenKind::EndLine])
                .try_err()?
        );

        if self.current_is(&SEMI_COLON) {
            self.bump();
        }
        self.skip_end_lines();
        
        if !self.current_is(&CURLY_OPEN) {
            
            return TryOk(Spanned::new(
                Field{name, ty, default_value, vis: FieldAccess::default()},
                self.new_span(start_span)
            ))
        }

        let vis = self.parse_field_access()
            .try_err()?;

        return TryOk(Spanned::new(
            Field{name, ty, default_value, vis},
            self.new_span(start_span)
        ))
    }

    fn parse_field_access(&mut self) -> SoulResult<FieldAccess> {
        const ALL_ACCESS_IDENTS: &[&str] = &[FieldAccess::PUBLIC_GET, FieldAccess::PRIVATE_GET, FieldAccess::PUBLIC_SET, FieldAccess::PRIVATE_SET];
        let start_span = self.token().span;

        self.expect(&CURLY_OPEN)?;
        let mut access = FieldAccess::default();
        
        loop {

            self.skip_end_lines();
            let ident = match &self.token().kind{
                TokenKind::Ident(val) => val,
                &CURLY_CLOSE => break,
                other => return Err(
                    SoulError::new(
                        format!("expeced ident got '{}'", other.display()),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.new_span(start_span)),
                    )
                ),
            };


            match ident.as_str() {
                FieldAccess::PUBLIC_GET => access.get = Some(Visibility::Public),
                FieldAccess::PRIVATE_GET => access.get = Some(Visibility::Private),
                FieldAccess::PUBLIC_SET => access.set = Some(Visibility::Public),
                FieldAccess::PRIVATE_SET => access.set = Some(Visibility::Private),
                other => return Err(
                    SoulError::new(
                        format!("expected on of ['{}'] got '{}'", ALL_ACCESS_IDENTS.iter().join("', '"), other),
                        SoulErrorKind::InvalidIdent,
                        Some(self.new_span(start_span)),
                    )
                ),
            }

            self.expect_any(&[SEMI_COLON, TokenKind::EndLine])?;
        }

        self.expect(&CURLY_CLOSE)?;
        Ok(
            access
        )
    }
}