use crate::{
    steps::{
        parse::{ARROW_LEFT, ARROW_RIGHT, ASSIGN, COLON, COMMA, ROUND_OPEN, parser::Parser},
        tokenize::token_stream::TokenKind,
    },
    utils::try_result::{ResultTryErr, ToResult, TryErr, TryError, TryNotValue, TryOk, TryResult},
};
use models::{
    abstract_syntax_tree::{
        expression::Expression,
        function::{Function, FunctionCall, FunctionCallee, FunctionSignature},
        soul_type::{GenericDeclare, GenericDefine, SoulType},
        spanned::Spanned,
        statment::{Ident, Statement, StatementKind},
    }, error::{SoulError, SoulErrorKind, SoulResult, Span}, soul_names::{KeyWord, TypeModifier}, symbool_kind::SymboolKind
};

impl<'a> Parser<'a> {
    pub(crate) fn try_parse_function_call<S: Into<String>>(
        &mut self,
        start_span: Span,
        callee: Option<&Expression>,
        name: S,
    ) -> TryResult<Spanned<FunctionCall>, SoulError> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue(self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT]));
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_type().try_err()?
        } else {
            vec![]
        };

        let arguments = self.parse_tuple().try_err()?;

        Ok(Spanned::new(
            FunctionCall {
                name: name.into(),
                callee: callee.map(|expr| Box::new(expr.clone())),
                generics,
                arguments,
            },
            self.new_span(start_span),
        ))
    }

    pub(crate) fn try_parse_function_signature(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
        extention_type: Option<SoulType>,
        name: Ident,
    ) -> TryResult<Spanned<FunctionSignature>, (Ident, SoulError)> {
        let begin_position = self.current_position();
        let result =
            self.inner_try_parse_function_signature(start_span, modifier, extention_type, name);
        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    pub(crate) fn try_parse_function_declaration(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
        extention_type: Option<SoulType>,
        name: Ident,
    ) -> TryResult<Statement, (Ident, SoulError)> {
        let Spanned {
            node: signature,
            span,
            attributes,
        } = self.try_parse_function_signature(start_span, modifier, extention_type, name)?;
        let block = self.parse_block(modifier).try_err()?;

        self.add_function_delcaration(signature.clone());
        Ok(Statement::with_atribute(
            StatementKind::Function(Function { signature, block }),
            span.combine(self.token().span),
            attributes,
        ))
    }

    pub(crate) fn parse_generic_declare(&mut self) -> SoulResult<Vec<GenericDeclare>> {
        const LIFETIME_SYMBOOL: TokenKind = TokenKind::Unknown('\'');

        self.expect(&ARROW_LEFT)?;

        let mut generics = vec![];
        loop {
            let is_lifetime = self.current_is(&LIFETIME_SYMBOOL);
            if is_lifetime {
                self.bump();
            }

            let ident_token = self.bump_consume();
            let name = match ident_token.kind {
                TokenKind::Ident(ident) => ident,
                other => {
                    return Err(SoulError::new(
                        format!("expected ident got '{}'", other.display()),
                        SoulErrorKind::InvalidAssignType,
                        Some(self.token().span),
                    ));
                }
            };

            let generic = self.inner_parse_generic_declare(name, is_lifetime)?;

            generics.push(generic);
            if !self.current_is(&COMMA) {
                break;
            }

            self.bump();
        }

        self.expect(&ARROW_RIGHT)?;
        Ok(generics)
    }

    pub(crate) fn parse_generic_type(&mut self) -> SoulResult<Vec<GenericDefine>> {
        todo!()
    }

    fn inner_try_parse_function_signature(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
        extention_type: Option<SoulType>,
        name: Ident,
    ) -> TryResult<Spanned<FunctionSignature>, (Ident, SoulError)> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue((name, self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT])));
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_declare().try_err()?
        } else {
            vec![]
        };

        if !self.current_is(&ROUND_OPEN) {
            return TryErr(self.get_expect_error(&ROUND_OPEN));
        }

        const IS_FUNCTION: bool = true;
        let (parameters, this) = match self.parse_named_tuple_type(IS_FUNCTION) {
            Ok(val) => val,
            Err(TryError::IsNotValue(err)) => {
                return TryNotValue((name, err));
            }
            Err(TryError::IsErr(err)) => return TryErr(err),
        };

        let return_type = if self.current_is(&COLON) {
            self.bump();
            match self.try_parse_type() {
                Ok(val) => val,
                Err(TryError::IsErr(err)) => return TryErr(err),
                Err(TryError::IsNotValue(err)) => return TryNotValue((name, err)),
            }
        } else {
            SoulType::none()
        };

        let callee = extention_type.map(|ty| {
            Spanned::new(
                FunctionCallee {
                    extention_type: ty,
                    this,
                },
                self.token().span,
            )
        });

        let signature = FunctionSignature {
            name,
            callee,
            generics,
            modifier,
            parameters,
            return_type,
            contructor: None,
        };

        TryOk(Spanned::new(signature, self.new_span(start_span)))
    }

    fn inner_parse_generic_declare(
        &mut self,
        name: Ident,
        is_lifetime: bool,
    ) -> SoulResult<GenericDeclare> {
        const IMPL_STR: &str = KeyWord::Impl.as_str();
        const COLON_STR: &str = SymboolKind::Colon.as_str();

        if is_lifetime {
            return Ok(GenericDeclare::Lifetime(name));
        }

        if self.current_is(&COLON) {
            let mut traits = vec![];
            loop {
                self.bump();
                let ty = self.try_parse_type().merge_to_result()?;

                traits.push(ty);

                if self.current_is_any(&[ARROW_RIGHT, ASSIGN]) {
                    break;
                }

                if self.current_is_ident(IMPL_STR) {
                    return Err(SoulError::new(
                        format!("can not have '{IMPL_STR}' and '{COLON_STR}' at the same time"),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    ));
                }

                if !self.current_is(&COMMA) {
                    return Err(self.get_expect_error(&COMMA));
                }
            }

            let default = if self.current_is(&ASSIGN) {
                self.bump();
                Some(self.try_parse_type().merge_to_result()?)
            } else {
                None
            };

            return Ok(GenericDeclare::Type {
                name,
                traits,
                default,
            });
        }

        if self.current_is_ident(KeyWord::Impl.as_str()) {
            self.bump();
            let ty = self.try_parse_type().merge_to_result()?;

            if self.current_is(&COLON) {
                return Err(SoulError::new(
                    format!("can not have '{IMPL_STR}' and '{COLON_STR}' at the same time"),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.token().span),
                ));
            }

            let default = if self.current_is(&ASSIGN) {
                self.bump();
                Some(self.parse_expression(&[COMMA, ARROW_RIGHT])?)
            } else {
                None
            };

            return Ok(GenericDeclare::Expression {
                name,
                for_type: Some(ty),
                default,
            });
        }

        Err(SoulError::new(
            format!("'{}' not valid for generic", self.token().kind.display()),
            SoulErrorKind::InvalidContext,
            Some(self.token().span),
        ))
    }
}
