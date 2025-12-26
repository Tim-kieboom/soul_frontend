use crate::{
    steps::{
        parse::{
            ARROW_LEFT, ASSIGN, COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, SEMI_COLON, SQUARE_CLOSE,
            SQUARE_OPEN, STAMENT_END_TOKENS, parser::Parser,
        },
        tokenize::token_stream::TokenKind,
    },
    utils::try_result::{ResultTryErr, TryErr, TryError, TryNotValue, TryOk, TryResult},
};
use models::{
    abstract_syntax_tree::{
        function::Function,
        objects::{
            Class, ClassMember, Field, FieldAccess, Struct, Trait, TraitSignature, Visibility,
        },
        soul_type::{SoulType, TypeKind},
        spanned::Spanned,
        statment::Ident,
    },
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{KeyWord, TypeModifier},
};

impl<'a> Parser<'a> {
    pub(crate) fn parse_class(&mut self) -> SoulResult<Class> {
        self.expect_ident(KeyWord::Class.as_str())?;

        let ident_token = self.bump_consume();
        let text = match ident_token.kind {
            TokenKind::Ident(val) => val,
            other => {
                return Err(SoulError::new(
                    format!("expected name got '{}'", other.display()),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.token().span),
                ));
            }
        };
        let ident = Ident::new(text, ident_token.span);

