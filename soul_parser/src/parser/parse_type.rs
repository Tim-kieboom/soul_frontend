use parser_models::ast::{
    ArrayKind, ArrayType, FunctionKind, NamedTupleType, ReferenceType, SoulType, TypeKind,
};
use soul_tokenizer::{Number, TokenKind};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_error_internal,
    soul_names::{InternalPrimitiveTypes, TypeModifier},
    try_result::{
        ResultTryErr, ResultTryNotValue, TryErr, TryError, TryNotValue, TryOk, TryResult,
    },
};

use crate::parser::{
    Parser,
    parse_utils::{
        ARRAY, COLON, COMMA, CONST_REF, CURLY_OPEN, MUT_REF, OPTIONAL, POINTER, ROUND_CLOSE,
        ROUND_OPEN, SQUARE_CLOSE, SQUARE_OPEN,
    },
};

impl<'a> Parser<'a> {
    pub(crate) fn try_parse_type(&mut self) -> TryResult<SoulType, SoulError> {
        let begin = self.current_position();
        let result = self.inner_parse_type();
        if result.is_err() {
            self.go_to(begin);
        }

        result
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

    fn inner_parse_type(&mut self) -> TryResult<SoulType, SoulError> {
        let wrapper = self.get_type_wrapper()?;
        let mut ty = self.get_base_type()?;

        const CONST: bool = false;
        const MUT: bool = true;
        for wrap in wrapper {
            let span = self.token().span.combine(ty.span);

            ty = match wrap {
                ParseWrappers::ConstRef => SoulType::new(
                    None,
                    TypeKind::Reference(ReferenceType::new(ty, CONST)),
                    span,
                ),
                ParseWrappers::MutRef => {
                    SoulType::new(None, TypeKind::Reference(ReferenceType::new(ty, MUT)), span)
                }
                ParseWrappers::Pointer => {
                    SoulType::new(None, TypeKind::Pointer(Box::new(ty)), span)
                }
                ParseWrappers::Option => {
                    SoulType::new(None, TypeKind::Optional(Box::new(ty)), span)
                }
                ParseWrappers::Array(kind) => {
                    let array = ArrayType {
                        of_type: Box::new(ty),
                        kind,
                    };
                    SoulType::new(None, TypeKind::Array(array), span)
                }
            };
        }
        Ok(ty)
    }

    fn get_type_wrapper(&mut self) -> TryResult<Vec<ParseWrappers>, SoulError> {
        let mut wrappers = vec![];
        loop {
            let possible_wrap = match &self.token().kind {
                &CONST_REF => Some(ParseWrappers::ConstRef),
                &MUT_REF => Some(ParseWrappers::MutRef),
                &POINTER => Some(ParseWrappers::Pointer),
                &OPTIONAL => Some(ParseWrappers::Option),
                &SQUARE_OPEN => Some(ParseWrappers::Array(self.get_array_type_wrapper()?)),
                &ARRAY => {
                    return TryNotValue(SoulError::new(
                        "empty array kind (so `[]`) is not allowed use `[<number>]` or `[*]` or `[@] or `[&]`",
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    ));
                }
                _ => None,
            };

            let wrap = match possible_wrap {
                Some(val) => val,
                None => break,
            };

            self.bump();
            wrappers.push(wrap);
        }

        wrappers.reverse();
        TryOk(wrappers)
    }

    fn get_array_type_wrapper(&mut self) -> TryResult<ArrayKind, SoulError> {
        self.bump();
        let kind = match &self.token().kind {
            &CONST_REF => ArrayKind::ConstSlice,
            &MUT_REF => ArrayKind::MutSlice,
            &POINTER => ArrayKind::HeapArray,
            TokenKind::Number(Number::Uint(size)) => ArrayKind::StackArray(*size),
            other => {
                return TryNotValue(SoulError::new(
                    format!(
                        "token '{}' not allowed in array typeWrapper",
                        other.display()
                    ),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.token().span),
                ));
            }
        };

        self.bump();
        if self.token().kind != SQUARE_CLOSE {
            return TryNotValue(self.get_expect_error(&SQUARE_CLOSE));
        }

        Ok(kind)
    }

    fn get_base_type(&mut self) -> TryResult<SoulType, SoulError> {
        const NONE_STR: &str = InternalPrimitiveTypes::None.as_str();

        match &self.token().kind {
            TokenKind::Ident(val) if val == NONE_STR => {
                return TryOk(SoulType::none(self.token().span));
            }
            &ROUND_OPEN => {
                return TryNotValue(soul_error_internal!(
                    "tuple type not impl",
                    Some(self.token().span)
                ));
            }
            &CURLY_OPEN => {
                return TryNotValue(soul_error_internal!(
                    "nametuple type not impl",
                    Some(self.token().span)
                ));
            }
            _ => (),
        };

        let ident = self.try_bump_consume_ident().try_not_value()?;

        if let Some(prim) = InternalPrimitiveTypes::from_str(ident.as_str()) {
            let span = self.token().span;
            return TryOk(SoulType::new(None, TypeKind::Primitive(prim), span));
        }

        TryNotValue(soul_error_internal!(
            "Stub type not impl",
            Some(self.token().span)
        ))
    }

    fn inner_parse_named_tuple_kinds(
        &mut self,
        kind: NamedTupleKinds,
    ) -> TryResult<(NamedTupleType, FunctionKind), SoulError> {
        let (open, close, can_have_this) = match kind {
            NamedTupleKinds::Function => (&ROUND_OPEN, &ROUND_CLOSE, true),
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
                    Some(modifier)
                }
                _ => match kind {
                    NamedTupleKinds::Function => Some(TypeModifier::Const),
                },
            };

            let name = self.try_bump_consume_ident().try_not_value()?;

            if !self.current_is(&COLON) {
                // is probebly tuple
                return Err(TryError::IsNotValue(self.get_expect_error(&COLON)));
            }
            self.bump();

            let mut ty = self.try_parse_type()?; // if not value is probebly named_tuple expression 
            ty.modifier = modifier;

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

enum ParseWrappers {
    ConstRef,
    MutRef,
    Pointer,
    Option,
    Array(ArrayKind),
}

enum Loop {
    None,
    Break,
    Continue,
}

enum NamedTupleKinds {
    Function,
}
