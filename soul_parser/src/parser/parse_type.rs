use parser_models::ast::{
    ArrayType, FunctionKind, GenericDeclare, GenericDefine, NamedTupleType, ReferenceType,
    SoulType, TupleType, TypeKind,
};
use soul_tokenizer::{Number, TokenKind};
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{InternalPrimitiveTypes, TypeModifier, TypeWrapper},
    try_result::{
        ResultTryErr, ResultTryNotValue, ToResult, TryErr, TryError, TryNotValue, TryOk, TryResult,
    },
};

use crate::parser::{
    Parser,
    parse_utils::{
        ARRAY, COLON, COMMA, CONST_REF, CURLY_CLOSE, CURLY_OPEN, MUT_REF, OPTIONAL, POINTER, ROUND_CLOSE, ROUND_OPEN, SQUARE_OPEN
    },
};

impl<'a> Parser<'a> {
    pub(crate) fn try_parse_type(&mut self, default_modifier: TypeModifier) -> TryResult<SoulType, SoulError> {
        let begin = self.current_position();
        let result = self.inner_parse_type(default_modifier);
        if result.is_err() {
            self.go_to(begin);
        }

        result
    }

    pub(crate) fn parse_generic_declare(&mut self) -> SoulResult<Vec<GenericDeclare>> {
        todo!()
    }

    pub(crate) fn parse_generic_define(&mut self) -> SoulResult<Vec<GenericDefine>> {
        todo!()
    }

    pub(crate) fn try_parse_parameters(
        &mut self,
    ) -> TryResult<(NamedTupleType, FunctionKind), SoulError> {
        let begin = self.current_position();

        let result = self.inner_parse_named_tuple_kinds(NamedTupleKinds::Function);
        if result.is_err() {
            self.go_to(begin);
        }

        result
    }

    pub(crate) fn try_parse_named_tuple_type(&mut self) -> TryResult<NamedTupleType, SoulError> {
        let begin = self.current_position();

        let result = self.inner_parse_named_tuple_kinds(NamedTupleKinds::NamedTuple);
        if result.is_err() {
            self.go_to(begin);
        }

        result.map(|(types, _)| types)
    }

    pub(crate) fn parse_tuple_type(&mut self) -> SoulResult<TupleType> {
        self.expect(&ROUND_OPEN)?;

        let mut types = TupleType::new();
        if self.current_is(&ROUND_CLOSE) {
            self.bump();
            return Ok(types);
        }

        const DEFAULT_MODIFIER: TypeModifier = TypeModifier::Mut;
        loop {
            let ty = self.try_parse_type(DEFAULT_MODIFIER).merge_to_result()?;
            types.push(ty);

            if self.current_is(&ROUND_CLOSE) {
                break;
            }

            self.expect(&COMMA)?;
        }

        self.expect(&ROUND_CLOSE)?;
        Ok(types)
    }

    fn inner_parse_type(&mut self, modifier: TypeModifier) -> TryResult<SoulType, SoulError> {
        let wrapper = self.get_type_wrapper()?;
        let mut ty = self.get_base_type()?;

        const INNER_MODIFIER: TypeModifier = TypeModifier::Mut;

        const CONST: bool = false;
        const MUT: bool = true;
        for (wrap, size) in wrapper {
            let span = self.token().span.combine(ty.span);

            ty = match wrap {
                TypeWrapper::ConstRef => SoulType::new(
                    INNER_MODIFIER,
                    TypeKind::Reference(ReferenceType::new(ty, CONST)),
                    span,
                ),
                TypeWrapper::MutRef => SoulType::new(
                    INNER_MODIFIER,
                    TypeKind::Reference(ReferenceType::new(ty, MUT)),
                    span,
                ),
                TypeWrapper::Pointer => {
                    SoulType::new(INNER_MODIFIER, TypeKind::Pointer(Box::new(ty)), span)
                }
                TypeWrapper::Array => {
                    SoulType::new(INNER_MODIFIER, TypeKind::Array(ArrayType::new(ty, size)), span)
                }
                TypeWrapper::Option => {
                    SoulType::new(INNER_MODIFIER, TypeKind::Optional(Box::new(ty)), span)
                }
            };
        }

        ty.modifier = modifier;
        Ok(ty)
    }

    fn get_type_wrapper(&mut self) -> TryResult<Vec<(TypeWrapper, Option<u64>)>, SoulError> {
        let mut wrappers = vec![];
        loop {

            let mut size = None;
            let possible_wrap = match &self.token().kind {
                &SQUARE_OPEN => {
                    self.bump();
                    match &self.token().kind {
                        TokenKind::Number(Number::Uint(num)) => size = Some(*num),
                        _ => {
                            return TryNotValue(SoulError::new(
                                format!(
                                    "expected literal number got '{}'",
                                    self.token().kind.display()
                                ),
                                SoulErrorKind::InvalidNumber,
                                Some(self.token().span),
                            ));
                        }
                    }
                    Some(TypeWrapper::Array)
                }
                &ARRAY => Some(TypeWrapper::Array),
                &CONST_REF => Some(TypeWrapper::ConstRef),
                &MUT_REF => Some(TypeWrapper::MutRef),
                &POINTER => Some(TypeWrapper::Pointer),
                &OPTIONAL => Some(TypeWrapper::Option),
                _ => None,
            };

            let wrap = match possible_wrap {
                Some(val) => val,
                None => return TryOk(wrappers),
            };

            if size.is_none() {
                self.bump();
            }
            wrappers.push((wrap, size));
        }
    }

