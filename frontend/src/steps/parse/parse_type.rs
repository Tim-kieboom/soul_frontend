use crate::{
    steps::{
        parse::{
            parse_statement::{COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN},
            parser::Parser,
        },
        tokenize::token_stream::TokenKind,
    },
    utils::try_result::{ResultTryResult, TryErr, TryError, TryNotValue, TryResult},
};
use models::{
    abstract_syntax_tree::{
        function::ThisCallee,
        soul_type::{ArrayType, NamedTupleType, ReferenceType, SoulType, TupleType, TypeKind},
    },
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{InternalComplexTypes, InternalPrimitiveTypes, TypeModifier, TypeWrapper},
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
                let modifier = TypeModifier::from_str(&ident);
                if modifier.is_some() {
                    self.bump();
                }
                modifier
            }
            _ => None,
        };

        let wrapper = self.get_type_wrappers();

        let mut ty = if self.current_is_ident(InternalPrimitiveTypes::None.as_str()) {
            SoulType::none()
        } else if self.current_is(&ROUND_OPEN) {
            self.parse_tuple_type()
                .map(|el| SoulType::new(None, TypeKind::Tuple(el)))
                .try_err()?
        } else if self.current_is(&CURLY_OPEN) {
            const IS_NAMED_TUPLE: bool = false;
            self.parse_named_tuple_type(IS_NAMED_TUPLE)
                .map(|(tuple, _this)| SoulType::new(None, TypeKind::NamedTuple(tuple)))?
        } else {
            let name = match self.bump_consume().kind {
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
                SoulType::new(None, TypeKind::Primitive(prim))
            } else if let Some(prim) = InternalComplexTypes::from_str(&name) {
                SoulType::new(None, TypeKind::InternalComplex(prim))
            } else {
                SoulType::new(None, TypeKind::Stub(name))
            }
        };

        for (wrap, lifetime, size) in wrapper {
            ty = match wrap {
                TypeWrapper::ConstRef => SoulType::new(
                    None,
                    TypeKind::Reference(ReferenceType {
                        inner: Box::new(ty),
                        lifetime,
                        mutable: false,
                    }),
                ),
                TypeWrapper::MutRef => SoulType::new(
                    None,
                    TypeKind::Reference(ReferenceType {
                        inner: Box::new(ty),
                        lifetime,
                        mutable: true,
                    }),
                ),
                TypeWrapper::Pointer => SoulType::new(None, TypeKind::Pointer(Box::new(ty))),
                TypeWrapper::Array => SoulType::new(
                    None,
                    TypeKind::Array(ArrayType {
                        of_type: Box::new(ty),
                        size,
                    }),
                ),
                TypeWrapper::Option => SoulType::new(None, TypeKind::Optional(Box::new(ty))),
            };
        }

        ty.modifier = modifier;
        Ok(ty)
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

        self.expect(open).map_err(|err| TryError::IsErr(err))?;

        if self.current_is(close) {
            self.bump();
            return Ok((NamedTupleType { types: vec![] }, ThisCallee::Static));
        }

        let mut this = ThisCallee::Static;
        let mut types = vec![];
        loop {
            let possible_this = if self.current_is_ident(TypeWrapper::ConstRef.as_str()) {
                Some(ThisCallee::ConstRef)
            } else if self.current_is_ident(TypeWrapper::MutRef.as_str()) {
                Some(ThisCallee::MutRef)
            } else if self.current_is_ident("this") {
                Some(ThisCallee::Consume)
            } else {
                None
            };

            if let Some(callee) = possible_this {
                if this != ThisCallee::Static {
                    return TryErr(SoulError::new(
                        format!("can not have more then one 'this' in methode"),
                        SoulErrorKind::InvalidContext,
                        Some(self.token().span),
                    ));
                }

                this = callee;
                self.expect_ident("this").try_err()?;
                self.expect(&COMMA).try_err()?;
                continue;
            }

            let token = self.bump_consume();
            let name = match token.kind {
                TokenKind::Ident(val) => val,
                other => {
                    return Err(TryError::IsNotValue(SoulError::new(
                        format!("'{}' should be ident", other.display()),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    )));
                }
            };

            if !self.current_is(&COLON) {
                return Err(TryError::IsNotValue(self.get_error_expected(&COLON))); // is probebly tuple
            }

            self.bump();
            let ty = self.try_parse_type()?; // is probebly named_tuple expression 

            types.push((name, ty));
            if self.current_is(close) {
                break;
            }

            self.expect(&COMMA).try_err()?;
        }

        self.expect(close).map_err(|err| TryError::IsErr(err))?;

        Ok((NamedTupleType { types }, this))
    }

    fn get_type_wrappers(&mut self) -> Vec<(TypeWrapper, String, Option<usize>)> {
        let mut wrapper = vec![];
        loop {
            let possible_wrap = match &self.token().kind {
                TokenKind::Symbool(sym) => TypeWrapper::from_symbool(*sym),
                _ => None,
            };

            let wrap = match possible_wrap {
                Some(val) => val,
                None => break wrapper,
            };

            wrapper.push((wrap, String::new(), None));
            self.bump();
        }
    }
}
