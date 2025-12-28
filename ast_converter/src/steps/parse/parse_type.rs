use crate::{
    steps::{
        parse::{
            COLON, COMMA, CONST_REF, CURLY_CLOSE, CURLY_OPEN, MUT_REF, ROUND_CLOSE, ROUND_OPEN,
            SQUARE_CLOSE, SQUARE_OPEN, parser::Parser,
        },
        tokenize::token_stream::{Number, TokenKind},
    },
    utils::try_result::{
        ResultTryErr, ResultTryNotValue, TryErr, TryError, TryNotValue, TryOk, TryResult,
    },
};
use soul_ast::{
    abstract_syntax_tree::{
        function::ThisCallee,
        soul_type::{ArrayType, NamedTupleType, ReferenceType, SoulType, TupleType, TypeKind},
        spanned::Spanned,
        statment::Ident,
    },
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{
        InternalComplexTypes, InternalPrimitiveTypes, StackArrayKind, TypeModifier, TypeWrapper,
    },
};

impl<'a> Parser<'a> {
    pub(crate) fn try_parse_type(&mut self) -> TryResult<SoulType, SoulError> {
        let begin_position = self.current_position();
        let result = self.inner_parse_type();
        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    pub(crate) fn parse_tuple_type(&mut self) -> SoulResult<TupleType> {
        self.expect(&ROUND_OPEN)?;

        if self.current_is(&ROUND_CLOSE) {
            self.bump();
            return Ok(TupleType { types: vec![] });
        }

        let mut types = vec![];

        loop {
            let ty = match self.try_parse_type() {
                Ok(val) => val,
                Err(TryError::IsErr(err)) | Err(TryError::IsNotValue(err)) => return Err(err),
            };
            types.push(ty);

            if self.current_is(&ROUND_CLOSE) {
                break;
            }

            self.expect(&COMMA)?;
        }

        self.expect(&ROUND_CLOSE)?;

        Ok(TupleType { types })
    }

    pub(crate) fn parse_named_tuple_type(
        &mut self,
        is_function: bool,
    ) -> TryResult<(NamedTupleType, ThisCallee), SoulError> {
        let begin_position = self.current_position();
        let result = self.inner_parse_named_tuple_type(is_function);
        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    fn inner_parse_type(&mut self) -> TryResult<SoulType, SoulError> {
        let modifier = match &self.token().kind {
            TokenKind::Ident(ident) => {
                let modifier = TypeModifier::from_str(ident);
                if modifier.is_some() {
                    self.bump();
                }
                modifier
            }
            _ => None,
        };

        let wrapper = self.get_type_wrappers()?;
        let mut ty = self.inner_get_base_type()?;

        for (wrap, lifetime, size) in wrapper {
            let span = ty.span;

            ty = match wrap {
                TypeWrapper::ConstRef => SoulType::new(
                    None,
                    TypeKind::Reference(ReferenceType {
                        inner: Box::new(ty),
                        lifetime,
                        mutable: false,
                    }),
                    self.token().span.combine(span),
                ),
                TypeWrapper::MutRef => SoulType::new(
                    None,
                    TypeKind::Reference(ReferenceType {
                        inner: Box::new(ty),
                        lifetime,
                        mutable: true,
                    }),
                    self.token().span.combine(span),
                ),
                TypeWrapper::Pointer => SoulType::new(
                    None,
                    TypeKind::Pointer(Box::new(ty)),
                    self.token().span.combine(span),
                ),
                TypeWrapper::Array => SoulType::new(
                    None,
                    TypeKind::Array(ArrayType {
                        of_type: Box::new(ty),
                        size,
                    }),
                    self.token().span.combine(span),
                ),
                TypeWrapper::Option => SoulType::new(
                    None,
                    TypeKind::Optional(Box::new(ty)),
                    self.token().span.combine(span),
                ),
            };
        }

        ty.modifier = modifier;
        Ok(ty)
    }

    fn inner_get_base_type(&mut self) -> TryResult<SoulType, SoulError> {
        const NONE_STR: &str = InternalPrimitiveTypes::None.as_str();

        match &self.token().kind {
            TokenKind::Ident(val) if val == NONE_STR => {
                return TryOk(SoulType::none(self.token().span));
            }
            &ROUND_OPEN => {
                return self
                    .parse_tuple_type()
                    .map(|el| SoulType::new(None, TypeKind::Tuple(el), self.token().span))
                    .try_err();
            }
            &CURLY_OPEN => {
                const IS_NAMED_TUPLE: bool = false;
                return self
                    .parse_named_tuple_type(IS_NAMED_TUPLE)
                    .map(|(tuple, _this)| {
                        SoulType::new(None, TypeKind::NamedTuple(tuple), self.token().span)
                    });
            }
            _ => (),
        }

        let token = self.bump_consume();
        let name = match token.kind {
            TokenKind::Ident(val) => val,
            other => {
                return TryNotValue(SoulError::new(
                    format!("expected ident got '{}'", other.display()),
                    SoulErrorKind::UnexpecedToken,
                    Some(self.token().span),
                ));
            }
        };

        if let Some(prim) = InternalPrimitiveTypes::from_str(&name) {
            return TryOk(SoulType::new(None, TypeKind::Primitive(prim), token.span));
        }

        if let Some(prim) = InternalComplexTypes::from_str(&name) {
            return TryOk(SoulType::new(
                None,
                TypeKind::InternalComplex(prim),
                token.span,
            ));
        }

        TryOk(SoulType::new(
            None,
            TypeKind::Stub {
                ident: Ident::new(name, token.span),
                resolved: None,
            },
            token.span,
        ))
    }

    fn inner_parse_named_tuple_type(
        &mut self,
        is_function: bool,
    ) -> TryResult<(NamedTupleType, ThisCallee), SoulError> {
        let open = if is_function {
            &ROUND_OPEN
        } else {
            &CURLY_OPEN
        };
        let close = if is_function {
            &ROUND_CLOSE
        } else {
            &CURLY_CLOSE
        };

        self.expect(open).try_err()?;

        if self.current_is(close) {
            self.bump();
            return Ok((NamedTupleType { types: vec![] }, ThisCallee::Static));
        }

        let mut this = ThisCallee::Static;
        let mut types = vec![];
        loop {
            let possible_this = match &self.token().kind {
                &CONST_REF => {
                    self.bump();
                    Some(ThisCallee::ConstRef)
                }
                &MUT_REF => {
                    self.bump();
                    Some(ThisCallee::MutRef)
                }
                TokenKind::Ident(val) if val == "this" => Some(ThisCallee::Consume),
                _ => None,
            };

            if let Some(callee) = possible_this {
                if this != ThisCallee::Static {
                    return TryErr(SoulError::new(
                        "can not have more then one 'this' in methode".to_string(),
                        SoulErrorKind::InvalidContext,
                        Some(self.token().span),
                    ));
                }

                this = callee;
                self.expect_ident("this").try_err()?;

                if self.current_is(&COMMA) {
                    self.bump();
                    continue;
                } else if self.current_is(&ROUND_CLOSE) {
                    break;
                } else {
                    return TryErr(self.get_expect_any_error(&[COMMA, ROUND_CLOSE]));
                }
            }

            let token = self.bump_consume();
            let name = match token.kind {
                TokenKind::Ident(val) => Spanned::new(val, token.span),
                other => {
                    return Err(TryError::IsNotValue(SoulError::new(
                        format!("'{}' should be ident", other.display()),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    )));
                }
            };

            if !self.current_is(&COLON) {
                // is probebly tuple
                return Err(TryError::IsNotValue(self.get_expect_error(&COLON)));
            }

            self.bump();
            let ty = self.try_parse_type()?; // is probebly named_tuple expression 

            types.push((name, ty, None));
            if self.current_is(close) {
                break;
            }

            self.expect(&COMMA).try_err()?;
        }

        self.expect(close).try_err()?;

        Ok((NamedTupleType { types }, this))
    }

    fn get_type_wrappers(
        &mut self,
    ) -> TryResult<Vec<(TypeWrapper, Ident, Option<StackArrayKind>)>, SoulError> {
        let mut wrapper = vec![];
        loop {
            let mut size = None;

            let possible_wrap = match &self.token().kind {
                &SQUARE_OPEN => {
                    self.bump();
                    size = self.inner_get_stack_modifier().try_not_value()?;

                    Some(TypeWrapper::Array)
                }
                TokenKind::Symbool(sym) => TypeWrapper::from_symbool(*sym),
                _ => None,
            };

            let wrap = match possible_wrap {
                Some(val) => val,
                None => break TryOk(wrapper),
            };

            if size.is_none() {
                self.bump();
            }
            wrapper.push((wrap, Ident::new(String::default(), self.token().span), size));
        }
    }

    fn inner_get_stack_modifier(&mut self) -> SoulResult<Option<StackArrayKind>> {
        let token = self.bump_consume();
        let size = match token.kind {
            TokenKind::Ident(generic_type) => Some(StackArrayKind::Ident {
                ident: Ident::new(generic_type, token.span),
                resolved: None,
            }),
            TokenKind::Number(Number::Uint(number)) => Some(StackArrayKind::Number(number)),
            other => {
                return Err(SoulError::new(
                    format!(
                        "expected ident or literal uint but got '{}'",
                        other.display()
                    ),
                    SoulErrorKind::InvalidTokenKind,
                    Some(token.span),
                ));
            }
        };

        self.expect(&SQUARE_CLOSE)?;
        Ok(size)
    }
}