        let this_type = SoulType::new(
            None,
            TypeKind::Stub {
                ident: ident.clone(),
                resolved: None,
            },
            self.token().span,
        );

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_declare()?
        } else {
            vec![]
        };

        self.expect(&CURLY_OPEN)?;

        let mut members = vec![];

        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            match self.try_parse_methode(&this_type) {
                Ok(val) => {
                    members.push(Spanned::new(ClassMember::Method(val.node), val.span));
                    continue;
                }
                Err(TryError::IsNotValue(_)) => (),
                Err(TryError::IsErr(err)) => return Err(err),
            }

            match self.try_parse_field() {
                Ok(val) => {
                    members.push(Spanned::new(ClassMember::Field(val.node), val.span));
                    continue;
                }
                Err(TryError::IsErr(err)) | Err(TryError::IsNotValue(err)) => return Err(err),
            }
        }

        self.skip_end_lines();
        self.expect(&CURLY_CLOSE)?;

        let class = Class {
            members,
            name: ident,
            generics,
            node_id: None,
            scope_id: 0,
        };
        Ok(class)
    }

    pub(crate) fn parse_struct(&mut self) -> SoulResult<Struct> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::Struct.as_str())?;

        let ident_token = self.bump_consume();
        let text = match ident_token.kind {
            TokenKind::Ident(val) => val,
            other => {
                return Err(SoulError::new(
                    format!("expected name got '{}'", other.display()),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.new_span(start_span)),
                ));
            }
        };
        let name = Ident::new(text, ident_token.span);

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_declare()?
        } else {
            vec![]
        };

        self.expect(&CURLY_OPEN)?;

        let mut fields = vec![];
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            match self.try_parse_field() {
                Ok(val) => fields.push(val),
                Err(TryError::IsErr(err)) | Err(TryError::IsNotValue(err)) => return Err(err),
            }
        }
        self.expect(&CURLY_CLOSE)?;

        Ok(Struct {
            fields,
            name,
            generics,
            node_id: None,
            scope_id: 0
        })
    }

    pub(crate) fn parse_trait(&mut self) -> SoulResult<Trait> {
        let start_span = self.token().span;
        self.expect_ident(KeyWord::Trait.as_str())?;

        let ident_token = self.bump_consume();
        let text = match ident_token.kind {
            TokenKind::Ident(val) => val,
            other => {
                return Err(SoulError::new(
                    format!("expected name got '{}'", other.display()),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.new_span(start_span)),
                ));
            }
        };
        let name = Ident::new(text, ident_token.span);

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_declare()?
        } else {
            vec![]
        };

        let mut signature = TraitSignature {
            name: name.clone(),
            generics,
            implements: vec![],
            for_types: vec![],
        };

        let this_type = SoulType::new(
            None,
            TypeKind::Stub {
                ident: name,
                resolved: None,
            },
            self.token().span,
        );

        let mut methods = vec![];
        let (for_types, implements) = self.inner_parse_trait_impls()?;
        signature.implements = implements;
        signature.for_types = for_types;

        if self.current_is_any(STAMENT_END_TOKENS) {
            self.bump();
            return Ok(Trait {
                signature,
                methods,
                node_id: None,
            });
        }

        self.expect(&CURLY_OPEN)?;
        loop {
            self.skip_end_lines();

            let modifier = match &self.token().kind {
                TokenKind::Ident(name) if TypeModifier::from_str(name).is_some() => {
                    let modifier =
                        TypeModifier::from_str(name).expect("just checked should be Some");
                    self.bump();
                    Some(modifier)
                }
                &CURLY_CLOSE => break,
                _ => None,
            };

            let ident_token = self.bump_consume();
            let text = match ident_token.kind {
                TokenKind::Ident(val) => val,
                other => {
                    return Err(SoulError::new(
                        format!("expected name got '{}'", other.display()),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    ));
                }
            };
            let name = Ident::new(text, ident_token.span);

            let mut this = this_type.clone();
            this.modifier = modifier;
            let result = self.try_parse_function_signature(
                self.token().span,
                modifier.unwrap_or(TypeModifier::Mut),
                Some(this),
                name,
            );

            match result {
                Ok(val) => methods.push(val),
                Err(TryError::IsErr(err)) => return Err(err),
                Err(TryError::IsNotValue(_)) => break,
            }
        }
        self.skip_end_lines();
        self.expect(&CURLY_CLOSE)?;

        Ok(Trait {
            signature,
            methods,
            node_id: None,
        })
    }

    fn inner_parse_trait_impls(&mut self) -> Result<(Vec<SoulType>, Vec<SoulType>), SoulError> {
        let mut for_types = vec![];
        let mut impl_traits = vec![];

        const IMPL: &str = KeyWord::Impl.as_str();
        const TYPEOF: &str = KeyWord::Typeof.as_str();

        if self.current_is_ident(TYPEOF) {
            self.bump();
            self.expect(&SQUARE_OPEN)?;
            loop {
                match self.try_parse_type() {
                    Ok(val) => for_types.push(val),
                    Err(TryError::IsErr(err)) | Err(TryError::IsNotValue(err)) => return Err(err),
                }

                if !self.current_is(&COMMA) {
                    break;
                }
                self.bump();
            }
            self.expect(&SQUARE_CLOSE)?;
        }

        if self.current_is_ident(IMPL) {
            loop {
                match self.try_parse_type() {
                    Ok(val) => impl_traits.push(val),
                    Err(TryError::IsErr(err)) | Err(TryError::IsNotValue(err)) => return Err(err),
                }

                if !self.current_is(&COMMA) {
                    break;
                }
                self.bump();
            }

            if self.current_is_ident(IMPL) {
                return Err(SoulError::new(
                    format!("can not have {IMPL} after {TYPEOF} in trait"),
                    SoulErrorKind::InvalidContext,
                    Some(self.token().span),
                ));
            }
        }

        Ok((for_types, impl_traits))
    }

    fn try_parse_methode(&mut self, this_type: &SoulType) -> TryResult<Spanned<Function>, ()> {
        let begin_position = self.current_position();
        let result = self.inner_parse_methode(this_type);
        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    fn inner_parse_methode(&mut self, this_type: &SoulType) -> TryResult<Spanned<Function>, ()> {
        let start_span = self.token().span;
        let modifier = match &self.token().kind {
            TokenKind::Ident(name) if TypeModifier::from_str(name).is_some() => {
                let modifier = TypeModifier::from_str(name).expect("just checked should be Some");
                self.bump();
                Some(modifier)
            }
            _ => None,
        };

        let ident_token = self.bump_consume();
        let text = match ident_token.kind {
            TokenKind::Ident(val) => val,
            _ => return TryNotValue(()),
        };
        let name = Ident::new(text, ident_token.span);

        let mut this_type = this_type.clone();
        this_type.modifier = modifier;
        let result = self.try_parse_function_signature(
            self.token().span,
            modifier.unwrap_or(TypeModifier::Mut),
            Some(this_type),
            name,
        );

        let signature = match result {
            Ok(val) => val,
            Err(TryError::IsErr(err)) => return TryErr(err),
            Err(TryError::IsNotValue(_)) => return TryNotValue(()),
        };

        let block = self
            .parse_block(modifier.unwrap_or(TypeModifier::Mut))
            .try_err()?;

        TryOk(Spanned::new(
            Function {
                signature,
                block,
                node_id: None,
            },
            self.new_span(start_span),
        ))
    }

    fn try_parse_field(&mut self) -> TryResult<Spanned<Field>, SoulError> {
        let begin_position = self.current_position();
        let result = self.inner_parse_field();
        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    fn inner_parse_field(&mut self) -> TryResult<Spanned<Field>, SoulError> {
        let start_span = self.token().span;

        self.skip_end_lines();

        let possible_modifier = match &self.token().kind {
            TokenKind::Ident(ident) => TypeModifier::from_str(ident),
            _ => None,
        };

        let modifier = match possible_modifier {
            Some(val) => {
                self.bump();
                val
            }
            None => TypeModifier::Const,
        };

        let ident_token = self.bump_consume();
        let text = match ident_token.kind {
            TokenKind::Ident(val) => val,
            other => {
                return TryNotValue(SoulError::new(
                    format!("expected ident but got '{}'", other.display()),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.token().span),
                ));
            }
        };
        let name = Ident::new(text, ident_token.span);

        let mut ty = if self.current_is(&COLON) {
            self.bump();
            self.try_parse_type()?
        } else {
            SoulType::none(self.token().span)
        };

        ty.modifier = Some(modifier);

        let vis = self.parse_field_access();

        if self.current_is_any(STAMENT_END_TOKENS) {
            return TryOk(Spanned::new(
                Field::new(ty, name),
                self.new_span(start_span),
            ));
        }

        self.expect(&ASSIGN).try_err()?;

        let default_value = Some(
            self.parse_expression(&[CURLY_OPEN, SEMI_COLON, TokenKind::EndLine])
                .try_err()?,
        );

        if self.current_is(&SEMI_COLON) {
            self.bump();
        }

        TryOk(Spanned::new(
            Field {
                ty,
                vis,
                name,
                default_value,
                node_id: None,
                allignment: u32::default(),
            },
            self.new_span(start_span),
        ))
    }

    fn parse_field_access(&mut self) -> FieldAccess {
        let mut access = FieldAccess::default();
        loop {
            match self.token().kind.try_as_ident() {
                Some(FieldAccess::PUBLIC_GET) => access.get = Some(Visibility::Public),
                Some(FieldAccess::PUBLIC_SET) => access.set = Some(Visibility::Public),
                Some(FieldAccess::PRIVATE_GET) => access.get = Some(Visibility::Private),
                Some(FieldAccess::PRIVATE_SET) => access.set = Some(Visibility::Private),
                _ => break,
            }

            self.bump();
        }

        access
    }
}
