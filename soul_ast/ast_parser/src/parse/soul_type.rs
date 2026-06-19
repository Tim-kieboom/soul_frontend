use ast_model::soul_type::{ArrayKind, ArrayType, ReferenceType, SoulType, Stub};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord, types::Types};
use soul_utils::{
    Ident,
    collections::try_result::{
        ResultTryErr, ResultTryNotValue, ToResult, TryErr, TryError, TryNotValue, TryOk, TryResult,
    },
    error::SoulResult,
    fault::Fault,
    literal::{Number, TokenLiteral},
    soul_error_internal,
    soul_names::PrimitiveTypes,
};

use crate::{
    parser::Parser,
    utils::{
        ARRAY, ARROW_LEFT, CURLY_OPEN, DOT, MUT, OPTIONAL, POINTER, REF, ROUND_OPEN, SQUARE_CLOSE,
        SQUARE_OPEN,
    },
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn try_parse_type(&mut self) -> TryResult<SoulType, Fault> {
        let begin = self.tokens.current_position();
        let result = self.inner_parse_type();
        if result.is_err() {
            self.goto(begin);
        }

        result
    }

    pub(crate) fn type_from_ident(&mut self, ident: Ident, generics: Vec<SoulType>) -> SoulType {
        if ident.as_str() == PrimitiveTypes::None.as_str() {
            self.bump();
            return SoulType::None;
        };

        if let Some(prim) = PrimitiveTypes::from_str(ident.as_str()) {
            return SoulType::Primitive(prim);
        }

        SoulType::Stub(Stub {
            name: ident.into_string(),
            generics,
        })
    }

    fn parse_token_type(&mut self, type_val: Types) -> TryResult<SoulType, Fault> {
        self.bump();
        if type_val == Types::RawPtr {
            return self.parse_raw_ptr().try_err();
        }

        let prim = match type_val {
            Types::None => return TryOk(SoulType::None),
            Types::Boolean => PrimitiveTypes::Boolean,
            Types::Int => PrimitiveTypes::Int,
            Types::Int8 => PrimitiveTypes::Int8,
            Types::Int16 => PrimitiveTypes::Int16,
            Types::Int32 => PrimitiveTypes::Int32,
            Types::Int64 => PrimitiveTypes::Int64,
            Types::Uint => PrimitiveTypes::Uint,
            Types::Uint8 => PrimitiveTypes::Uint8,
            Types::Uint16 => PrimitiveTypes::Uint16,
            Types::Uint32 => PrimitiveTypes::Uint32,
            Types::Uint64 => PrimitiveTypes::Uint64,
            Types::Float16 => PrimitiveTypes::Float16,
            Types::Float32 => PrimitiveTypes::Float32,
            Types::Float64 => PrimitiveTypes::Float64,
            Types::Char => PrimitiveTypes::Char,
            Types::Char8 => PrimitiveTypes::Char8,
            Types::Char16 => PrimitiveTypes::Char16,
            Types::Char32 => PrimitiveTypes::Char32,
            Types::Char64 => PrimitiveTypes::Char64,
            Types::CInt => PrimitiveTypes::CInt,
            Types::CUint => PrimitiveTypes::CUint,
            Types::CString => PrimitiveTypes::CStr,
            _ => {
                return TryNotValue(Fault::error(
                    format!("type '{}' not yet supported", type_val.as_str()),
                    Some(self.current().span),
                ));
            }
        };
        TryOk(SoulType::Primitive(prim))
    }

    fn parse_raw_ptr(&mut self) -> SoulResult<SoulType> {
        let inner = if self.current_is(&ARROW_LEFT) {
            let mut generics = self.parse_generic_define().merge_to_result()?;

            if generics.len() != 1 {
                return Err(Fault::error(
                    "RawPtr expects exactly one generic type parameter, e.g. `RawPtr<int>`",
                    Some(self.current().span),
                ));
            }

            let inner = generics.pop().expect("just check if .len() is 1");
            Some(Box::new(inner))
        } else {
            None
        };

        Ok(SoulType::RawPtr(inner))
    }

    fn inner_parse_type(&mut self) -> TryResult<SoulType, Fault> {
        let wrapper = self.get_type_wrapper()?;
        let mut ty = match self.get_base_type() {
            Ok(ty) => ty,
            Err(TryError::IsNotValue(_)) if !wrapper.is_empty() => {
                return TryErr(Fault::error(
                    "expected element type after array size, e.g. `[64]char`",
                    Some(self.current().span),
                ));
            }
            Err(e) => return Err(e),
        };

        const CONST: bool = false;
        const MUT: bool = true;
        for wrap in wrapper {
            ty = match wrap {
                ParseWrappers::ConstRef => SoulType::Reference(ReferenceType::new(ty, CONST)),
                ParseWrappers::MutRef => SoulType::Reference(ReferenceType::new(ty, MUT)),
                ParseWrappers::ConstPointer => SoulType::Pointer(ReferenceType::new(ty, CONST)),
                ParseWrappers::MutPointer => SoulType::Pointer(ReferenceType::new(ty, MUT)),
                ParseWrappers::Option => SoulType::Optional(Box::new(ty)),
                ParseWrappers::Array(kind) => {
                    let array = ArrayType {
                        of_type: Box::new(ty),
                        kind,
                    };
                    SoulType::Array(array)
                }
            };
        }

        if self.current_is(&DOT) {
            let save = self.tokens.current_position();
            self.bump();
            if let Ok(variant) = self.try_bump_consume_ident() {
                return TryOk(SoulType::NamedVariant {
                    base: Box::new(ty),
                    variant,
                });
            }
            self.goto(save);
        }

        Ok(ty)
    }

    fn get_base_type(&mut self) -> TryResult<SoulType, Fault> {
        const NONE_STR: &str = PrimitiveTypes::None.as_str();

        match &self.current().kind {
            TokenKind::Ident(val) if val == NONE_STR => {
                self.bump();
                return TryOk(SoulType::None);
            }
            TokenKind::Types(type_val) => {
                return self.parse_token_type(*type_val);
            }
            &ROUND_OPEN => {
                return TryNotValue(soul_error_internal!(
                    "tuple type not impl",
                    Some(self.current().span)
                ));
            }
            &CURLY_OPEN => {
                return TryNotValue(soul_error_internal!(
                    "nametuple type not impl",
                    Some(self.current().span)
                ));
            }
            _ => (),
        };

        let ident = self.try_bump_consume_ident().try_not_value()?;
        if let Some(keyword) = KeyWord::from_str(ident.as_str()) {
            return TryNotValue(Fault::error(
                format!("keyword '{}' can not be type", keyword.as_str()),
                Some(ident.span()),
            ));
        }

        if let Some(prim) = PrimitiveTypes::from_str(ident.as_str()) {
            return TryOk(SoulType::Primitive(prim));
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_define()?
        } else {
            vec![]
        };

        TryOk(SoulType::Stub(Stub {
            name: ident.into_string(),
            generics,
        }))
    }

    fn get_type_wrapper(&mut self) -> TryResult<Vec<ParseWrappers>, Fault> {
        let mut wrappers = vec![];
        loop {
            let possible_wrap = match self.current().kind {
                REF => {
                    if self.peek_is(&MUT) {
                        self.bump();
                        Some(ParseWrappers::MutRef)
                    } else {
                        Some(ParseWrappers::ConstRef)
                    }
                }
                POINTER => {
                    if self.peek_is(&MUT) {
                        self.bump();
                        Some(ParseWrappers::MutPointer)
                    } else {
                        Some(ParseWrappers::ConstPointer)
                    }
                }
                OPTIONAL => Some(ParseWrappers::Option),
                ARRAY => Some(ParseWrappers::Array(ArrayKind::HeapArray)),
                SQUARE_OPEN => Some(ParseWrappers::Array(self.get_array_type_wrapper()?)),
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

    fn get_array_type_wrapper(&mut self) -> TryResult<ArrayKind, Fault> {
        self.bump();

        let kind = if self.current_is_ident("_") {
            ArrayKind::StackArrayWildcard
        } else {
            match &self.current().kind {
                &REF => {
                    if matches!(self.peek().kind, TokenKind::Keyword(KeyWord::Mut)) {
                        self.bump();
                        ArrayKind::MutSlice
                    } else {
                        ArrayKind::ConstSlice
                    }
                }
                TokenKind::Literal(TokenLiteral::Number(Number::Uint(size))) => {
                    ArrayKind::StackArray(*size)
                }
                other => {
                    return TryNotValue(Fault::error(
                        format!(
                            "token '{}' not allowed in array typeWrapper",
                            other.display()
                        ),
                        Some(self.current().span),
                    ));
                }
            }
        };

        self.bump();
        if self.current().kind != SQUARE_CLOSE {
            return TryNotValue(self.get_expect_error(&SQUARE_CLOSE));
        }

        Ok(kind)
    }
}

enum ParseWrappers {
    ConstRef,
    MutRef,
    ConstPointer,
    MutPointer,
    Option,
    Array(ArrayKind),
}