    fn get_base_type(&mut self) -> TryResult<SoulType, SoulError> {
        const NONE_STR: &str = InternalPrimitiveTypes::None.as_str();
        const MODIFIER: TypeModifier = TypeModifier::Mut;

        match &self.token().kind {
            TokenKind::Ident(val) if val == NONE_STR => {
                return TryOk(SoulType::none(self.token().span));
            }
            &ROUND_OPEN => {
                let span = self.token().span;
                return self
                    .parse_tuple_type()
                    .map(|types| SoulType::new(MODIFIER, TypeKind::Tuple(types), span))
                    .try_err();
            }
            &CURLY_OPEN => {
                let span = self.token().span;
                return self
                    .try_parse_named_tuple_type()
                    .map(|types| SoulType::new(MODIFIER, TypeKind::NamedTuple(types), span));
            }
            _ => (),
        };

        let ident = self.try_bump_consume_ident().try_not_value()?;

        if let Some(prim) = InternalPrimitiveTypes::from_str(ident.as_str()) {
            let span = self.token().span;
            return TryOk(SoulType::new(MODIFIER, TypeKind::Primitive(prim), span));
        }

        TryOk(SoulType::new(
            MODIFIER,
            TypeKind::Stub {
                ident,
                resolved: None,
            },
            self.token().span,
        ))
    }

    fn inner_parse_named_tuple_kinds(
        &mut self,
        kind: NamedTupleKinds,
    ) -> TryResult<(NamedTupleType, FunctionKind), SoulError> {
        let (open, close, can_have_this) = match kind {
            NamedTupleKinds::Function => (&ROUND_OPEN, &ROUND_CLOSE, true),
            NamedTupleKinds::NamedTuple => (&CURLY_OPEN, &CURLY_CLOSE, false),
        };

        self.expect(open).try_err()?;

        let mut types = NamedTupleType::new();
        let mut function_kind = FunctionKind::Static;
        if self.current_is(close) {
            self.bump();
            return TryOk((types, function_kind));
        }

        loop {
            match self.inner_parse_named_this(&mut function_kind, can_have_this)? {
                Loop::None => (),
                Loop::Break => break,
                Loop::Continue => continue,
            }

            
            let modifier = match self.try_token_as_ident_str().map(TypeModifier::from_str) {
                Ok(Some(modifier)) => {
                    self.bump();
                    modifier
                }
                _ => TypeModifier::Const,
            };

            let name = self.try_bump_consume_ident().try_not_value()?;

            if !self.current_is(&COLON) {
                // is probebly tuple
                return Err(TryError::IsNotValue(self.get_expect_error(&COLON)));
            }
            self.bump();

            let ty = self.try_parse_type(modifier)?; // if not value is probebly named_tuple expression 

            types.push((name, ty, None));
            if self.current_is(close) {
                break;
            }

            self.expect(&COMMA).try_err()?;
        }

        self.expect(close).try_err()?;

        Ok((types, function_kind))
    }

    fn inner_parse_named_this(
        &mut self,
        kind: &mut FunctionKind,
        should_have_this: bool,
    ) -> TryResult<Loop, SoulError> {
        let this = match &self.token().kind {
            &CONST_REF => {
                self.bump();
                Some(FunctionKind::ConstRef)
            }
            &MUT_REF => {
                self.bump();
                Some(FunctionKind::MutRef)
            }
            TokenKind::Ident(val) if val == "this" => Some(FunctionKind::Consume),
            _ => None,
        };

        if let Some(callee) = this {
            if !should_have_this {
                return TryErr(SoulError::new(
                    "can not have 'this' in namedTuple",
                    SoulErrorKind::InvalidContext,
                    Some(self.token().span),
                ));
            }

            if *kind != FunctionKind::Static {
                return TryErr(SoulError::new(
                    "can not have more then one 'this' in methode",
                    SoulErrorKind::InvalidContext,
                    Some(self.token().span),
                ));
            }

            *kind = callee;
            self.expect_ident("this").try_err()?;

            return match self.token().kind {
                ROUND_CLOSE => TryOk(Loop::Break),
                COMMA => {
                    self.bump();
                    TryOk(Loop::Continue)
                }
                _ => TryErr(self.get_expect_any_error(&[COMMA, ROUND_CLOSE])),
            };
        }

        Ok(Loop::None)
    }
}

enum Loop {
    None,
    Break,
    Continue,
}

enum NamedTupleKinds {
    Function,
    NamedTuple,
}
