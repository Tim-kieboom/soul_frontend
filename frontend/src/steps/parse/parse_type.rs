use crate::steps::{parse::{parse_statement::{COLON, COMMA, CURLY_CLOSE, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN}, parser::{Parser, TryError, TryResult}}, tokenize::token_stream::TokenKind};
use models::{abstract_syntax_tree::soul_type::{ArrayType, NamedTupleType, ReferenceType, SoulType, TupleType, TypeKind}, error::{SoulError, SoulErrorKind, SoulResult}, soul_names::{InternalComplexTypes, InternalPrimitiveTypes, TypeModifier, TypeWrapper}};

impl<'a> Parser<'a> {
    
    pub(crate) fn parse_type(&mut self) -> SoulResult<SoulType> {
        
        let modifier = match &self.token().kind {
            TokenKind::Ident(ident) => {
                let modifier = TypeModifier::from_str(&ident);
                if modifier.is_some() {
                    self.bump();
                }
                modifier 
            },
            _ => None,
        };

        let wrapper = self.get_type_wrappers();

        let mut ty = if self.current_is_ident(InternalPrimitiveTypes::None.as_str()) {
            SoulType::none()
        }
        else if self.current_is(&ROUND_OPEN) {

            self.parse_tuple_type()
                .map(|el| SoulType::new(None, TypeKind::Tuple(el)))?
        }
        else if self.current_is(&CURLY_OPEN) {

            match self.parse_named_tuple_type(&CURLY_OPEN, &CURLY_CLOSE) {
                Ok(val) => SoulType::new(None, TypeKind::NamedTuple(val)),
                Err(TryError::IsErr(err)) |
                Err(TryError::IsNotValue(err)) => return Err(err),
            }
        }
        else {

            let name = match self.bump_consume().kind {
                TokenKind::Ident(val) => val,
                other => return Err(
                    SoulError::new(
                        format!("expected ident got '{}'", other.display()),
                        SoulErrorKind::UnexpecedToken,
                        None,
                    )
                ),
            };

            if let Some(prim) = InternalPrimitiveTypes::from_str(&name) {
                SoulType::new(None, TypeKind::Primitive(prim))
            }
            else if let Some(prim) = InternalComplexTypes::from_str(&name) {
                SoulType::new(None, TypeKind::InternalComplex(prim))
            }
            else {
                SoulType::new(None, TypeKind::Unknown(name))
            }
        };

        for (wrap, lifetime, size) in wrapper {

            ty = match wrap {
                TypeWrapper::ConstRef => SoulType::new(
                    None, 
                    TypeKind::Reference(ReferenceType{
                        inner: Box::new(ty), 
                        lifetime, 
                        mutable: false,
                    }
                )),
                TypeWrapper::MutRef => SoulType::new(
                    None, 
                    TypeKind::Reference(ReferenceType{
                        inner: Box::new(ty), 
                        lifetime, 
                        mutable: true,
                    }
                )),
                TypeWrapper::Pointer => SoulType::new(
                    None, 
                    TypeKind::Pointer(Box::new(ty)),
                ),
                TypeWrapper::Array => SoulType::new(
                    None, 
                    TypeKind::Array(ArrayType{
                        of_type: Box::new(ty),
                        size,
                    }),
                ),
                TypeWrapper::Option => SoulType::new(
                    None, 
                    TypeKind::Optional(Box::new(ty)),
                ),
            };
        }

        ty.modifier = modifier;
        Ok(ty)
    }

    pub(crate) fn parse_tuple_type(&mut self) -> SoulResult<TupleType> {
        self.expect(&ROUND_OPEN)?;

        if self.current_is(&ROUND_CLOSE) {
            self.bump();
            return Ok(
                TupleType{types: vec![]}
            ) 
        }

        let mut types = vec![];

        loop {
            
            let ty = self.parse_type()?;

            if self.current_is(&ROUND_CLOSE) {
                break
            }

            types.push(ty);
            self.expect(&COMMA)?;
        
        }

        self.expect(&ROUND_CLOSE)?;
        
        Ok(
            TupleType{types}
        )
    }

    pub(crate) fn parse_named_tuple_type(&mut self, open: &TokenKind, close: &TokenKind) -> TryResult<NamedTupleType, SoulError> {
        let begin_position = self.current_position();
        let result = self.inner_parse_named_tuple_type(open, close);
        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    fn inner_parse_named_tuple_type(&mut self, open: &TokenKind, close: &TokenKind) -> TryResult<NamedTupleType, SoulError> {
        self.expect(open)
            .map_err(|err| TryError::IsErr(err))?;

        if self.current_is(close) {
            self.bump();
            return Ok(
                NamedTupleType{types: vec![]}
            ) 
        }

        let mut types = vec![];

        loop {
            
            let token = self.bump_consume();
            let name = match token.kind {
                TokenKind::Ident(val) => val,
                other => return Err(TryError::IsNotValue(
                    SoulError::new(
                        format!("'{}' should be ident", other.display()),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    )
                )),
            };

            if !self.current_is(&COLON) {
                return Err(TryError::IsNotValue(self.get_error_expected(&COLON))) // is probebly tuple
            }

            self.bump();
            let ty = self.parse_type()
                .map_err(|err| TryError::IsNotValue(err))?; // is probebly named_tuple expression 
            
            types.push((name, ty));
            if self.current_is(close) {
                break
            }

            self.expect(&COMMA)
                .map_err(|err| TryError::IsErr(err))?;
        
        }

        self.expect(close)
            .map_err(|err| TryError::IsErr(err))?;
        
        Ok(
            NamedTupleType{types}
        )
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